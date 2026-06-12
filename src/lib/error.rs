use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Failed to parse config file")]
    ConfigParseError(#[from] serde_json::Error),

    #[error("Yahoo API error: {0}")]
    ApiError(String),
}

impl From<yahoo_finance_api::YahooError> for AppError {
    fn from(e: yahoo_finance_api::YahooError) -> Self {
        AppError::ApiError(e.to_string())
    }
}
