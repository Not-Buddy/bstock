use crate::lib::stock_data::StockData;

#[derive(Clone, Copy, Debug, PartialEq)]
#[allow(dead_code)]
pub enum TimeRange {
    OneDay,
    OneWeek,
    OneMonth,
    ThreeMonths,
    SixMonths,
    YearToDate,
    OneYear,
    TwoYears,
    FiveYears,
    TenYears,
    All,
}

impl TimeRange {
    pub fn all() -> &'static [TimeRange] {
        &[
            TimeRange::OneDay,
            TimeRange::ThreeMonths,
            TimeRange::SixMonths,
            TimeRange::YearToDate,
            TimeRange::OneYear,
            TimeRange::TwoYears,
            TimeRange::FiveYears,
            TimeRange::TenYears,
            TimeRange::All,
        ]
    }

    pub fn as_str(&self) -> &str {
        match self {
            TimeRange::OneDay => "1D",
            TimeRange::OneWeek => "1W",
            TimeRange::OneMonth => "1M",
            TimeRange::ThreeMonths => "3M",
            TimeRange::SixMonths => "6M",
            TimeRange::YearToDate => "YTD",
            TimeRange::OneYear => "1Y",
            TimeRange::TwoYears => "2Y",
            TimeRange::FiveYears => "5Y",
            TimeRange::TenYears => "10Y",
            TimeRange::All => "All",
        }
    }

    /// Yahoo Finance v8 API (range, interval) pairs.
    pub fn yahoo_params(&self) -> (&'static str, &'static str) {
        match self {
            TimeRange::OneDay => ("1d", "1m"),
            TimeRange::OneWeek => ("5d", "5m"),
            TimeRange::OneMonth => ("1mo", "1h"),
            TimeRange::ThreeMonths => ("3mo", "1d"),
            TimeRange::SixMonths => ("6mo", "1d"),
            TimeRange::YearToDate => ("ytd", "1d"),
            TimeRange::OneYear => ("1y", "1d"),
            TimeRange::TwoYears => ("2y", "1d"),
            TimeRange::FiveYears => ("5y", "1wk"),
            TimeRange::TenYears => ("10y", "1mo"),
            TimeRange::All => ("max", "1mo"),
        }
    }

    /// Whether this is an intraday range (sub-hourly or sub-daily intervals).
    #[allow(dead_code)]
    pub fn is_intraday(&self) -> bool {
        matches!(self, TimeRange::OneDay | TimeRange::OneWeek | TimeRange::OneMonth)
    }
}

/// OHLC data for a single bar.
#[derive(Clone, Debug)]
pub struct FilteredBar {
    pub timestamp: i64,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: u64,
}

/// Return all bars — the API now provides the correct window via range/interval.
pub fn filter_bars(stock_data: &StockData, _time_range: TimeRange) -> Vec<FilteredBar> {
    let total = stock_data.closes.len();
    (0..total)
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
