use crate::data::TimeRange;
use crate::lib::error::AppError;
use crate::lib::stock_data::StockData;
use time::OffsetDateTime;
use yahoo_finance_api::YahooConnector;

/// Spoof a browser so Yahoo doesn't block us.
const USER_AGENT: &str =
    "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0.0.0 Safari/537.36";

fn days_for_range(tr: TimeRange) -> i64 {
    match tr {
        TimeRange::OneDay => 1,
        TimeRange::OneWeek => 7,
        TimeRange::OneMonth => 30,
        TimeRange::ThreeMonths => 90,
        TimeRange::SixMonths => 180,
        TimeRange::YearToDate => {
            let now = OffsetDateTime::now_utc();
            let start_of_year = now
                .replace_month(time::Month::January)
                .and_then(|d| d.replace_day(1))
                .unwrap_or(now);
            (now - start_of_year).whole_days().max(1)
        }
        TimeRange::OneYear => 365,
        TimeRange::TwoYears => 730,
        TimeRange::FiveYears => 1825,
        TimeRange::TenYears => 3650,
        TimeRange::All => 3650,
    }
}

pub async fn fetch_stock_data(symbol: &str, time_range: TimeRange) -> Result<StockData, AppError> {
    let provider = YahooConnector::builder()
        .build_with_agent(USER_AGENT)
        .map_err(|e| AppError::ApiError(format!("Failed to create connector: {e}")))?;

    let end = OffsetDateTime::now_utc();
    let days = days_for_range(time_range);
    let start = end - time::Duration::days(days);
    let (_, interval) = time_range.yahoo_params();

    let response = provider
        .get_quote_history_interval(symbol, start, end, interval)
        .await
        .map_err(|e| AppError::ApiError(format!(
            "HTTP request failed for {symbol} (range={days}d interval={interval}): {e}"
        )))?;

    let mut stock_data = StockData::new();
    let quotes = response
        .quotes()
        .map_err(|e| AppError::ApiError(format!("Failed to parse quotes for {symbol}: {e}")))?;

    if quotes.is_empty() {
        return Err(AppError::ApiError(format!(
            "No data returned for {symbol} (range={days}d interval={interval})"
        )));
    }

    for bar in quotes {
        stock_data.add_point(
            bar.timestamp as i64, bar.open, bar.high, bar.low, bar.close, bar.volume,
        );
    }

    Ok(stock_data)
}
