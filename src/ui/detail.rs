use ratatui::{
    prelude::{Constraint, Direction, Layout, Rect, Alignment, Style, Color},
    widgets::{Block, Borders, Paragraph, BorderType},
    Frame,
};
use stock_predictor_lib::stock_data::StockData;

use crate::app::AnalysisWithChartData;
use crate::data::{filter_data_by_time_range, TimeRange};

use super::{chart, metrics};

/// Draw Y-axis labels for the chart
fn draw_y_axis_labels(f: &mut Frame, area: Rect, stock_data: &StockData, time_range: TimeRange) {
    // Filter the stock data based on the selected time range
    let filtered_prices = filter_data_by_time_range(stock_data, time_range);

    let max_price = if !filtered_prices.is_empty() {
        *filtered_prices.iter().max_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal)).unwrap_or(&stock_data.closes.last().copied().unwrap_or(0.0))
    } else {
        stock_data.closes.last().copied().unwrap_or(0.0)
    };

    let min_price = if !filtered_prices.is_empty() {
        *filtered_prices.iter().min_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal)).unwrap_or(&stock_data.closes.last().copied().unwrap_or(0.0))
    } else {
        stock_data.closes.last().copied().unwrap_or(0.0)
    };

    // Calculate a few key values to display as Y-axis labels
    let range = max_price - min_price;
    let y_bounds_min = if range == 0.0 { min_price * 0.8 } else { min_price - 0.1 * range };
    let y_bounds_max = if range == 0.0 { max_price * 1.2 } else { max_price + 0.1 * range };

    // Create Y-axis labels at 4 key positions: max, 3/4, 1/2, 1/4, and min
    let step = (y_bounds_max - y_bounds_min) / 4.0;
    let labels = [
        format!("${:.2}", y_bounds_max),
        format!("${:.2}", y_bounds_max - step),
        format!("${:.2}", y_bounds_max - 2.0 * step),
        format!("${:.2}", y_bounds_max - 3.0 * step),
        format!("${:.2}", y_bounds_min),
    ];

    // Create a vertical layout for the Y-axis labels
    let y_axis_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(25),
            Constraint::Percentage(25),
            Constraint::Percentage(25),
            Constraint::Percentage(25),
            Constraint::Min(0),  // Remaining space
        ])
        .split(area);

    // Render each label
    for (i, (label, chunk)) in labels.iter().zip(y_axis_layout.iter()).enumerate() {
        if i < y_axis_layout.len() {
            let paragraph = Paragraph::new(label.as_str())
                .style(Style::default().fg(Color::Cyan))
                .alignment(Alignment::Right);
            f.render_widget(paragraph, *chunk);
        }
    }
}

/// Renders the user interface for the detailed view.
pub fn draw_detail_ui(f: &mut Frame, data: &AnalysisWithChartData, area: Rect) {
    // Create a layout with header area at the top
    let main_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),      // Header with stock symbol
            Constraint::Min(0),         // Main content area
        ])
        .split(area);

    // Draw the header with stock symbol in top-right
    let header_block = Block::default()
        .borders(Borders::BOTTOM)
        .border_type(BorderType::Plain);
    f.render_widget(header_block, main_layout[0]);

    // Extract and display the stock symbol (from the analysis inside AnalysisWithChartData)
    // Create a paragraph widget for the stock symbol in top-right
    let stock_symbol_widget = Paragraph::new(data.analysis.symbol.clone())
        .style(Style::default().fg(Color::Yellow))
        .alignment(Alignment::Right);

    f.render_widget(stock_symbol_widget, main_layout[0]);

    // Split the main content area horizontally - graph area on left, metrics on right
    let content_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(70),  // Graph area on left
            Constraint::Percentage(30),  // Metrics on right
        ])
        .split(main_layout[1]);

    // Split the graph area vertically to have Y-axis labels on left and chart on right
    let graph_layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(8),       // Y-axis labels on left
            Constraint::Min(0),          // Chart on right
        ])
        .split(content_chunks[0]);

    // Draw the Y-axis labels on the left
    draw_y_axis_labels(f, graph_layout[0], &data.stock_data, data.time_range);

    // Draw the chart on the right (after the Y-axis labels)
    chart::draw_chart(f, &data.stock_data, graph_layout[1], data.time_range);

    // Draw the metrics on the right side of main content
    metrics::draw_metrics(f, &data.stock_data, content_chunks[1]);
}
