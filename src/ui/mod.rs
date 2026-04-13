pub mod theme;
pub mod utils;
pub mod components {
    pub mod header_footer;
    pub mod table;
}
pub mod modals;

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::Style,
    widgets::Block,
    Frame,
};

use crate::state::{AppMode, AppState};
pub use utils::{centered_rect, hit_test_table_row, hit_test_table_col};
use theme::*;
use components::header_footer::{draw_header, draw_footer};
use components::table::draw_table;
use modals::*;

pub fn draw(f: &mut Frame, state: &AppState) {
    let area = f.size();
    f.render_widget(Block::default().style(Style::default().bg(BG)), area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(0), Constraint::Length(3)])
        .split(area);

    draw_header(f, state, chunks[0]);
    draw_body(f, state, chunks[1]);
    draw_footer(f, state, chunks[2]);

    // Overlay modals
    match &state.mode {
        AppMode::Startup              => draw_startup(f),
        AppMode::ConfirmDelete { step } => draw_confirm_delete(f, state, *step),
        AppMode::Deleting { .. }     => draw_loading(f, state),
        AppMode::ManageDirs          => draw_dir_modal(f, state),
        AppMode::RunExe { prefix_idx, input } => draw_run_exe(f, state, *prefix_idx, input),
        AppMode::Help                => draw_help(f),
        AppMode::Error(msg)          => { let m = msg.clone(); draw_error(f, &m); }
        _                            => {}
    }
}

fn draw_body(f: &mut Frame, state: &AppState, area: Rect) {
    draw_table(f, state, area);
}
