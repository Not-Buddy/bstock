use anyhow::Result;
use crossterm::event::{self, Event, KeyCode};
use ratatui::prelude::*;

use stock_predictor_lib::{
    analysis::{analyze_stock, StockAnalysis},
    config::{StockConfig},
    stock_data::StockData,
    yahooapi::fetch_stock_data,
};
use std::{io, sync::mpsc, time::Duration};
use tokio::runtime::Runtime;

use crate::{
    data::TimeRange,
    event::AppEvent,
    ui::{detail::draw_detail_ui, layout::draw_ui},
};

pub enum View {
    Main,
    Detail,
}

pub struct AnalysisWithChartData {
    pub analysis: StockAnalysis,
    pub stock_data: StockData,
    pub time_range: TimeRange,
}

pub struct App {
    analyses: Vec<AnalysisWithChartData>,
    selected_index: usize,
    selected_time_range_index: usize,
    rt: Runtime,
    current_view: View,
}

impl App {
    pub fn new() -> Result<Self> {
        Ok(Self {
            analyses: Vec::new(),
            selected_index: 0,
            selected_time_range_index: 0,
            rt: Runtime::new()?,
            current_view: View::Main,
        })
    }

    pub fn run(
        &mut self,
        terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
        config: &StockConfig,
    ) -> Result<()> {
        let (tx, rx) = mpsc::channel();
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
                            tx.send(AppEvent::Update(analysis, stock_data, default_time_range))
                                .unwrap();
                        } else {
                            tx.send(AppEvent::Error(format!(
                                "No data found for symbol: {}",
                                symbol
                            )))
                            .unwrap();
                        }
                    }
                    Err(e) => {
                        tx.send(AppEvent::Error(format!(
                            "Error fetching data for {}: {}",
                            symbol, e
                        )))
                        .unwrap();
                    }
                }
            });
        }

        loop {
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
            }

            if event::poll(Duration::from_millis(100))? {
                if let Event::Key(key) = event::read()? {
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
                            self.current_view = View::Main;
                        }
                        _ => {}
                    }
                }
            }
        }
    }
}
