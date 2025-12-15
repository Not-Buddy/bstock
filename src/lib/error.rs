use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("Failed to read config file")]
    ConfigReadError(std::io::Error),

    #[error("Failed to parse config file")]
    ConfigParseError(serde_json::Error),

    #[error("Failed to fetch stock data")]
    ApiError(yahoo_finance_api::YahooError),

    #[error("IO Error")]
    Io(std::io::Error),
}
