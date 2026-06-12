use ratatui::{
    prelude::{Constraint, Direction, Layout, Rect, Alignment, Style, Color, text::Span},
    widgets::{Paragraph},
    Frame,
};

use crate::app::AnalysisWithChartData;
use crate::data::{filter_bars, TimeRange};

use super::{chart, metrics};

/// Y-axis price labels (ratatui text — always sharp & readable).
fn draw_y_axis(f: &mut Frame, area: Rect, y_lo: f64, y_hi: f64) {
    let labels = chart::y_axis_labels(y_lo, y_hi, 5);
    let chunks = Layout::default().direction(Direction::Vertical).constraints([
        Constraint::Percentage(20), Constraint::Percentage(20),
        Constraint::Percentage(20), Constraint::Percentage(20),
        Constraint::Min(0),
    ]).split(area);
    for (label, chunk) in labels.iter().zip(chunks.iter()) {
        f.render_widget(
            Paragraph::new(label.as_str()).style(Style::default().fg(Color::Cyan)).alignment(Alignment::Right),
            *chunk,
        );
    }
}

/// X-axis date labels evenly distributed under the chart.
fn draw_x_axis(f: &mut Frame, area: Rect, ts: &[i64], n: usize, time_range: TimeRange) {
    if n == 0 || ts.is_empty() { return; }
    let start = ts.len().saturating_sub(n);
    let tss = &ts[start..];
    let positions: Vec<usize> = if n <= 5 { (0..n).collect() }
        else { (0..=4).map(|i| ((i as f64) * (n - 1) as f64 / 4.0).round() as usize).collect() };
    let labels: Vec<String> = positions.iter().filter_map(|&pos| {
        let ts_val = *tss.get(pos)?;
        let dt = chrono::DateTime::from_timestamp(ts_val, 0)?;
        Some(match time_range {
            TimeRange::OneDay | TimeRange::FiveDays => dt.format("%m/%d").to_string(),
            TimeRange::OneMonth => dt.format("%m/%d").to_string(),
            TimeRange::SixMonths => dt.format("%b").to_string(),
        })
    }).collect();
    let w = area.width as usize;
    let spacer = " ".repeat(w.saturating_sub(labels.iter().map(|s| s.len() + 2).sum::<usize>()).max(1));
    let spans: Vec<Span> = labels.iter().enumerate().flat_map(|(i, l)| {
        let mut v = vec![];
        if i > 0 { v.push(Span::raw(spacer.clone())); }
        v.push(Span::styled(l.clone(), Style::default().fg(Color::DarkGray)));
        v
    }).collect();
    f.render_widget(Paragraph::new(ratatui::text::Line::from(spans)).alignment(Alignment::Center), area);
}

/// Renders the detail view: header, chart, volume, crosshair info, metrics.
pub fn draw_detail_ui(
    f: &mut Frame,
    data: &AnalysisWithChartData,
    area: Rect,
    crosshair_index: Option<usize>,
) {
    let bars = filter_bars(&data.stock_data, data.time_range);
    let n_bars = bars.len();
    let all_y: Vec<f64> = bars.iter().flat_map(|b| [b.high, b.low]).collect();
    let y_max = all_y.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let y_min = all_y.iter().cloned().fold(f64::INFINITY, f64::min);
    let (y_lo, y_hi, _step) = chart::nice_y_bounds(y_min, y_max);

    // ── title ───────────────────────────────────────────
    let v = Layout::default().direction(Direction::Vertical).constraints([
        Constraint::Length(1), Constraint::Min(0),
    ]).split(area);
    f.render_widget(
        Paragraph::new(format!(" {}  |  {}  |  ←→ crosshair  ↑↓ range  Esc back ", data.analysis.symbol, data.time_range.as_str()))
            .style(Style::default().fg(Color::Yellow)),
        v[0],
    );

    // ── body ────────────────────────────────────────────
    let body = Layout::default().direction(Direction::Horizontal).constraints([
        Constraint::Length(8), Constraint::Min(0), Constraint::Length(22),
    ]).split(v[1]);

    let mut cc: Vec<Constraint> = vec![Constraint::Min(8), Constraint::Percentage(18)]; // chart + volume
    cc.push(Constraint::Length(1)); // x-axis
    cc.push(Constraint::Length(1)); // legend
    if crosshair_index.is_some() { cc.push(Constraint::Length(1)); }
    let chart_col = Layout::default().direction(Direction::Vertical).constraints(cc).split(body[1]);

    // ── Y-axis ──────────────────────────────────────────
    draw_y_axis(f, body[0], y_lo, y_hi);

    // ── Price chart ─────────────────────────────────────
    let title = format!(" {} | {} ", data.analysis.symbol, data.time_range.as_str());
    let xhair_x = crosshair_index.map(|i| i as f64);
    let price_canvas = chart::create_price_chart(
        &data.stock_data, &data.analysis, data.time_range, xhair_x, &title,
        chart_col[0].width,
    );
    f.render_widget(price_canvas, chart_col[0]);

    // ── Volume chart ────────────────────────────────────
    f.render_widget(
        chart::create_volume_chart(&data.stock_data, data.time_range, chart_col[1].width),
        chart_col[1],
    );

    // ── X-axis ──────────────────────────────────────────
    draw_x_axis(f, chart_col[2], &data.stock_data.timestamps, n_bars, data.time_range);

    // ── Legend ──────────────────────────────────────────
    f.render_widget(chart::create_legend_line(), chart_col[3]);

    // ── Crosshair info ──────────────────────────────────
    if let Some(idx) = crosshair_index
        && let Some(snap) = chart::crosshair_info(&data.stock_data, data.time_range, idx)
    {
        let info = Paragraph::new(format!(
            " {} │ ${:.2} │ O:${:.2} H:${:.2} L:${:.2} C:${:.2} │ Vol: {} │ SMA10: {} SMA50: {} EMA20: {} │ {}/{} ",
            snap.date, snap.price,
            bars[idx].open, bars[idx].high, bars[idx].low, bars[idx].close,
            metrics::fmt_volume(snap.volume),
            snap.sma10.map_or("--".into(), |v| format!("${:.2}", v)),
            snap.sma50.map_or("--".into(), |v| format!("${:.2}", v)),
            snap.ema20.map_or("--".into(), |v| format!("${:.2}", v)),
            snap.index + 1, snap.total,
        )).style(Style::default().fg(Color::LightYellow)).alignment(Alignment::Center);
        f.render_widget(info, chart_col[4]);
    }

    // ── Metrics ─────────────────────────────────────────
    metrics::draw_metrics(f, &data.analysis, &data.stock_data, body[2], data.time_range);
}
