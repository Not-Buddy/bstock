use stock_predictor_lib::{config::read_config, yahooapi::fetch_stock_data};

#[tokio::test]
async fn test_fetch_and_analyze() {
    let config = read_config("stocks_config.json").unwrap();
    let symbol = &config.symbols[0];

    let result = fetch_stock_data(symbol, config.analysis_period_days).await;
    assert!(result.is_ok());

    let stock_data = result.unwrap();
    assert!(stock_data.len() > 0);
}
