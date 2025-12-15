use ratatui::{
    prelude::*,
    widgets::{Block, Paragraph},
};
use crate::data::TimeRange;

// Function to render the time range selector
pub fn render_time_range_selector(current_time_range: TimeRange, is_selected: bool) -> Paragraph<'static> {
    let mut text = String::new();
    let time_ranges = TimeRange::all();

    for (i, tr) in time_ranges.iter().enumerate() {
        if i > 0 {
            text.push(' ');
        }

        if *tr == current_time_range {
            if is_selected {
                text.push_str(&format!(" [{}] ", tr.as_str()));
            } else {
                text.push_str(&format!(" ({}) ", tr.as_str()));
            }
        } else {
            text.push_str(&format!(" {} ", tr.as_str()));
        }
    }

    Paragraph::new(text)
        .alignment(Alignment::Center)
        .block(Block::default())
}
