use ratatui::{
    prelude::*,
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, BorderType},
};

use crate::app::App;

/// Renders the user interface for the edit view where users can add/remove stocks
pub fn draw_edit_ui(f: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),        // Title
            Constraint::Length(3),        // New symbol input
            Constraint::Min(10),          // Stock list
            Constraint::Length(3),        // Instructions
        ])
        .split(area);

    // Title
    let title_block = Block::default()
        .borders(Borders::BOTTOM)
        .border_type(BorderType::Plain);
    f.render_widget(title_block, chunks[0]);
    
    let title = Paragraph::new("Edit Stocks - Add or Remove Symbols")
        .style(Style::default().fg(Color::Yellow))
        .alignment(Alignment::Center);
    f.render_widget(title, chunks[0]);

    // Input field for new symbols
    let input_block = Block::default()
        .borders(Borders::ALL)
        .title("Add New Symbol (Press Enter to add)");
    let input_text = Paragraph::new(app.new_symbol_input.as_str())
        .block(input_block);
    f.render_widget(input_text, chunks[1]);

    // Stock list with selection
    let mut list_state = ListState::default();
    list_state.select(Some(app.editing_selected_index));
    
    // Create list items
    let items: Vec<ListItem> = app.editing_symbols
        .iter()
        .enumerate()
        .map(|(i, symbol)| {
            let content = if i == app.editing_selected_index {
                // Highlight selected item
                Line::from(vec![
                    Span::styled(">", Style::default().fg(Color::Yellow)),
                    Span::raw(format!(" {}", symbol)),
                ])
            } else {
                Line::from(format!("  {}", symbol))
            };
            ListItem::new(content)
        })
        .collect();

    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title("Current Symbols (Delete to remove)"))
        .highlight_style(Style::default().bg(Color::DarkGray));
    
    f.render_stateful_widget(list, chunks[2], &mut list_state);

    // Instructions
    let instructions = Paragraph::new(
        "Up/Down: Navigate | Delete: Remove selected | Enter: Add new symbol | Ctrl+S: Save & Exit | Esc: Cancel"
    )
    .style(Style::default().fg(Color::Gray))
    .alignment(Alignment::Center);
    f.render_widget(instructions, chunks[3]);
}