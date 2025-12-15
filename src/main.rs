use anyhow::Result;
use clap::Parser;
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{prelude::*, backend::CrosstermBackend};
use stock_predictor_lib::config::{read_config, StockConfig};
use std::io;

mod app;
mod data;
mod event;
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

    let config = if let Some(symbols) = args.symbols {
        let period = args.period.unwrap_or(90);
        StockConfig {
            symbols,
            analysis_period_days: period,
        }
    } else {
        read_config("stocks_config.json")?
    };

    // setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new()?;
    let res = app.run(&mut terminal, &config);

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