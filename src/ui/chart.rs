use ratatui::{
    prelude::*,
    text::{Line as TextLine, Span},
    widgets::{
        Block, Borders, Paragraph,
        canvas::{Canvas, Line},
    },
};
use crate::data::{filter_bars, TimeRange};
use crate::lib::{analysis::StockAnalysis, stock_data::StockData};

type CanvasFn<'a> = Box<dyn Fn(&mut ratatui::widgets::canvas::Context<'_>) + 'a>;

// ── colours ────────────────────────────────────────────────────
const GRID_C: Color = Color::DarkGray;
const PRED_C: Color = Color::Gray;
const SMA10_C: Color = Color::Yellow;
const SMA50_C: Color = Color::Cyan;
const EMA20_C: Color = Color::Magenta;
const XHAIR_C: Color = Color::LightYellow;
const CANDLE_UP: Color = Color::Green;
const CANDLE_DOWN: Color = Color::Red;
const VOL_UP: Color = Color::Green;
const VOL_DOWN: Color = Color::Red;

// ── nice-number axis ───────────────────────────────────────────

pub fn nice_y_bounds(min_val: f64, max_val: f64) -> (f64, f64, f64) {
    if (max_val - min_val).abs() < 1e-9 {
        let c = min_val;
        return (c * 0.95, c * 1.05, c * 0.05);
    }
    let range = max_val - min_val;
    let rough = range / 4.0;
    let exp = (rough.log10()).floor();
    let mantissa = rough / 10f64.powf(exp);
    let nice = if mantissa <= 1.0 { 1.0 }
        else if mantissa <= 2.0 { 2.0 }
        else if mantissa <= 5.0 { 5.0 }
        else { 10.0 };
    let step = nice * 10f64.powf(exp);
    let lo = (min_val / step).floor() * step;
    let hi = (max_val / step).ceil() * step;
    let pad = (hi - lo) * 0.05;
    (lo - pad, hi + pad, step)
}

pub fn y_axis_labels(lo: f64, hi: f64, n: usize) -> Vec<String> {
    let step = (hi - lo) / (n as f64 - 1.0).max(1.0);
    (0..n).map(|i| {
        let v = hi - step * i as f64;
        if v >= 1000.0 { format!("${:.0}", v) }
        else if v >= 1.0 { format!("${:.2}", v) }
        else { format!("${:.4}", v) }
    }).collect()
}

// ── helpers ────────────────────────────────────────────────────

fn align_overlay(overlay: &[f64], full_start: usize, n: usize, period: usize) -> Vec<(f64, f64)> {
    let first = full_start.saturating_sub(period);
    let last = (full_start + n).saturating_sub(period).min(overlay.len());
    if first >= last { return vec![]; }
    (first..last)
        .map(|oi| ((oi + period - full_start) as f64, overlay[oi]))
        .collect()
}

/// Draw a continuous line series point-to-point.
/// Bresenham interpolation happens inside ratatui's `Line` draw.
fn draw_series(
    ctx: &mut ratatui::widgets::canvas::Context<'_>,
    pts: &[(f64, f64)],
    color: Color,
) {
    for w in pts.windows(2) {
        ctx.draw(&Line { x1: w[0].0, y1: w[0].1, x2: w[1].0, y2: w[1].1, color });
    }
}

/// Draw a dashed series (alternating on/off segments).
fn draw_dashed(
    ctx: &mut ratatui::widgets::canvas::Context<'_>,
    pts: &[(f64, f64)],
    color: Color,
    dash_len: f64,
) {
    for w in pts.windows(2) {
        let dx = w[1].0 - w[0].0;
        let dy = w[1].1 - w[0].1;
        let dist = (dx * dx + dy * dy).sqrt();
        if dist < 1e-9 { continue; }
        let segs = (dist / dash_len).ceil() as usize;
        for s in 0..segs {
            if s % 2 == 0 {
                let t0 = s as f64 / segs as f64;
                let t1 = ((s + 1) as f64 / segs as f64).min(1.0);
                ctx.draw(&Line {
                    x1: w[0].0 + t0 * dx, y1: w[0].1 + t0 * dy,
                    x2: w[0].0 + t1 * dx, y2: w[0].1 + t1 * dy,
                    color,
                });
            }
        }
    }
}

/// Draw a single OHLC candle with dynamically-scaled body width.
///
/// `dot_x` is the coordinate width of one Braille dot  (= x_range / (char_cells * 2)).
/// `gap_x` is the coordinate distance between consecutive candles  (= x_range / n).
#[allow(clippy::too_many_arguments)]
fn draw_candle(
    ctx: &mut ratatui::widgets::canvas::Context<'_>,
    x: f64, open: f64, high: f64, low: f64, close: f64,
    dot_x: f64,
    gap_x: f64,
) {
    let up = close >= open;
    let color = if up { CANDLE_UP } else { CANDLE_DOWN };
    let top = if up { close } else { open };
    let bot = if up { open } else { close };

    // Wick: razor-thin single vertical line at exact centre
    ctx.draw(&Line { x1: x, y1: low, x2: x, y2: high, color });

    // Body: step by dot_x * 0.5 to guarantee solid fill (slight overlap)
    let half_width = (gap_x * 0.4).max(dot_x * 0.5); // 80 % width, minimum 1 dot
    let step = dot_x * 0.5;
    let mut dx = -half_width;
    while dx <= half_width {
        ctx.draw(&Line { x1: x + dx, y1: bot, x2: x + dx, y2: top, color });
        dx += step;
    }
}

/// Draw a solid volume bar with dynamically-scaled width.
fn draw_vol_bar(
    ctx: &mut ratatui::widgets::canvas::Context<'_>,
    x: f64, h: f64, up: bool,
    dot_x: f64,
    gap_x: f64,
) {
    let color = if up { VOL_UP } else { VOL_DOWN };
    let half_width = (gap_x * 0.4).max(dot_x * 0.5);
    let step = dot_x * 0.5;
    let mut dx = -half_width;
    while dx <= half_width {
        ctx.draw(&Line { x1: x + dx, y1: 0.0, x2: x + dx, y2: h, color });
        dx += step;
    }
}

// ── price chart ────────────────────────────────────────────────

pub fn create_price_chart<'a>(
    stock_data: &'a StockData,
    analysis: &'a StockAnalysis,
    time_range: TimeRange,
    crosshair_x: Option<f64>,
    title: &'a str,
    canvas_char_width: u16,
) -> Canvas<'a, CanvasFn<'a>> {
    let bars = filter_bars(stock_data, time_range);
    let n = bars.len();
    let full_start = stock_data.closes.len().saturating_sub(n);

    let sma10_full = stock_data.sma(10).map(|a| a.to_vec()).unwrap_or_default();
    let sma50_full = stock_data.sma(50).map(|a| a.to_vec()).unwrap_or_default();
    let ema20_full = stock_data.ema(20).map(|a| a.to_vec()).unwrap_or_default();
    let sma10_pts = align_overlay(&sma10_full, full_start, n, 10);
    let sma50_pts = align_overlay(&sma50_full, full_start, n, 50);
    let ema20_pts = align_overlay(&ema20_full, full_start, n, 20);

    // Predictions
    let pred_pts: Vec<(f64, f64)> = analysis.predictions.iter().enumerate()
        .map(|(i, &p)| ((n as f64) + i as f64, p)).collect();
    let mut pred_full = vec![];
    if !pred_pts.is_empty() {
        if let Some(last) = bars.last() {
            pred_full.push((n as f64 - 1.0, last.close));
        }
        pred_full.extend(&pred_pts);
    }

    // Y range
    let mut all_y: Vec<f64> = bars.iter().flat_map(|b| [b.high, b.low]).collect();
    all_y.extend(sma10_pts.iter().map(|(_, y)| *y));
    all_y.extend(sma50_pts.iter().map(|(_, y)| *y));
    all_y.extend(ema20_pts.iter().map(|(_, y)| *y));
    all_y.extend(analysis.predictions.iter().copied());
    let y_max = all_y.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let y_min = all_y.iter().cloned().fold(f64::INFINITY, f64::min);
    let (y_lo, y_hi, _step) = nice_y_bounds(y_min, y_max);

    let x_max = if pred_pts.is_empty() {
        (n as f64 - 1.0).max(0.0)
    } else {
        n as f64 + pred_pts.len() as f64
    };

    Canvas::default()
        .block(Block::default().borders(Borders::ALL).title(title))
        .marker(Marker::Braille)
        .x_bounds([0.0, x_max.max(1.0)])
        .y_bounds([y_lo, y_hi])
        .paint(Box::new(move |ctx: &mut ratatui::widgets::canvas::Context<'_>| {
            if n == 0 { return; }

            // ── grid ──────────────────────────────────────
            for i in 0..=5 {
                let gy = y_lo + (y_hi - y_lo) * (i as f64 / 5.0);
                ctx.draw(&Line { x1: 0.0, y1: gy, x2: x_max, y2: gy, color: GRID_C });
                let gx = i as f64 / 5.0 * x_max;
                ctx.draw(&Line { x1: gx, y1: y_lo, x2: gx, y2: y_hi, color: GRID_C });
            }

            // ── SMA-50 ────────────────────────────────────
            if sma50_pts.len() > 1 {
                draw_series(ctx, &sma50_pts, SMA50_C);
            }
            // ── SMA-10 ────────────────────────────────────
            if sma10_pts.len() > 1 {
                draw_series(ctx, &sma10_pts, SMA10_C);
            }
            // ── EMA-20 ────────────────────────────────────
            if ema20_pts.len() > 1 {
                draw_series(ctx, &ema20_pts, EMA20_C);
            }

            // ── OHLC candles ──────────────────────────────
            let dot_x = x_max / (canvas_char_width as f64 * 2.0).max(1.0);
            let gap_x = if n > 1 { x_max / (n - 1) as f64 } else { 1.0 };
            for (i, bar) in bars.iter().enumerate() {
                draw_candle(ctx, i as f64, bar.open, bar.high, bar.low, bar.close, dot_x, gap_x);
            }

            // ── predictions ───────────────────────────────
            if pred_full.len() > 1 {
                let sep_x = n as f64 - 0.5;
                ctx.draw(&Line { x1: sep_x, y1: y_lo, x2: sep_x, y2: y_hi, color: GRID_C });
                draw_dashed(ctx, &pred_full, PRED_C, 0.3);
            }

            // ── crosshair ─────────────────────────────────
            if let Some(cx) = crosshair_x {
                ctx.draw(&Line { x1: cx, y1: y_lo, x2: cx, y2: y_hi, color: XHAIR_C });
            }
        }) as CanvasFn<'a>)
}

// ── volume chart (solid bars via HalfBlock + dense lines) ──────

pub fn create_volume_chart<'a>(
    stock_data: &'a StockData,
    time_range: TimeRange,
    canvas_char_width: u16,
) -> Canvas<'a, CanvasFn<'a>> {
    let bars = filter_bars(stock_data, time_range);
    let n = bars.len();
    let max_vol = bars.iter().map(|b| b.volume).max().unwrap_or(1);
    let x_max = (n as f64 - 1.0).max(1.0);

    Canvas::default()
        .block(Block::default().borders(Borders::ALL).title(" Volume "))
        .marker(Marker::HalfBlock)
        .x_bounds([0.0, x_max])
        .y_bounds([0.0, max_vol as f64 * 1.05])
        .paint(Box::new(move |ctx: &mut ratatui::widgets::canvas::Context<'_>| {
            let dot_x = x_max / (canvas_char_width as f64).max(1.0);
            let gap_x = if n > 1 { x_max / (n - 1) as f64 } else { 1.0 };
            for (i, bar) in bars.iter().enumerate() {
                draw_vol_bar(ctx, i as f64, bar.volume as f64, bar.close >= bar.open, dot_x, gap_x);
            }
        }) as CanvasFn<'a>)
}

// ── legend ─────────────────────────────────────────────────────

pub fn create_legend_line() -> Paragraph<'static> {
    let items: Vec<(&str, Color)> = vec![
        ("│ OHLC ", Color::White),
        ("─ SMA10 ", SMA10_C),
        ("─ SMA50 ", SMA50_C),
        ("─ EMA20 ", EMA20_C),
        ("╌ Pred ", PRED_C),
        ("│", Color::Reset),
        (" ▲ Vol ", VOL_UP),
        (" ▼ Vol ", VOL_DOWN),
    ];
    let spans: Vec<Span<'static>> = items.into_iter()
        .map(|(l, c)| Span::styled(l.to_string(), Style::default().fg(c)))
        .collect();
    Paragraph::new(TextLine::from(spans))
        .alignment(Alignment::Center)
        .style(Style::default().fg(Color::DarkGray))
}

// ── crosshair info ─────────────────────────────────────────────

pub struct CrosshairSnapshot {
    pub date: String,
    pub price: f64,
    pub sma10: Option<f64>,
    pub sma50: Option<f64>,
    pub ema20: Option<f64>,
    pub volume: u64,
    pub index: usize,
    pub total: usize,
}

pub fn crosshair_info(
    stock_data: &StockData,
    time_range: TimeRange,
    index: usize,
) -> Option<CrosshairSnapshot> {
    let bars = filter_bars(stock_data, time_range);
    let n = bars.len();
    if n == 0 || index >= n { return None; }
    let bar = &bars[index];
    let date = chrono::DateTime::from_timestamp(bar.timestamp, 0)
        .map(|dt| dt.format("%Y-%m-%d").to_string())
        .unwrap_or_else(|| "?".into());
    let full_start = stock_data.closes.len().saturating_sub(n);
    let full_idx = full_start + index;
    let sma10 = stock_data.sma(10).and_then(|a| a.to_vec().get(full_idx.saturating_sub(10)).copied());
    let sma50 = stock_data.sma(50).and_then(|a| a.to_vec().get(full_idx.saturating_sub(50)).copied());
    let ema20 = stock_data.ema(20).and_then(|a| a.to_vec().get(full_idx.saturating_sub(20)).copied());
    Some(CrosshairSnapshot { date, price: bar.close, sma10, sma50, ema20, volume: bar.volume, index, total: n })
}
