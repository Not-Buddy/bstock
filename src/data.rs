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
            TimeRange::OneDay => "1D",
            TimeRange::FiveDays => "5D",
            TimeRange::OneMonth => "1M",
            TimeRange::SixMonths => "6M",
        }
    }
}

// Function to filter stock data based on selected time range
pub fn filter_data_by_time_range(stock_data: &StockData, time_range: TimeRange) -> Vec<f64> {
    // Since we don't have the exact timestamp for each close value, we'll take the last N values
    // where N corresponds to the time range (approximate)
    let total_points = stock_data.closes.len();
    let points_to_show = match time_range {
        TimeRange::OneDay => std::cmp::min(2, total_points),     // Last day (at least 2 points)
        TimeRange::FiveDays => std::cmp::min(5, total_points),   // Last 5 days
        TimeRange::OneMonth => std::cmp::min(30, total_points),  // Last 30 days
        TimeRange::SixMonths => std::cmp::min(180, total_points), // Last ~6 months
    };

    // Take the last N points based on the time range
    let start_index = total_points.saturating_sub(points_to_show);

    stock_data.closes[start_index..].to_vec()
}

// Helper function to calculate volatility (standard deviation of returns)
pub fn calculate_volatility(prices: &[f64]) -> f64 {
    if prices.len() < 2 {
        return 0.0;
    }

    // Calculate daily returns
    let returns: Vec<f64> = prices.windows(2)
        .map(|w| if w[0] != 0.0 { (w[1] - w[0]) / w[0] } else { 0.0 })
        .collect();

    if returns.is_empty() {
        return 0.0;
    }

    // Calculate mean return
    let mean_return = returns.iter().sum::<f64>() / returns.len() as f64;

    // Calculate variance
    let variance = returns.iter()
        .map(|r| (r - mean_return).powi(2))
        .sum::<f64>() / returns.len() as f64;

    // Standard deviation (volatility) as percentage
    variance.sqrt() * 100.0
}
