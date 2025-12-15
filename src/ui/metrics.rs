use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Paragraph},
};
use crate::lib::{
    analysis::{StockAnalysis},
    stock_data::StockData,
};
use crate::data::{calculate_volatility, TimeRange};

// Function to draw metrics for the selected stock
pub fn draw_metrics(f: &mut Frame, stock_data: &StockData, area: Rect) {
    // Create a dummy analysis - using current price from stock data
    let dummy_analysis = crate::lib::analysis::StockAnalysis {
        symbol: String::from("---"), // Placeholder
        current_price: stock_data.closes.last().copied().unwrap_or(0.0),
        sma_10: None,
        sma_50: None,
        ema_20: None,
        predictions: vec![],
        recent_change: None,
    };

    let metrics_widget = render_additional_metrics(stock_data, &dummy_analysis, TimeRange::OneMonth); // Default time range
    f.render_widget(metrics_widget, area);
}

// Function to render additional metrics for the selected stock
pub fn render_additional_metrics(stock_data: &StockData, analysis: &StockAnalysis, time_range: TimeRange) -> Paragraph<'static> {
    // Calculate various metrics based on the stock data
    let high_52w = stock_data.closes.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let low_52w = stock_data.closes.iter().cloned().fold(f64::INFINITY, f64::min);
    let current_price = analysis.current_price;

    let change_from_high = ((current_price - high_52w) / high_52w) * 100.0;
    let change_from_low = ((current_price - low_52w) / low_52w) * 100.0;

    let avg_volume: u64 = if !stock_data.volumes.is_empty() {
        (stock_data.volumes.iter().sum::<u64>() as f64 / stock_data.volumes.len() as f64) as u64
    } else {
        0
    };

    // Calculate volatility based on the standard deviation of returns
    let volatility = calculate_volatility(&stock_data.closes);

    // Format the metrics text with shorter labels to fit in smaller space
    let metrics_text = format!(
        "Hi: ${:.2}\nLo: ${:.2}\nHi%: {:.2}%\nLo%: {:.2}%\nVol: {:.2}%\nVol: {}\n\n{}",
        high_52w,
        low_52w,
        change_from_high,
        change_from_low,
        volatility,
        avg_volume,
        time_range.as_str()
    );

    Paragraph::new(metrics_text)
        .block(Block::default().borders(Borders::ALL).title("Metrics"))
        .style(Style::default().fg(Color::White))
}
