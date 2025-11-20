use ndarray::{Array1};
use std::error::Error;
use time::OffsetDateTime;
use yahoo_finance_api::YahooConnector;

#[derive(Debug)]
struct StockData {
    timestamps: Vec<i64>,
    closes: Vec<f64>,
    volumes: Vec<u64>,
}

impl StockData {
    fn new() -> Self {
        StockData {
            timestamps: Vec::new(),
            closes: Vec::new(),
            volumes: Vec::new(),
        }
    }

    fn add_point(&mut self, timestamp: i64, close: f64, volume: u64) {
        self.timestamps.push(timestamp);
        self.closes.push(close);
        self.volumes.push(volume);
    }

    fn len(&self) -> usize {
        self.closes.len()
    }

    // Calculate Simple Moving Average
    fn sma(&self, period: usize) -> Option<Array1<f64>> {
        if self.len() < period {
            return None;
        }

        let mut sma_values = Vec::new();
        for i in period..self.len() {
            let sum: f64 = self.closes[i - period..i].iter().sum();
            sma_values.push(sum / period as f64);
        }

        Some(Array1::from(sma_values))
    }

    // Calculate Exponential Moving Average
    fn ema(&self, period: usize) -> Option<Array1<f64>> {
        if self.len() < period {
            return None;
        }

        let mut ema_values = Vec::new();
        let multiplier = 2.0 / (period as f64 + 1.0);
        
        // First EMA is SMA of first period
        let initial_sma: f64 = self.closes[0..period].iter().sum::<f64>() / period as f64;
        ema_values.push(initial_sma);

        for i in period..self.len() {
            let ema = (self.closes[i] - ema_values.last().unwrap()) * multiplier 
                     + ema_values.last().unwrap();
            ema_values.push(ema);
        }

        Some(Array1::from(ema_values))
    }

    // Simple prediction based on trend
    fn predict_next(&self, periods: usize) -> Vec<f64> {
        if self.len() < 2 {
            return vec![];
        }

        // Calculate recent trend
        let recent_period = periods.min(self.len());
        let mut predictions = Vec::new();
        
        // Use linear regression on recent data
        let n = recent_period as f64;
        let x: Vec<f64> = (0..recent_period).map(|i| i as f64).collect();
        let y = &self.closes[self.len() - recent_period..];
        
        // Calculate slope and intercept
        let sum_x: f64 = x.iter().sum();
        let sum_y: f64 = y.iter().sum();
        let sum_xy: f64 = x.iter().zip(y.iter()).map(|(xi, yi)| xi * yi).sum();
        let sum_x2: f64 = x.iter().map(|xi| xi * xi).sum();
        
        let slope = (n * sum_xy - sum_x * sum_y) / (n * sum_x2 - sum_x * sum_x);
        let intercept = (sum_y - slope * sum_x) / n;
        
        // Predict next values
        for i in 1..=3 {
            let next_x = (recent_period + i) as f64;
            predictions.push(slope * next_x + intercept);
        }
        
        predictions
    }
}

async fn fetch_stock_data(symbol: &str, period_days: i64) -> Result<StockData, Box<dyn Error>> {
    let provider = YahooConnector::new()?;
    
    let end = OffsetDateTime::now_utc();
    let start = end - time::Duration::days(period_days);
    
    let response = provider
        .get_quote_history(symbol, start, end)
        .await?;
    
    let mut stock_data = StockData::new();
    
    let quotes = response.quotes()?;
    for bar in quotes {
        // FIX: Convert u64 timestamp to i64
        stock_data.add_point(
            bar.timestamp as i64,  // Cast from u64 to i64
            bar.close,
            bar.volume,
        );
    }
    
    Ok(stock_data)
}


fn analyze_stock(stock_data: &StockData) {
    println!("📊 Stock Analysis");
    println!("=================");
    println!("Total data points: {}", stock_data.len());
    
    if let Some(current_price) = stock_data.closes.last() {
        println!("Current price: ${:.2}", current_price);
    }
    
    // Calculate moving averages
    if let Some(sma_10) = stock_data.sma(10) {
        if let Some(current_sma) = sma_10.last() {
            println!("10-day SMA: ${:.2}", current_sma);
        }
    }
    
    if let Some(sma_50) = stock_data.sma(50) {
        if let Some(current_sma) = sma_50.last() {
            println!("50-day SMA: ${:.2}", current_sma);
        }
    }
    
    if let Some(ema_20) = stock_data.ema(20) {
        if let Some(current_ema) = ema_20.last() {
            println!("20-day EMA: ${:.2}", current_ema);
        }
    }
    
    // Make predictions
    println!("\n🔮 Predictions (next 3 days):");
    let predictions = stock_data.predict_next(20);
    for (i, price) in predictions.iter().enumerate() {
        println!("Day {}: ${:.2}", i + 1, price);
    }
    
    // Calculate trend
    if stock_data.len() >= 2 {
        let recent_change = (stock_data.closes.last().unwrap() - stock_data.closes[stock_data.len() - 2]) 
                          / stock_data.closes[stock_data.len() - 2] * 100.0;
        println!("\n📈 Recent trend: {:.2}%", recent_change);
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    println!("🚀 Stock Predictor in Rust");
    println!("==========================\n");
    
    let symbol = "AAPL";
    println!("Fetching data for: {}\n", symbol);
    
    // Fetch last 90 days of data
    let stock_data = fetch_stock_data(symbol, 90).await?;
    
    if stock_data.len() > 0 {
        analyze_stock(&stock_data);
    } else {
        println!("No data found for symbol: {}", symbol);
    }
    
    Ok(())
}

