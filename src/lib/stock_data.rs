use ndarray::Array1;

#[derive(Debug)]
pub struct StockData {
    pub timestamps: Vec<i64>,
    pub opens: Vec<f64>,
    pub highs: Vec<f64>,
    pub lows: Vec<f64>,
    pub closes: Vec<f64>,
    pub volumes: Vec<u64>,
}

impl Default for StockData {
    fn default() -> Self {
        Self::new()
    }
}

impl StockData {
    pub fn new() -> Self {
        StockData {
            timestamps: Vec::new(),
            opens: Vec::new(),
            highs: Vec::new(),
            lows: Vec::new(),
            closes: Vec::new(),
            volumes: Vec::new(),
        }
    }

    pub fn add_point(&mut self, timestamp: i64, open: f64, high: f64, low: f64, close: f64, volume: u64) {
        self.timestamps.push(timestamp);
        self.opens.push(open);
        self.highs.push(high);
        self.lows.push(low);
        self.closes.push(close);
        self.volumes.push(volume);
    }

    pub fn len(&self) -> usize {
        self.closes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.closes.is_empty()
    }

    // Calculate Simple Moving Average
    pub fn sma(&self, period: usize) -> Option<Array1<f64>> {
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
    pub fn ema(&self, period: usize) -> Option<Array1<f64>> {
        if self.len() < period {
            return None;
        }

        let mut ema_values = Vec::new();
        let multiplier = 2.0 / (period as f64 + 1.0);

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
    pub fn predict_next(&self, periods: usize) -> Vec<f64> {
        if self.len() < 2 {
            return vec![];
        }

        let recent_period = periods.min(self.len());
        let mut predictions = Vec::new();

        let n = recent_period as f64;
        let x: Vec<f64> = (0..recent_period).map(|i| i as f64).collect();
        let y = &self.closes[self.len() - recent_period..];

        let sum_x: f64 = x.iter().sum();
        let sum_y: f64 = y.iter().sum();
        let sum_xy: f64 = x.iter().zip(y.iter()).map(|(xi, yi)| xi * yi).sum();
        let sum_x2: f64 = x.iter().map(|xi| xi * xi).sum();

        let slope = (n * sum_xy - sum_x * sum_y) / (n * sum_x2 - sum_x * sum_x);
        let intercept = (sum_y - slope * sum_x) / n;

        for i in 1..=5 {
            let next_x = (recent_period + i) as f64;
            predictions.push(slope * next_x + intercept);
        }

        predictions
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ndarray::arr1;
    use approx::assert_abs_diff_eq;

    fn create_stock_data() -> StockData {
        let mut sd = StockData::new();
        sd.add_point(1672531200, 99.0, 102.0, 98.0, 100.0, 1000);
        sd.add_point(1672617600, 101.0, 104.0, 100.0, 102.0, 1200);
        sd.add_point(1672704000, 102.0, 106.0, 101.0, 105.0, 1100);
        sd.add_point(1672790400, 104.0, 105.0, 101.0, 103.0, 1300);
        sd.add_point(1672876800, 103.0, 108.0, 102.0, 106.0, 1400);
        sd.add_point(1672963200, 105.0, 110.0, 104.0, 108.0, 1500);
        sd
    }

    #[test]
    fn test_sma() {
        let sd = create_stock_data();
        let sma = sd.sma(3).unwrap();
        let expected = arr1(&[102.33333333333333, 103.33333333333333, 104.66666666666667]);
        assert_abs_diff_eq!(sma, expected, epsilon = 1e-10);
    }

    #[test]
    fn test_ema() {
        let sd = create_stock_data();
        let ema = sd.ema(3).unwrap();
        let expected = arr1(&[102.33333333333333, 102.66666666666666, 104.33333333333333, 106.16666666666666]);
        assert_abs_diff_eq!(ema, expected, epsilon = 1e-10);
    }

    #[test]
    fn test_predict_next() {
        let sd = create_stock_data();
        let predictions = sd.predict_next(5);
        assert_eq!(predictions.len(), 5);
        assert!(predictions[0] > 100.0);
    }
}
