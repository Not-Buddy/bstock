use crate::lib::{config::StockConfig, error::AppError};
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize)]
pub struct AppConfig {
    pub stock_config: StockConfig,
    pub last_updated: Option<u64>, // Unix timestamp
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            stock_config: StockConfig {
                symbols: vec![
                    "PLTR".to_string(),
                    "NBIS".to_string(),
                    "GOOGL".to_string(),
                    "NVDA".to_string(),
                    "MSFT".to_string(),
                    "TSLA".to_string(),
                    "SLDP".to_string(),
                    "IREN".to_string(),
                ],
                analysis_period_days: 90,
            },
            last_updated: None,
        }
    }
}


pub struct PersistenceManager {
    config_dir: PathBuf,
    config_file: PathBuf,
}

impl PersistenceManager {
    pub fn new() -> Result<Self, AppError> {
        // Use ProjectDirs to get the appropriate config directory for the OS
        let project_dirs = ProjectDirs::from("com", "bstock", "bstock")
            .ok_or_else(|| AppError::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "Could not determine project directories"
            )))?;

        let config_dir = project_dirs.config_dir().to_path_buf();
        let config_file = config_dir.join("config.json");

        // Create config directory if it doesn't exist
        fs::create_dir_all(&config_dir)
            .map_err(AppError::Io)?;

        Ok(PersistenceManager {
            config_dir,
            config_file,
        })
    }

    pub fn load_config(&self) -> Result<AppConfig, AppError> {
        if self.config_file.exists() {
            let config_content = fs::read_to_string(&self.config_file)
                .map_err(AppError::ConfigReadError)?;
            let app_config: AppConfig = serde_json::from_str(&config_content)
                .map_err(AppError::ConfigParseError)?;
            Ok(app_config)
        } else {
            // Return default config if file doesn't exist
            Ok(AppConfig::default())
        }
    }

    pub fn save_config(&self, config: &AppConfig) -> Result<(), AppError> {
        let config_content = serde_json::to_string_pretty(config)
            .map_err(AppError::ConfigParseError)?;
        fs::write(&self.config_file, config_content)
            .map_err(AppError::ConfigReadError)?;
        Ok(())
    }

    pub fn save_stock_config(&self, stock_config: &StockConfig) -> Result<(), AppError> {
        let new_config = AppConfig {
            stock_config: stock_config.clone(),
            last_updated: Some(std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs()),
        };
        self.save_config(&new_config)
    }

    pub fn get_stock_config(&self) -> Result<StockConfig, AppError> {
        let config = self.load_config().unwrap_or_else(|_| AppConfig::default());
        Ok(config.stock_config)
    }
}