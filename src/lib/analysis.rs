use crate::lib::stock_data::StockData;

pub struct StockAnalysis {
    pub symbol: String,
    pub current_price: f64,
    pub sma_10: Option<f64>,
    pub sma_50: Option<f64>,
    pub ema_20: Option<f64>,
    /// Full SMA-10 series (computed once at fetch time).
    pub sma10_values: Vec<f64>,
    /// Full SMA-50 series.
    pub sma50_values: Vec<f64>,
    /// Full EMA-20 series.
    pub ema20_values: Vec<f64>,
    pub predictions: Vec<f64>,
    pub recent_change: Option<f64>,
}

pub fn analyze_stock(stock_data: &StockData, symbol: &str) -> StockAnalysis {
    let current_price = stock_data.closes.last().copied().unwrap_or(0.0);

    let sma10_values = stock_data.sma(10).map(|a| a.to_vec()).unwrap_or_default();
    let sma50_values = stock_data.sma(50).map(|a| a.to_vec()).unwrap_or_default();
    let ema20_values = stock_data.ema(20).map(|a| a.to_vec()).unwrap_or_default();

    let sma_10 = sma10_values.last().copied();
    let sma_50 = sma50_values.last().copied();
    let ema_20 = ema20_values.last().copied();

    let predictions = stock_data.predict_next(20);

    let recent_change = if stock_data.len() >= 2 {
        let last = stock_data.closes.last().unwrap();
        let second_last = stock_data.closes[stock_data.len() - 2];
        Some((last - second_last) / second_last * 100.0)
    } else {
        None
    };

    StockAnalysis {
        symbol: symbol.to_string(),
        current_price,
        sma_10,
        sma_50,
        ema_20,
        sma10_values,
        sma50_values,
        ema20_values,
        predictions,
        recent_change,
    }
}
