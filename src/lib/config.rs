use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct StockConfig {
    pub symbols: Vec<String>,
    pub analysis_period_days: i64,
}

