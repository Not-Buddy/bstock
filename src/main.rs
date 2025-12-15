use anyhow::Result;
use clap::Parser;
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{prelude::*, backend::CrosstermBackend};
use crate::lib::{config::StockConfig, persistence::PersistenceManager};
use std::io;

mod app;
mod data;
mod event;
mod lib {
    pub mod analysis;
    pub mod config;
    pub mod error;
    pub mod stock_data;
    pub mod yahooapi;
    pub mod persistence;
}
mod ui;

use app::App;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Stock symbols to analyze
    #[arg(short, long, num_args = 1..)]
    symbols: Option<Vec<String>>,

    /// Analysis period in days
    #[arg(short, long)]
    period: Option<i64>,
}

fn main() -> Result<()> {
    let args = Args::parse();

    // setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new()?;

    // Initialize persistence manager
    let persistence_manager = PersistenceManager::new()?;

    let config = if let Some(symbols) = args.symbols {
        let period = args.period.unwrap_or(90);
        let stock_config = StockConfig {
            symbols,
            analysis_period_days: period,
        };
        // Save the command-line config to persistent storage
        persistence_manager.save_stock_config(&stock_config)?;
        stock_config
    } else {
        // Load config from persistent storage
        persistence_manager.get_stock_config()?
    };

    // Use a fixed config file path that represents the persistent storage
    let config_file_path = "persistent_config"; // Placeholder string, won't be used for file operations

    let res = app.run(&mut terminal, &config, config_file_path);

    // restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    res
}