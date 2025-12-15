use crate::error::AppError;
use serde::{Deserialize, Serialize};
use std::fs;

#[derive(Debug, Serialize, Deserialize)]
pub struct StockConfig {
    pub symbols: Vec<String>,
    pub analysis_period_days: i64,
}

pub fn read_config(file_path: &str) -> Result<StockConfig, AppError> {
    let config_content = fs::read_to_string(file_path)?;
    let config: StockConfig = serde_json::from_str(&config_content)?;
    Ok(config)
}

pub fn write_config(config: &StockConfig, file_path: &str) -> Result<(), AppError> {
    let config_content = serde_json::to_string_pretty(config)?;
    fs::write(file_path, config_content)?;
    Ok(())
}
