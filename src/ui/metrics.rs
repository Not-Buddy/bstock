use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Paragraph},
};
use crate::lib::{
    analysis::StockAnalysis,
    stock_data::StockData,
};
use crate::data::{calculate_volatility, TimeRange};

/// Render the metrics panel with real analysis data.
pub fn draw_metrics(
    f: &mut Frame,
    analysis: &StockAnalysis,
    stock_data: &StockData,
    area: Rect,
    time_range: TimeRange,
) {
    let widget = render_metrics(analysis, stock_data, time_range);
    f.render_widget(widget, area);
}

pub fn render_metrics(
    analysis: &StockAnalysis,
    stock_data: &StockData,
    time_range: TimeRange,
) -> Paragraph<'static> {
    let high = stock_data.closes.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let low = stock_data.closes.iter().cloned().fold(f64::INFINITY, f64::min);
    let current = analysis.current_price;

    let from_high_pct = ((current - high) / high) * 100.0;
    let from_low_pct = ((current - low) / low) * 100.0;

    let avg_vol: u64 = if !stock_data.volumes.is_empty() {
        (stock_data.volumes.iter().sum::<u64>() as f64
            / stock_data.volumes.len() as f64) as u64
    } else {
        0
    };

    let volatility = calculate_volatility(&stock_data.closes);

    let change_str = analysis
        .recent_change
        .map_or_else(|| String::from("--"), |c| format!("{:+.2}%", c));

    let sma10_str = analysis.sma_10.map_or_else(|| "--".into(), |v| format!("${:.2}", v));
    let sma50_str = analysis.sma_50.map_or_else(|| "--".into(), |v| format!("${:.2}", v));
    let ema20_str = analysis.ema_20.map_or_else(|| "--".into(), |v| format!("${:.2}", v));

    // Colour-coded legend line
    let legend = "\n  ■Price  ■SMA10  ■SMA50  ■EMA20  ◆Pred";

    let text = format!(
        " Price:  ${:.2}\n\
         Change: {}\n\
         ──────────────────\n\
         SMA-10: {}\n\
         SMA-50: {}\n\
         EMA-20: {}\n\
         ──────────────────\n\
         Hi:     ${:.2}\n\
         Lo:     ${:.2}\n\
         Hi%:    {:+.2}%\n\
         Lo%:    {:+.2}%\n\
         ──────────────────\n\
         Vol:    {:.2}%\n\
         AvgVol: {}\n\
         ──────────────────\n\
         Range:  {}\
         {}",
        current,
        change_str,
        sma10_str,
        sma50_str,
        ema20_str,
        high,
        low,
        from_high_pct,
        from_low_pct,
        volatility,
        fmt_volume(avg_vol),
        time_range.as_str(),
        legend,
    );

    Paragraph::new(text)
        .block(Block::default().borders(Borders::ALL).title(" Metrics "))
        .style(Style::default().fg(Color::White))
}

/// Compact volume formatting: 1.2M, 345K, etc.
pub fn fmt_volume(v: u64) -> String {
    if v >= 1_000_000 {
        format!("{:.1}M", v as f64 / 1_000_000.0)
    } else if v >= 1_000 {
        format!("{}K", v / 1_000)
    } else {
        v.to_string()
    }
}
