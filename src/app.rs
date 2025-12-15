use anyhow::Result;
use crossterm::event::{self, Event, KeyCode};
use ratatui::prelude::*;

use stock_predictor_lib::{
    analysis::StockAnalysis,
    config::{StockConfig},
    stock_data::StockData,
};
use std::io;
use std::time::Duration;
use tokio::runtime::Runtime;

use crate::{
    data::TimeRange,
    event::AppEvent,
    ui::{detail::draw_detail_ui, layout::draw_ui},
};

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

pub struct App {
    pub analyses: Vec<AnalysisWithChartData>,
    pub selected_index: usize,
    pub selected_time_range_index: usize,
    rt: Runtime,
    pub current_view: View,
    pub config_file_path: String,  // Path to the config file
    pub editing_symbols: Vec<String>, // Symbols being edited
    pub editing_selected_index: usize, // Selected index in the editing list
    pub new_symbol_input: String, // Currently typed new symbol
    should_refresh_after_save: bool, // Flag to indicate we need to refresh after saving
    channel_rx: Option<std::sync::mpsc::Receiver<AppEvent>>, // Channel receiver for app events
}

impl App {
    pub fn new() -> Result<Self> {
        Ok(Self {
            analyses: Vec::new(),
            selected_index: 0,
            selected_time_range_index: 0,
            rt: Runtime::new()?,
            current_view: View::Main,
            config_file_path: String::from("stocks_config.json"), // Default path
            editing_symbols: Vec::new(),
            editing_selected_index: 0,
            new_symbol_input: String::new(),
            should_refresh_after_save: false,
            channel_rx: None,
        })
    }

    pub fn run(
        &mut self,
        terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
        config: &StockConfig,
        config_file_path: &str,
    ) -> Result<()> {
        // Update the config file path
        self.config_file_path = config_file_path.to_string();

        // Initialize with the provided config
        self.initialize_data_fetching(config)?;

        loop {
            // Check if we need to refresh the data
            if self.should_refresh_after_save {
                self.should_refresh_after_save = false;
                if let Ok(config) = stock_predictor_lib::config::read_config(&self.config_file_path) {
                    let _ = self.initialize_data_fetching(&config);
                }
            }

            // Process events from the stored receiver
            if let Some(ref rx) = self.channel_rx {
                if let Ok(app_event) = rx.try_recv() {
                    match app_event {
                        AppEvent::Update(analysis, stock_data, time_range) => {
                            self.analyses.push(AnalysisWithChartData {
                                analysis,
                                stock_data,
                                time_range,
                            });
                        }
                        AppEvent::Error(_err) => {
                            // In this case, we'll just ignore them for now
                        }
                    }
                }
            }

            match self.current_view {
                View::Main => {
                    terminal.draw(|f| draw_ui(f, &self.analyses, self.selected_index))?;
                }
                View::Detail => {
                    terminal.draw(|f| {
                        if let Some(selected_data) = self.analyses.get(self.selected_index) {
                            draw_detail_ui(f, selected_data, f.size());
                        }
                    })?;
                }
                View::Edit => {
                    terminal.draw(|f| {
                        super::ui::edit::draw_edit_ui(f, self, f.size());
                    })?;
                }
            }

            // Handle key events differently based on current view
            if event::poll(Duration::from_millis(100))? {
                if let Event::Key(key) = event::read()? {
                    match self.current_view {
                        View::Main | View::Detail => {
                            match key.code {
                                KeyCode::Char('q') => return Ok(()),
                                KeyCode::Char('c') if key.modifiers == event::KeyModifiers::CONTROL => {
                                    return Ok(());
                                }
                                KeyCode::Left => {
                                    if self.selected_index > 0 {
                                        self.selected_index -= 1;
                                    }
                                }
                                KeyCode::Right => {
                                    if !self.analyses.is_empty()
                                        && self.selected_index < self.analyses.len() - 1
                                    {
                                        self.selected_index += 1;
                                    }
                                }
                                KeyCode::Up => {
                                    if !self.analyses.is_empty()
                                        && self.selected_index < self.analyses.len()
                                    {
                                        // Change time range to previous option
                                        if self.selected_time_range_index > 0 {
                                            self.selected_time_range_index -= 1;
                                        } else {
                                            self.selected_time_range_index = TimeRange::all().len() - 1; // Wrap to last
                                        }
                                        // Update the time range for the currently selected stock
                                        if !self.analyses.is_empty() {
                                            self.analyses[self.selected_index].time_range =
                                                TimeRange::all()[self.selected_time_range_index];
                                        }
                                    }
                                }
                                KeyCode::Down => {
                                    if !self.analyses.is_empty()
                                        && self.selected_index < self.analyses.len()
                                    {
                                        // Change time range to next option
                                        if self.selected_time_range_index < TimeRange::all().len() - 1 {
                                            self.selected_time_range_index += 1;
                                        } else {
                                            self.selected_time_range_index = 0; // Wrap to first
                                        }
                                        // Update the time range for the currently selected stock
                                        if !self.analyses.is_empty() {
                                            self.analyses[self.selected_index].time_range =
                                                TimeRange::all()[self.selected_time_range_index];
                                        }
                                    }
                                }
                                KeyCode::Enter => {
                                    self.current_view = View::Detail;
                                }
                                KeyCode::Esc => {
                                    match self.current_view {
                                        View::Edit => self.current_view = View::Main, // Exit edit mode
                                        View::Detail => self.current_view = View::Main, // Exit detail mode
                                        View::Main => return Ok(()), // Exit app
                                    }
                                }
                                KeyCode::Char('e') => {
                                    // Enter edit mode
                                    self.current_view = View::Edit;
                                    // Initialize editing symbols with current config
                                    self.editing_symbols = self.analyses
                                        .iter()
                                        .map(|a| a.analysis.symbol.clone())
                                        .collect();
                                    self.editing_selected_index = 0;
                                    self.new_symbol_input = String::new();
                                }
                                _ => {}
                            }
                        }
                        View::Edit => {
                            // Handle key events in edit mode
                            match key.code {
                                KeyCode::Esc => {
                                    self.current_view = View::Main; // Exit edit mode
                                }
                                KeyCode::Enter => {
                                    // Add the new symbol if it's not empty
                                    if !self.new_symbol_input.trim().is_empty() {
                                        let new_symbol = self.new_symbol_input.trim().to_uppercase();
                                        if !self.editing_symbols.contains(&new_symbol) {
                                            self.editing_symbols.push(new_symbol);
                                        }
                                        self.new_symbol_input.clear();
                                    }
                                }
                                KeyCode::Char(c) => {
                                    // Check if this is Ctrl+S (save command)
                                    if c == 's' && key.modifiers.contains(event::KeyModifiers::CONTROL) {
                                        // Save the changes to the config file
                                        let updated_config = stock_predictor_lib::config::StockConfig {
                                            symbols: self.editing_symbols.clone(),
                                            analysis_period_days: 90, // Use current value or get from original config
                                        };

                                        if let Err(e) = stock_predictor_lib::config::write_config(&updated_config, &self.config_file_path) {
                                            // In a real application, you might want to show an error message
                                            eprintln!("Error saving config: {}", e);
                                        } else {
                                            // Return to main view after saving
                                            self.current_view = View::Main;
                                            // Refresh the analyses with new symbols
                                            self.refresh_analyses(&updated_config);
                                        }
                                    } else {
                                        // Add character to the new symbol input
                                        self.new_symbol_input.push(c);
                                    }
                                }
                                KeyCode::Backspace => {
                                    // Remove last character from input
                                    self.new_symbol_input.pop();
                                }
                                KeyCode::Delete => {
                                    // Remove selected symbol
                                    if !self.editing_symbols.is_empty() &&
                                       self.editing_selected_index < self.editing_symbols.len() {
                                        self.editing_symbols.remove(self.editing_selected_index);
                                        if self.editing_selected_index > 0 {
                                            self.editing_selected_index -= 1;
                                        }
                                    }
                                }
                                KeyCode::Up => {
                                    // Move selection up
                                    if self.editing_selected_index > 0 {
                                        self.editing_selected_index -= 1;
                                    }
                                }
                                KeyCode::Down => {
                                    // Move selection down
                                    if !self.editing_symbols.is_empty() &&
                                       self.editing_selected_index < self.editing_symbols.len() - 1 {
                                        self.editing_selected_index += 1;
                                    }
                                }
                                _ => {} // Ignore other keys in edit mode
                            }
                        }
                    }
                }
            }
        }
    }
}

impl App {
    /// Set flag to refresh analyses after saving config
    pub fn refresh_analyses(&mut self, _config: &StockConfig) {
        self.should_refresh_after_save = true;
    }

    /// Initialize data fetching for the given configuration
    fn initialize_data_fetching(&mut self, config: &StockConfig) -> Result<()> {
        use std::sync::mpsc;
        use stock_predictor_lib::{
            analysis::analyze_stock,
            yahooapi::fetch_stock_data,
        };
        use crate::data::TimeRange;

        // Clear existing analyses
        self.analyses.clear();

        let (tx, rx) = mpsc::channel();
        // Store the receiver so we can access it later if needed
        self.channel_rx = Some(rx);
        let default_time_range = TimeRange::OneMonth;

        for symbol in &config.symbols {
            let symbol = symbol.clone();
            let tx = tx.clone();
            let analysis_period_days = config.analysis_period_days;
            self.rt.spawn(async move {
                match fetch_stock_data(&symbol, analysis_period_days).await {
                    Ok(stock_data) => {
                        if !stock_data.is_empty() {
                            let analysis = analyze_stock(&stock_data, &symbol);
                            let _ = tx.send(AppEvent::Update(analysis, stock_data, default_time_range));
                        } else {
                            let _ = tx.send(AppEvent::Error(format!(
                                "No data found for symbol: {}",
                                symbol
                            )));
                        }
                    }
                    Err(e) => {
                        let _ = tx.send(AppEvent::Error(format!(
                            "Error fetching data for {}: {}",
                            symbol, e
                        )));
                    }
                }
            });
        }

        Ok(())
    }
}
