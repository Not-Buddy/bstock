use anyhow::Result;
use crossterm::event::{self, Event};
use ratatui::prelude::*;
use std::io;
use std::time::Duration;

use crate::lib::config::StockConfig;
use crate::ui::{detail::draw_detail_ui, layout::draw_ui};

use super::state::{App, View};

impl App {
    pub fn run(
        &mut self,
        terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
        config: &StockConfig,
        config_file_path: &str,
    ) -> Result<()> {
        self.config_file_path = config_file_path.to_string();
        self.initialize_placeholders(config);

        loop {
            self.check_refresh();
            self.drain_events();

            // ── render ───────────────────────────────────────
            match self.current_view {
                View::Main => {
                    terminal.draw(|f| draw_ui(
                        f,
                        &self.analyses,
                        self.selected_index,
                        self.loading_total,
                        self.loading_done,
                        &self.loading_errors,
                    ))?;
                }
                View::Detail => {
                    terminal.draw(|f| {
                        if let Some(data) = self.analyses.get(self.selected_index) {
                            draw_detail_ui(
                                f, data, f.size(), self.crosshair_index,
                                self.loading_total, self.loading_done,
                            );
                        }
                    })?;
                }
                View::Edit => {
                    terminal.draw(|f| {
                        crate::ui::edit::draw_edit_ui(f, self, f.size());
                    })?;
                }
            }

            // ── input ────────────────────────────────────────
            if event::poll(Duration::from_millis(100))?
                && let Event::Key(key) = event::read()?
            {
                let code = key.code;
                let mods = key.modifiers;

                let quit = match self.current_view {
                    View::Main => self.handle_main_key(code, mods),
                    View::Detail => self.handle_detail_key(code, mods),
                    View::Edit => {
                        self.handle_edit_key(code, mods);
                        None
                    }
                };

                if quit.is_some() {
                    return Ok(());
                }
            }
        }
    }
}
