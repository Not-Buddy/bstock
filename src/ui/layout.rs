use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Paragraph, Clear},
};
use crate::{
    app::AnalysisWithChartData,
    ui::{
        chart::create_stock_chart, metrics::render_additional_metrics,
        selector::render_time_range_selector,
    },
};

pub fn draw_ui(f: &mut Frame, analyses: &[AnalysisWithChartData], selected_index: usize) {
    let size = f.size();

    // Check if terminal is too small and display overlay if needed
    if size.width < 100 || size.height < 35 {
        // Create overlay for small terminal message
        let overlay_area = Rect::new(
            size.width.saturating_sub(50) / 2,
            size.height.saturating_sub(10) / 2,
            50.min(size.width),
            10.min(size.height),
        );

        let block = Block::default()
            .borders(Borders::ALL)
            .title("Terminal Size Warning");

        // Determine colors based on whether dimensions meet requirements
        let width_color = if size.width >= 100 { Color::Green } else { Color::Red };
        let height_color = if size.height >= 35 { Color::Green } else { Color::Red };

        // Create colored text for dimensions
        let text = vec![
            ratatui::text::Line::from("Terminal size:"),
            ratatui::text::Line::from(vec![
                Span::raw("  Width = "),
                Span::styled(size.width.to_string(), Style::default().fg(width_color)),
                Span::raw("    Height = "),
                Span::styled(size.height.to_string(), Style::default().fg(height_color)),
            ]),
            ratatui::text::Line::from(""),
            ratatui::text::Line::from("Needed for current config:"),
            ratatui::text::Line::from("  Width = 100  Height = 35"),
        ];

        let paragraph = Paragraph::new(text).block(block).alignment(Alignment::Center);

        f.render_widget(Clear, overlay_area); // Clear the area to create the modal effect
        f.render_widget(paragraph, overlay_area);
    } else {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(1)
            .constraints(
                [
                    Constraint::Percentage(10),
                    Constraint::Percentage(80),
                    Constraint::Percentage(10),
                ]
                .as_ref(),
            )
            .split(size);

        let num_stocks = analyses.len();
        let num_pages = (num_stocks as f32 / 4.0).ceil() as usize;
        let current_page = selected_index / 4 + 1;

        let title =
            Paragraph::new(format!("Stock Predictor - Page {}/{}", current_page, num_pages))
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
                            Constraint::Min(10),   // Main content area (text + chart)
                            Constraint::Length(3), // Time range selector
                        ])
                        .split(inner_area);

                    // Split the main content area for text, metrics and chart
                    let main_content_chunks = Layout::default()
                        .direction(Direction::Horizontal)
                        .constraints([
                            Constraint::Percentage(45), // 45% for text details
                            Constraint::Percentage(20), // 20% for metrics
                            Constraint::Percentage(35), // 35% for chart
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
                        ratatui::text::Line::from(format!(
                            "10-day SMA: ${:.2}",
                            analysis.sma_10.unwrap_or(0.0)
                        )),
                        ratatui::text::Line::from(format!(
                            "50-day SMA: ${:.2}",
                            analysis.sma_50.unwrap_or(0.0)
                        )),
                        ratatui::text::Line::from(format!(
                            "20-day EMA: ${:.2}",
                            analysis.ema_20.unwrap_or(0.0)
                        )),
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
                        ratatui::text::Line::from(format!(
                            "Day 1: ${:.2}",
                            analysis.predictions[0]
                        )),
                        ratatui::text::Line::from(format!(
                            "Day 2: ${:.2}",
                            analysis.predictions[1]
                        )),
                        ratatui::text::Line::from(format!(
                            "Day 3: ${:.2}",
                            analysis.predictions[2]
                        )),
                    ];

                    // Render the text details
                    let paragraph = Paragraph::new(text);
                    f.render_widget(paragraph, main_content_chunks[0]);

                    // Render the additional metrics
                    let metrics = render_additional_metrics(
                        stock_data,
                        analysis,
                        analysis_with_data.time_range,
                    );
                    f.render_widget(metrics, main_content_chunks[1]);

                    // Render the chart with the selected time range
                    let chart = create_stock_chart(
                        analysis,
                        stock_data,
                        analysis_with_data.time_range,
                    );
                    f.render_widget(chart, main_content_chunks[2]);

                    // Render the time range selector below the chart
                    let time_range_selector = render_time_range_selector(
                        analysis_with_data.time_range,
                        selected_index == index,
                    );
                    f.render_widget(time_range_selector, content_with_selector[1]);
                }
            }
        }

        let help_text = Paragraph::new("Use arrow keys to change pages and 'e' to edit stocks , 'q' or Ctrl-C to quit.")
            .alignment(Alignment::Center);
        f.render_widget(help_text, chunks[2]);
    }
}
