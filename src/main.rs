use anyhow::Result;
use clap::Parser;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Paragraph},
};
use stock_predictor_lib::{
    analysis::{analyze_stock, StockAnalysis},
    config::{read_config, StockConfig},
    yahooapi::fetch_stock_data,
};
use std::{io, sync::mpsc, thread, time::Duration};
use tokio::runtime::Runtime;

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

enum AppEvent {
    Update(StockAnalysis),
    Error(String),
}

fn main() -> Result<()> {
    let args = Args::parse();
    let rt = Runtime::new()?;

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

    let (tx, rx) = mpsc::channel();

    rt.spawn(async move {
        for symbol in &config.symbols {
            match fetch_stock_data(symbol, config.analysis_period_days).await {
                Ok(stock_data) => {
                    if stock_data.len() > 0 {
                        let analysis = analyze_stock(&stock_data, symbol);
                        tx.send(AppEvent::Update(analysis)).unwrap();
                    } else {
                        tx.send(AppEvent::Error(format!("No data found for symbol: {}", symbol))).unwrap();
                    }
                }
                Err(e) => {
                    tx.send(AppEvent::Error(format!("Error fetching data for {}: {}", symbol, e))).unwrap();
                }
            }
            tokio::time::sleep(Duration::from_secs(1)).await;
        }
    });

    let mut analyses = Vec::new();
    let mut is_loading = true;

    loop {
        match rx.try_recv() {
            Ok(app_event) => {
                match app_event {
                    AppEvent::Update(analysis) => analyses.push(analysis),
                    AppEvent::Error(_err) => {
                        // In this case, we'll just ignore them for now
                    }
                }
            }
            Err(mpsc::TryRecvError::Disconnected) => {
                is_loading = false; // All data is loaded
            }
            Err(mpsc::TryRecvError::Empty) => {
                // No new data
            }
        }

        terminal.draw(|f| {
            let size = f.size();
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .margin(1)
                .constraints([Constraint::Percentage(10), Constraint::Percentage(90)].as_ref())
                .split(size);

            let title = Paragraph::new("Stock Predictor").alignment(Alignment::Center);
            f.render_widget(title, chunks[0]);

            let num_stocks = analyses.len();
            if num_stocks == 0 {
                if !is_loading {
                    let text = Paragraph::new("No data to display.").alignment(Alignment::Center);
                    f.render_widget(text, chunks[1]);
                }
                return;
            }
            
            let num_cols = (num_stocks as f32).sqrt().ceil() as usize;
            let num_rows = (num_stocks as f32 / num_cols as f32).ceil() as usize;

            let stock_chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints(
                    (0..num_rows)
                        .map(|_| Constraint::Ratio(1, num_rows as u32))
                        .collect::<Vec<_>>(),
                )
                .split(chunks[1]);

            for i in 0..num_rows {
                let row_chunks = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints(
                        (0..num_cols)
                            .map(|_| Constraint::Ratio(1, num_cols as u32))
                            .collect::<Vec<_>>(),
                    )
                    .split(stock_chunks[i]);

                for j in 0..num_cols {
                    let index = i * num_cols + j;
                    if index < num_stocks {
                        let analysis = &analyses[index];
                        let block = Block::default()
                            .title(analysis.symbol.as_str())
                            .borders(Borders::ALL);
                        
                        let text = vec![
                            Line::from(vec![
                                Span::raw("Price: "),
                                Span::styled(
                                    format!("${:.2}", analysis.current_price),
                                    Style::default().fg(Color::Green),
                                ),
                            ]),
                            Line::from(format!("10-day SMA: ${:.2}", analysis.sma_10.unwrap_or(0.0))),
                            Line::from(format!("50-day SMA: ${:.2}", analysis.sma_50.unwrap_or(0.0))),
                            Line::from(format!("20-day EMA: ${:.2}", analysis.ema_20.unwrap_or(0.0))),
                            Line::from(vec![
                                Span::raw("Trend: "),
                                Span::styled(
                                    format!("{:.2}%", analysis.recent_change.unwrap_or(0.0)),
                                    if analysis.recent_change.unwrap_or(0.0) > 0.0 {
                                        Style::default().fg(Color::Green)
                                    } else {
                                        Style::default().fg(Color::Red)
                                    },
                                ),
                            ]),
                            Line::from(""),
                            Line::from("Predictions:"),
                            Line::from(format!("Day 1: ${:.2}", analysis.predictions[0])),
                            Line::from(format!("Day 2: ${:.2}", analysis.predictions[1])),
                            Line::from(format!("Day 3: ${:.2}", analysis.predictions[2])),
                        ];

                        let paragraph = Paragraph::new(text).block(block);
                        f.render_widget(paragraph, row_chunks[j]);
                    }
                }
            }
        })?;

        if !is_loading {
            thread::sleep(Duration::from_millis(200));
            break;
        }

        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if let KeyCode::Char('q') = key.code {
                    break;
                }
            }
        }
    }

    // restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    Ok(())
}
