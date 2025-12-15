use stock_predictor_lib::{
    analysis::StockAnalysis,
    stock_data::StockData,
};
use crate::data::TimeRange;

pub enum AppEvent {
    Update(StockAnalysis, StockData, TimeRange),
    Error(String),
}
