use anyhow::Result;
use tokio::runtime::Runtime;

use crate::lib::{
    analysis::{analyze_stock, StockAnalysis},
    config::StockConfig,
    persistence::PersistenceManager,
    stock_data::StockData,
    yahooapi::fetch_stock_data,
};
use crate::data::TimeRange;
use crate::event::AppEvent;

// ── public types ───────────────────────────────────────────────

pub enum View {
    Main,
    Detail,
    Edit,
}

pub struct AnalysisWithChartData {
    pub analysis: StockAnalysis,
    pub stock_data: StockData,
    pub time_range: TimeRange,
}

// ── App state ──────────────────────────────────────────────────

pub struct App {
    pub analyses: Vec<AnalysisWithChartData>,
    pub selected_index: usize,
    pub selected_time_range_index: usize,
    pub(super) rt: Runtime,
    pub current_view: View,
    pub config_file_path: String,
    pub editing_symbols: Vec<String>,
    pub editing_selected_index: usize,
    pub new_symbol_input: String,
    pub(super) should_refresh_after_save: bool,
    pub(super) channel_rx: Option<std::sync::mpsc::Receiver<AppEvent>>,
    pub(super) persistence_manager: PersistenceManager,
    pub crosshair_index: Option<usize>,
    /// How many stocks are being fetched in the current batch.
    pub loading_total: usize,
    /// How many have completed (success or error) so far.
    pub loading_done: usize,
    /// Error messages collected during the current load batch.
    pub loading_errors: Vec<String>,
}

impl App {
    pub fn new() -> Result<Self> {
        let persistence_manager = PersistenceManager::new()?;
        Ok(Self {
            analyses: Vec::new(),
            selected_index: 0,
            selected_time_range_index: 0,
            rt: Runtime::new()?,
            current_view: View::Main,
            config_file_path: String::from("persistent_config"),
            editing_symbols: Vec::new(),
            editing_selected_index: 0,
            new_symbol_input: String::new(),
            should_refresh_after_save: false,
            channel_rx: None,
            persistence_manager,
            crosshair_index: None,
            loading_total: 0,
            loading_done: 0,
            loading_errors: Vec::new(),
        })
    }

    /// Set flag to refresh analyses after saving config.
    pub fn refresh_analyses(&mut self, _config: &StockConfig) {
        self.should_refresh_after_save = true;
    }

    /// Check and process any pending refresh.
    pub(super) fn check_refresh(&mut self) {
        if self.should_refresh_after_save {
            self.should_refresh_after_save = false;
            if let Ok(config) = self.persistence_manager.get_stock_config() {
                self.initialize_placeholders(&config);
            }
        }
    }

    /// Drain async events from the channel into analyses.
    pub(super) fn drain_events(&mut self) {
        // Drain all available events (not just one per frame)
        loop {
            let event = if let Some(ref rx) = self.channel_rx {
                match rx.try_recv() {
                    Ok(e) => e,
                    Err(_) => break, // channel empty or disconnected
                }
            } else {
                break;
            };

            self.loading_done += 1;

            match event {
                AppEvent::Update(analysis, stock_data, time_range) => {
                    // Replace existing entry for this symbol (re-fetch), or push new
                    if let Some(existing) = self.analyses.iter_mut()
                        .find(|a| a.analysis.symbol == analysis.symbol)
                    {
                        existing.analysis = analysis;
                        existing.stock_data = stock_data;
                        existing.time_range = time_range;
                    } else {
                        self.analyses.push(AnalysisWithChartData {
                            analysis,
                            stock_data,
                            time_range,
                        });
                    }
                }
                AppEvent::Error(err) => {
                    self.loading_errors.push(err);
                }
            }
        }
    }

    /// Create empty placeholder entries for each configured symbol.
    /// Data is fetched lazily — when the user enters detail view.
    pub(super) fn initialize_placeholders(&mut self, config: &StockConfig) {
        self.analyses.clear();
        self.loading_total = 0;
        self.loading_done = 0;
        self.loading_errors.clear();

        let default_time_range = TimeRange::ThreeMonths;
        for symbol in &config.symbols {
            self.analyses.push(AnalysisWithChartData {
                analysis: StockAnalysis {
                    symbol: symbol.clone(),
                    current_price: 0.0,
                    sma_10: None,
                    sma_50: None,
                    ema_20: None,
                    sma10_values: vec![],
                    sma50_values: vec![],
                    ema20_values: vec![],
                    predictions: vec![],
                    recent_change: None,
                },
                stock_data: StockData::new(),
                time_range: default_time_range,
            });
        }
    }

    /// Fetch data for a single stock (called on Enter or time-range change).
    /// Clears the existing data immediately so old data doesn't show while loading.
    pub(super) fn fetch_single_stock(&mut self, index: usize, time_range: TimeRange) {
        if index >= self.analyses.len() {
            return;
        }
        // Clear old data immediately — chart shows empty until new data arrives
        self.analyses[index].stock_data = StockData::new();
        self.analyses[index].analysis = StockAnalysis {
            symbol: self.analyses[index].analysis.symbol.clone(),
            current_price: 0.0,
            sma_10: None, sma_50: None, ema_20: None,
            sma10_values: vec![], sma50_values: vec![], ema20_values: vec![],
            predictions: vec![], recent_change: None,
        };

        let symbol = self.analyses[index].analysis.symbol.clone();
        let (tx, rx) = std::sync::mpsc::channel();
        self.channel_rx = Some(rx);
        self.loading_total = 1;
        self.loading_done = 0;
        self.loading_errors.clear();
        self.rt.spawn(async move {
            match fetch_stock_data(&symbol, time_range).await {
                Ok(stock_data) => {
                    if !stock_data.is_empty() {
                        let analysis = analyze_stock(&stock_data, &symbol);
                        let _ = tx.send(AppEvent::Update(analysis, stock_data, time_range));
                    } else {
                        let _ = tx.send(AppEvent::Error(format!("No data for {symbol}")));
                    }
                }
                Err(e) => {
                    let _ = tx.send(AppEvent::Error(format!("{symbol}: {e}")));
                }
            }
        });
    }

    // ── shared helpers ─────────────────────────────────────────

    /// Cycle the time range and re-fetch with the new range/interval.
    pub(super) fn cycle_time_range(&mut self, direction: i8) {
        if self.analyses.is_empty() || self.selected_index >= self.analyses.len() {
            return;
        }
        let ranges = TimeRange::all();
        let len = ranges.len();
        if direction > 0 {
            if self.selected_time_range_index < len - 1 {
                self.selected_time_range_index += 1;
            } else {
                self.selected_time_range_index = 0;
            }
        } else {
            if self.selected_time_range_index > 0 {
                self.selected_time_range_index -= 1;
            } else {
                self.selected_time_range_index = len - 1;
            }
        }
        let new_range = ranges[self.selected_time_range_index];
        self.analyses[self.selected_index].time_range = new_range;
        self.fetch_single_stock(self.selected_index, new_range);
    }

    /// Get the number of visible bars for the currently selected stock.
    pub(super) fn visible_bar_count(&self) -> usize {
        if let Some(data) = self.analyses.get(self.selected_index) {
            crate::data::filter_bars(&data.stock_data, data.time_range).len()
        } else {
            0
        }
    }
}
