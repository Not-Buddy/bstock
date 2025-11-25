use anyhow::Result;
use clap::Parser;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Paragraph, canvas::{Canvas, Line}},
};
use stock_predictor_lib::{
    analysis::{analyze_stock, StockAnalysis},
    config::{read_config, StockConfig},
    yahooapi::fetch_stock_data,
};
use std::{io, sync::mpsc, time::Duration};
use tokio::runtime::Runtime;

// Function to create a simple line chart for a stock based on selected time range
fn create_stock_chart<'a>(
    stock_analysis: &'a StockAnalysis,
    stock_data: &'a StockData,
    time_range: TimeRange
) -> Canvas<'a, Box<dyn Fn(&mut ratatui::widgets::canvas::Context<'_>) + 'a>> {
    // Filter the stock data based on the selected time range
    let filtered_prices = filter_data_by_time_range(stock_data, time_range);

    let max_price = if !filtered_prices.is_empty() {
        *filtered_prices.iter().max_by(|a, b| a.partial_cmp(b).unwrap()).unwrap_or(&stock_analysis.current_price)
    } else {
        stock_analysis.current_price
    };

    let min_price = if !filtered_prices.is_empty() {
        *filtered_prices.iter().min_by(|a, b| a.partial_cmp(b).unwrap()).unwrap_or(&stock_analysis.current_price)
    } else {
        stock_analysis.current_price
    };

    let range = max_price - min_price;
    let y_bounds_min = if range == 0.0 { min_price * 0.8 } else { min_price - 0.1 * range };
    let y_bounds_max = if range == 0.0 { max_price * 1.2 } else { max_price + 0.1 * range };

    let x_bounds_max = (filtered_prices.len() as f64).max(1.0);

    // Clone the data to avoid borrowing issues
    let filtered_prices = filtered_prices;
    let current_price = stock_analysis.current_price;

    Canvas::default()
        .block(Block::default().borders(Borders::ALL).title("Price Chart"))
        .paint(Box::new(move |ctx: &mut ratatui::widgets::canvas::Context<'_>| {
            // Draw a simple line chart from historical data points
            if filtered_prices.len() > 1 {
                for i in 0..filtered_prices.len() - 1 {
                    let x1 = i as f64;
                    let y1 = filtered_prices[i];
                    let x2 = (i + 1) as f64;
                    let y2 = filtered_prices[i + 1];

                    ctx.draw(&Line {
                        x1,
                        y1,
                        x2,
                        y2,
                        color: if y2 >= y1 { Color::Green } else { Color::Red },
                    });
                }
            } else if filtered_prices.len() == 1 {
                // Draw a single point at the current price
                ctx.draw(&Line {
                    x1: 0.0,
                    y1: filtered_prices[0],
                    x2: 1.0,
                    y2: filtered_prices[0],
                    color: Color::Gray,
                });
            } else {
                // Draw a single point at the current price if no historical data
                ctx.draw(&Line {
                    x1: 0.0,
                    y1: current_price,
                    x2: 1.0,
                    y2: current_price,
                    color: Color::Gray,
                });
            }
        }) as Box<dyn Fn(&mut ratatui::widgets::canvas::Context<'_>) + 'a>)
        .marker(Marker::Braille)
        .x_bounds([0.0, x_bounds_max])
        .y_bounds([y_bounds_min, y_bounds_max])
}

// Function to filter stock data based on selected time range
fn filter_data_by_time_range(stock_data: &StockData, time_range: TimeRange) -> Vec<f64> {
    // Since we don't have the exact timestamp for each close value, we'll take the last N values
    // where N corresponds to the time range (approximate)
    let total_points = stock_data.closes.len();
    let points_to_show = match time_range {
        TimeRange::OneDay => std::cmp::min(2, total_points),     // Last day (at least 2 points)
        TimeRange::FiveDays => std::cmp::min(5, total_points),   // Last 5 days
        TimeRange::OneMonth => std::cmp::min(30, total_points),  // Last 30 days
        TimeRange::SixMonths => std::cmp::min(180, total_points), // Last ~6 months
        TimeRange::YTD => std::cmp::min(365, total_points),      // Year to date
    };

    // Take the last N points based on the time range
    let start_index = if total_points > points_to_show {
        total_points - points_to_show
    } else {
        0
    };

    stock_data.closes[start_index..].to_vec()
}

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

use stock_predictor_lib::stock_data::StockData;

#[derive(Clone, Copy, Debug, PartialEq)]
enum TimeRange {
    OneDay = 1,
    FiveDays = 5,
    OneMonth = 30,
    SixMonths = 180,
    YTD = 365, // Year to date (approx.)
}

impl TimeRange {
    fn all() -> Vec<TimeRange> {
        vec![
            TimeRange::OneDay,
            TimeRange::FiveDays,
            TimeRange::OneMonth,
            TimeRange::SixMonths,
            TimeRange::YTD,
        ]
    }

    fn as_str(&self) -> &str {
        match self {
            TimeRange::OneDay => "1D",
            TimeRange::FiveDays => "5D",
            TimeRange::OneMonth => "1M",
            TimeRange::SixMonths => "6M",
            TimeRange::YTD => "YTD",
        }
    }
}

enum AppEvent {
    Update(StockAnalysis, StockData, TimeRange),
    Error(String),
}

// Function to render the time range selector
fn render_time_range_selector(current_time_range: TimeRange, is_selected: bool) -> Paragraph<'static> {
    let mut text = String::new();
    let time_ranges = TimeRange::all();

    for (i, tr) in time_ranges.iter().enumerate() {
        if i > 0 {
            text.push(' ');
        }

        if *tr == current_time_range {
            if is_selected {
                text.push_str(&format!(" [{}] ", tr.as_str()));
            } else {
                text.push_str(&format!(" ({}) ", tr.as_str()));
            }
        } else {
            text.push_str(&format!(" {} ", tr.as_str()));
        }
    }

    Paragraph::new(text)
        .alignment(Alignment::Center)
        .block(Block::default())
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

    // Default time range for initial data fetch
    let default_time_range = TimeRange::OneMonth;

    rt.spawn(async move {
        for symbol in &config.symbols {
            // For now, fetch with the default time range, but in a full implementation
            // we'd want to fetch the maximum range needed and then slice it based on selection
            // Using the config analysis period for now
            match fetch_stock_data(symbol, config.analysis_period_days).await {
                Ok(stock_data) => {
                    if !stock_data.is_empty() {
                        let analysis = analyze_stock(&stock_data, symbol);
                        tx.send(AppEvent::Update(analysis, stock_data, default_time_range)).unwrap();
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

    // Create a struct to hold both the analysis and the original stock data
    struct AnalysisWithChartData {
        analysis: StockAnalysis,
        stock_data: StockData,
        time_range: TimeRange,
    }

    let mut analyses = Vec::<AnalysisWithChartData>::new();
    let mut selected_index = 0;
    // Track selected time range index for the currently selected stock
    let mut selected_time_range_index = 0; // Default to first time range

    loop {
        if let Ok(app_event) = rx.try_recv() {
            match app_event {
                AppEvent::Update(analysis, stock_data, time_range) => {
                    analyses.push(AnalysisWithChartData {
                        analysis,
                        stock_data,
                        time_range,
                    });
                },
                AppEvent::Error(_err) => {
                    // In this case, we'll just ignore them for now
                }
            }
        }

        terminal.draw(|f| {
            let size = f.size();
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .margin(1)
                .constraints([Constraint::Percentage(10), Constraint::Percentage(80), Constraint::Percentage(10)].as_ref())
                .split(size);

            let num_stocks = analyses.len();
            let num_pages = (num_stocks as f32 / 4.0).ceil() as usize;
            let current_page = selected_index / 4 + 1;

            let title = Paragraph::new(format!("Stock Predictor - Page {}/{}", current_page, num_pages))
                .alignment(Alignment::Center);
            f.render_widget(title, chunks[0]);

            if num_stocks == 0 {
                let text = Paragraph::new("Loading data...").alignment(Alignment::Center);
                f.render_widget(text, chunks[1]);
                return;
            }

            let num_cols = 2;
            let num_rows = 2;

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
                    let index = (current_page - 1) * 4 + i * num_cols + j;
                    if index < num_stocks {
                        let analysis_with_data = &analyses[index];
                        let analysis = &analysis_with_data.analysis;
                        let stock_data = &analysis_with_data.stock_data;

                        // Create a detailed block with a chart
                        let mut block = Block::default()
                            .title(analysis.symbol.as_str())
                            .borders(Borders::ALL);

                        if index == selected_index {
                            block = block.border_style(Style::default().fg(Color::Yellow));
                        }

                        // Draw the border first
                        f.render_widget(block.clone(), row_chunks[j]);

                        // Get the inner area of the block for content
                        let inner_area = block.inner(row_chunks[j]);

                        // Split area for content and time range selector
                        let content_with_selector = Layout::default()
                            .direction(Direction::Vertical)
                            .constraints([
                                Constraint::Min(10),      // Main content area (text + chart)
                                Constraint::Length(3),    // Time range selector
                            ])
                            .split(inner_area);

                        // Split the main content area for text and chart
                        let text_and_chart_chunks = Layout::default()
                            .direction(Direction::Horizontal)
                            .constraints([
                                Constraint::Percentage(60),  // 60% for text
                                Constraint::Percentage(40),  // 40% for chart
                            ])
                            .split(content_with_selector[0]);

                        // Render the text details
                        let text = vec![
                            ratatui::text::Line::from(vec![
                                Span::raw("Price: "),
                                Span::styled(
                                    format!("${:.2}", analysis.current_price),
                                    Style::default().fg(Color::Green),
                                ),
                            ]),
                            ratatui::text::Line::from(format!("10-day SMA: ${:.2}", analysis.sma_10.unwrap_or(0.0))),
                            ratatui::text::Line::from(format!("50-day SMA: ${:.2}", analysis.sma_50.unwrap_or(0.0))),
                            ratatui::text::Line::from(format!("20-day EMA: ${:.2}", analysis.ema_20.unwrap_or(0.0))),
                            ratatui::text::Line::from(vec![
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
                            ratatui::text::Line::from(""),
                            ratatui::text::Line::from("Predictions:"),
                            ratatui::text::Line::from(format!("Day 1: ${:.2}", analysis.predictions[0])),
                            ratatui::text::Line::from(format!("Day 2: ${:.2}", analysis.predictions[1])),
                            ratatui::text::Line::from(format!("Day 3: ${:.2}", analysis.predictions[2])),
                        ];

                        let paragraph = Paragraph::new(text);
                        f.render_widget(paragraph, text_and_chart_chunks[0]);

                        // Render the chart with the selected time range
                        let chart = create_stock_chart(analysis, stock_data, analysis_with_data.time_range);
                        f.render_widget(chart, text_and_chart_chunks[1]);

                        // Render the time range selector below the chart
                        let time_range_selector = render_time_range_selector(analysis_with_data.time_range, selected_index == index);
                        f.render_widget(time_range_selector, content_with_selector[1]);
                    }
                }
            }

            let help_text = Paragraph::new("Use arrow keys to change pages, 'q' or Ctrl-C to quit.")
                .alignment(Alignment::Center);
            f.render_widget(help_text, chunks[2]);
        })?;

        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') => break,
                    KeyCode::Char('c') if key.modifiers == event::KeyModifiers::CONTROL => break,
                    KeyCode::Left => {
                        if selected_index > 0 {
                            selected_index -= 1;
                        }
                    }
                    KeyCode::Right => {
                        if !analyses.is_empty() && selected_index < analyses.len() - 1 {
                            selected_index += 1;
                        }
                    }
                    KeyCode::Up => {
                        if !analyses.is_empty() && selected_index < analyses.len() {
                            // Change time range to previous option
                            if selected_time_range_index > 0 {
                                selected_time_range_index -= 1;
                            } else {
                                selected_time_range_index = TimeRange::all().len() - 1; // Wrap to last
                            }
                            // Update the time range for the currently selected stock
                            if !analyses.is_empty() {
                                analyses[selected_index].time_range = TimeRange::all()[selected_time_range_index];
                            }
                        }
                    }
                    KeyCode::Down => {
                        if !analyses.is_empty() && selected_index < analyses.len() {
                            // Change time range to next option
                            if selected_time_range_index < TimeRange::all().len() - 1 {
                                selected_time_range_index += 1;
                            } else {
                                selected_time_range_index = 0; // Wrap to first
                            }
                            // Update the time range for the currently selected stock
                            if !analyses.is_empty() {
                                analyses[selected_index].time_range = TimeRange::all()[selected_time_range_index];
                            }
                        }
                    }
                    _ => {}
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
