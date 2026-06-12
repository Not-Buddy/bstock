use crossterm::event::{KeyCode, KeyModifiers};

use crate::lib::config::StockConfig;

use super::state::{App, View};

impl App {
    // ── main view ──────────────────────────────────────────────

    pub(super) fn handle_main_key(&mut self, code: KeyCode, modifiers: KeyModifiers) -> Option<()> {
        match code {
            KeyCode::Char('q') => return Some(()),
            KeyCode::Char('c') if modifiers == KeyModifiers::CONTROL => return Some(()),
            KeyCode::Left => {
                if self.selected_index > 0 {
                    self.selected_index -= 1;
                }
            }
            KeyCode::Right => {
                if !self.analyses.is_empty()
                    && self.selected_index < self.analyses.len() - 1
                {
                    self.selected_index += 1;
                }
            }
            KeyCode::Up => self.cycle_time_range(-1),
            KeyCode::Down => self.cycle_time_range(1),
            KeyCode::Enter => {
                self.crosshair_index = None;
                // Lazy-load: fetch data for this stock on first entry
                if self.analyses.get(self.selected_index)
                    .is_none_or(|a| a.stock_data.is_empty())
                {
                    let tr = self.analyses[self.selected_index].time_range;
                    self.fetch_single_stock(self.selected_index, tr);
                }
                self.current_view = View::Detail;
            }
            KeyCode::Esc => return Some(()),
            KeyCode::Char('e') => self.enter_edit_mode(),
            _ => {}
        }
        None
    }

    // ── detail view ────────────────────────────────────────────

    pub(super) fn handle_detail_key(&mut self, code: KeyCode, modifiers: KeyModifiers) -> Option<()> {
        match code {
            KeyCode::Char('q') => return Some(()),
            KeyCode::Char('c') if modifiers == KeyModifiers::CONTROL => return Some(()),

            KeyCode::Left => {
                let n = self.visible_bar_count();
                if n > 0 {
                    let idx = self.crosshair_index.unwrap_or(n / 2);
                    self.crosshair_index = Some(if idx > 0 { idx - 1 } else { 0 });
                }
            }
            KeyCode::Right => {
                let n = self.visible_bar_count();
                if n > 0 {
                    let idx = self.crosshair_index.unwrap_or(n / 2);
                    self.crosshair_index = Some(
                        if idx + 1 < n { idx + 1 } else { n.saturating_sub(1) },
                    );
                }
            }
            KeyCode::Up => {
                self.crosshair_index = None;
                self.cycle_time_range(-1);
            }
            KeyCode::Down => {
                self.crosshair_index = None;
                self.cycle_time_range(1);
            }
            KeyCode::Esc => {
                if self.crosshair_index.is_some() {
                    self.crosshair_index = None;
                } else {
                    self.current_view = View::Main;
                }
            }
            KeyCode::Enter => {
                self.crosshair_index = None;
                self.current_view = View::Main;
            }
            _ => {}
        }
        None
    }

    // ── edit view ──────────────────────────────────────────────

    fn enter_edit_mode(&mut self) {
        self.current_view = View::Edit;
        self.editing_symbols = self
            .analyses
            .iter()
            .map(|a| a.analysis.symbol.clone())
            .collect();
        self.editing_selected_index = 0;
        self.new_symbol_input = String::new();
    }

    pub(super) fn handle_edit_key(&mut self, code: KeyCode, modifiers: KeyModifiers) {
        match code {
            KeyCode::Esc => self.current_view = View::Main,

            KeyCode::Enter => {
                if !self.new_symbol_input.trim().is_empty() {
                    let sym = self.new_symbol_input.trim().to_uppercase();
                    if !self.editing_symbols.contains(&sym) {
                        self.editing_symbols.push(sym);
                    }
                    self.new_symbol_input.clear();
                }
            }

            KeyCode::Char(c) => {
                if c == 's' && modifiers.contains(KeyModifiers::CONTROL) {
                    let config = StockConfig {
                        symbols: self.editing_symbols.clone(),
                        analysis_period_days: 90,
                    };
                    if let Err(e) = self.persistence_manager.save_stock_config(&config) {
                        eprintln!("Error saving config: {}", e);
                    } else {
                        self.current_view = View::Main;
                        self.refresh_analyses(&config);
                    }
                } else {
                    self.new_symbol_input.push(c);
                }
            }

            KeyCode::Backspace => {
                self.new_symbol_input.pop();
            }

            KeyCode::Delete => {
                if !self.editing_symbols.is_empty()
                    && self.editing_selected_index < self.editing_symbols.len()
                {
                    self.editing_symbols.remove(self.editing_selected_index);
                    if self.editing_selected_index > 0 {
                        self.editing_selected_index -= 1;
                    }
                }
            }

            KeyCode::Up => {
                if self.editing_selected_index > 0 {
                    self.editing_selected_index -= 1;
                }
            }

            KeyCode::Down => {
                if !self.editing_symbols.is_empty()
                    && self.editing_selected_index < self.editing_symbols.len() - 1
                {
                    self.editing_selected_index += 1;
                }
            }

            _ => {}
        }
    }
}
