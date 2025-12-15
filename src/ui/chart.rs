use ratatui::{
    prelude::*,
    widgets::{Block, Borders, canvas::{Canvas, Line,}},
};
use stock_predictor_lib::{
    analysis::{StockAnalysis},
    stock_data::StockData,
};
use crate::data::{filter_data_by_time_range, TimeRange};

// Function to create a simple line chart for a stock based on selected time range
pub fn create_stock_chart<'a>(
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
