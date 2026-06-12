use crate::lib::error::AppError;
use crate::lib::stock_data::StockData;
use time::OffsetDateTime;
use yahoo_finance_api::YahooConnector;

pub async fn fetch_stock_data(symbol: &str, period_days: i64) -> Result<StockData, AppError> {
    let provider = YahooConnector::new()
        .map_err(AppError::ApiError)?;

    let end = OffsetDateTime::now_utc();
    let start = end - time::Duration::days(period_days);

    let response = provider.get_quote_history(symbol, start, end)
        .await
        .map_err(AppError::ApiError)?;

    let mut stock_data = StockData::new();

    let quotes = response.quotes()
        .map_err(AppError::ApiError)?;

    for bar in quotes {
        stock_data.add_point(
            bar.timestamp as i64,
            bar.open,
            bar.high,
            bar.low,
            bar.close,
            bar.volume,
        );
    }

    Ok(stock_data)
}
