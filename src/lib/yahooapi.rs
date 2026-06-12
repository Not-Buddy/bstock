use crate::data::TimeRange;
use crate::lib::error::AppError;
use crate::lib::stock_data::StockData;
use yahoo_finance_api::YahooConnector;

const USER_AGENT: &str =
    "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0.0.0 Safari/537.36";

pub async fn fetch_stock_data(symbol: &str, time_range: TimeRange) -> Result<StockData, AppError> {
    let provider = YahooConnector::builder()
        .build_with_agent(USER_AGENT)
        .map_err(|e| AppError::ApiError(format!("Connector: {e}")))?;

    let (range, interval) = time_range.yahoo_params();

    let response = provider
        .get_quote_range(symbol, interval, range)
        .await
        .map_err(|e| AppError::ApiError(format!(
            "{symbol} (range={range} interval={interval}): {e}"
        )))?;

    let mut stock_data = StockData::new();
    let quotes = response
        .quotes()
        .map_err(|e| AppError::ApiError(format!("Parse {symbol}: {e}")))?;

    if quotes.is_empty() {
        return Err(AppError::ApiError(format!(
            "{symbol}: no data (range={range} interval={interval})"
        )));
    }

    for bar in quotes {
        stock_data.add_point(
            bar.timestamp as i64, bar.open, bar.high, bar.low, bar.close, bar.volume,
        );
    }

    Ok(stock_data)
}
