use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("Failed to read config file")]
    ConfigReadError(#[from] std::io::Error),

    #[error("Failed to parse config file")]
    ConfigParseError(#[from] serde_json::Error),

    #[error("Failed to fetch stock data")]
    ApiError(#[from] yahoo_finance_api::YahooError),
}
