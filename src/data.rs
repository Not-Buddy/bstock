use crate::lib::stock_data::StockData;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum TimeRange {
    OneDay = 1,
    FiveDays = 5,
    OneMonth = 30,
    SixMonths = 180,
}

impl TimeRange {
    pub fn all() -> Vec<TimeRange> {
        vec![
            TimeRange::OneDay,
            TimeRange::FiveDays,
            TimeRange::OneMonth,
            TimeRange::SixMonths,
        ]
    }

    pub fn as_str(&self) -> &str {
        match self {
            TimeRange::OneDay => "1W",
            TimeRange::FiveDays => "2W",
            TimeRange::OneMonth => "1M",
            TimeRange::SixMonths => "6M",
        }
    }
}

/// OHLC data for a single bar, filtered to a time window.
#[derive(Clone, Debug)]
pub struct FilteredBar {
    pub timestamp: i64,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: u64,
}

/// Filter stock data to the last N bars for the given time range.
pub fn filter_bars(stock_data: &StockData, time_range: TimeRange) -> Vec<FilteredBar> {
    let total = stock_data.closes.len();
    let n = match time_range {
        TimeRange::OneDay => 5usize.min(total),     // ~1 week
        TimeRange::FiveDays => 10usize.min(total),  // ~2 weeks
        TimeRange::OneMonth => 30usize.min(total),
        TimeRange::SixMonths => 180usize.min(total),
    };
    let start = total.saturating_sub(n);

    (start..total)
        .map(|i| FilteredBar {
            timestamp: stock_data.timestamps[i],
            open: stock_data.opens[i],
            high: stock_data.highs[i],
            low: stock_data.lows[i],
            close: stock_data.closes[i],
            volume: stock_data.volumes[i],
        })
        .collect()
}

/// Legacy — returns just closing prices for a time window.
pub fn filter_data_by_time_range(stock_data: &StockData, time_range: TimeRange) -> Vec<f64> {
    let total_points = stock_data.closes.len();
    let points_to_show = match time_range {
        TimeRange::OneDay => std::cmp::min(5, total_points),    // ~1 week
        TimeRange::FiveDays => std::cmp::min(10, total_points), // ~2 weeks
        TimeRange::OneMonth => std::cmp::min(30, total_points),
        TimeRange::SixMonths => std::cmp::min(180, total_points),
    };
    let start_index = total_points.saturating_sub(points_to_show);
    stock_data.closes[start_index..].to_vec()
}

/// Calculate volatility (standard deviation of returns).
pub fn calculate_volatility(prices: &[f64]) -> f64 {
    if prices.len() < 2 {
        return 0.0;
    }
    let returns: Vec<f64> = prices
        .windows(2)
        .map(|w| if w[0] != 0.0 { (w[1] - w[0]) / w[0] } else { 0.0 })
        .collect();
    if returns.is_empty() {
        return 0.0;
    }
    let mean_return = returns.iter().sum::<f64>() / returns.len() as f64;
    let variance = returns
        .iter()
        .map(|r| (r - mean_return).powi(2))
        .sum::<f64>()
        / returns.len() as f64;
    variance.sqrt() * 100.0
}
