use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Paragraph, Clear},
};
use crate::{
    app::AnalysisWithChartData,
    ui::{
        metrics::render_metrics,
        selector::render_time_range_selector,
    },
};

pub fn draw_ui(
    f: &mut Frame,
    analyses: &[AnalysisWithChartData],
    selected_index: usize,
    loading_total: usize,
    loading_done: usize,
    loading_errors: &[String],
) {
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
            Paragraph::new(format!("Bstock - Page {}/{}", current_page, num_pages))
                .alignment(Alignment::Center);
        f.render_widget(title, chunks[0]);

        if num_stocks == 0 {
            let loading = loading_total > 0;
            let done = loading_done >= loading_total && loading_total > 0;
            let msg = if loading {
                let pct = if loading_total > 0 {
                    loading_done * 100 / loading_total
                } else {
                    0
                };
                let bar_width = 40usize;
                let filled = bar_width * loading_done / loading_total.max(1);
                let bar = format!(
                    "▐{}{}▌",
                    "█".repeat(filled),
                    "░".repeat(bar_width.saturating_sub(filled))
                );
                let mut text = format!(
                    "Fetching stock data…\n\n{}  {}/{}  ({}%)\n",
                    bar, loading_done, loading_total, pct
                );
                if done
                    && !loading_errors.is_empty() {
                        text.push_str("\n── ERRORS ──────────────────────────────\n");
                        for (i, err) in loading_errors.iter().enumerate() {
                            let truncated = if err.len() > 80 {
                                format!("{}…", &err[..77])
                            } else {
                                err.clone()
                            };
                            text.push_str(&format!("  {}. {}\n", i + 1, truncated));
                        }
                        text.push_str("────────────────────────────────────────\n");
                        text.push_str("\nNo data loaded. Check your connection or try again.\n");
                    }
                text.push_str("\nPress q to quit");
                text
            } else {
                "Loading data…".into()
            };
            let text = Paragraph::new(msg).alignment(Alignment::Center);
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
                            analysis.predictions.first().copied().unwrap_or(0.0)
                        )),
                        ratatui::text::Line::from(format!(
                            "Day 2: ${:.2}",
                            analysis.predictions.get(1).copied().unwrap_or(0.0)
                        )),
                        ratatui::text::Line::from(format!(
                            "Day 3: ${:.2}",
                            analysis.predictions.get(2).copied().unwrap_or(0.0)
                        )),
                    ];

                    // Render the text details
                    let paragraph = Paragraph::new(text);
                    f.render_widget(paragraph, main_content_chunks[0]);

                    // Render the additional metrics
                    let metrics = render_metrics(
                        analysis,
                        stock_data,
                        analysis_with_data.time_range,
                    );
                    f.render_widget(metrics, main_content_chunks[1]);

                    // Render the chart with the selected time range (Braille Canvas)
                    let bars = crate::data::filter_bars(stock_data, analysis_with_data.time_range);
                    let full_len = stock_data.closes.len();
                    let prev_close = if bars.len() >= 2 {
                        Some(bars[bars.len() - 2].close)
                    } else {
                        None
                    };
                    let chart = crate::ui::chart::create_price_chart(
                        &bars, full_len, analysis,
                        None, analysis.symbol.as_str(),
                        main_content_chunks[2].width, prev_close,
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

        // ── bottom bar: chart legend + help + loading indicator ──
        let bottom = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1),  // legend
                Constraint::Length(1),  // help + loading
            ])
            .split(chunks[2]);

        let legend = crate::ui::chart::create_legend_line();
        f.render_widget(legend, bottom[0]);

        // Help row: left-aligned help text, right-aligned loading indicator
        let help_row = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Min(0), Constraint::Length(30)])
            .split(bottom[1]);

        let help = Paragraph::new(
            "←→ select stock │ ↑↓ time range │ Enter details │ e edit │ q quit",
        )
        .alignment(Alignment::Left)
        .style(Style::default().fg(Color::DarkGray));
        f.render_widget(help, help_row[0]);

        if loading_total > 0 && loading_done < loading_total {
            let bar_w = 12usize;
            let filled = bar_w * loading_done / loading_total.max(1);
            let spinner = ['◐', '◓', '◑', '◒'][(loading_done * 2) % 4];
            let load_text = format!(
                " {} ▐{}{}▌ {}/{} ",
                spinner,
                "█".repeat(filled),
                "░".repeat(bar_w.saturating_sub(filled)),
                loading_done,
                loading_total,
            );
            let load_widget = Paragraph::new(load_text)
                .alignment(Alignment::Right)
                .style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD));
            f.render_widget(load_widget, help_row[1]);
        }
    }
}
