use ratatui::{
    layout::{Constraint, Margin, Rect},
    style::{Modifier, Style, Color},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Row, Scrollbar, ScrollbarOrientation, ScrollbarState, Table},
    Frame,
};
use crate::state::AppState;
use crate::{FilterMode, SortColumn};
use crate::ui::theme::*;

const COL_WIDTHS_ALL: [Constraint; 5] = [
    Constraint::Min(28),
    Constraint::Length(10),
    Constraint::Length(10),
    Constraint::Length(7),
    Constraint::Length(11),
];
const COL_WIDTHS_UNINSTALLED: [Constraint; 4] = [
    Constraint::Min(28),
    Constraint::Length(10),
    Constraint::Length(10),
    Constraint::Length(7),
];

fn col_header_span(label: &str, col: SortColumn, state: &AppState) -> Span<'static> {
    let active = state.sort_col == col;
    let indicator = if active { if state.sort_asc() { " ▲" } else { " ▼" } } else { "" };
    let text = format!("{}{}", label, indicator);
    if active {
        Span::styled(text, Style::default().fg(Color::White).add_modifier(Modifier::BOLD))
    } else {
        Span::styled(text, Style::default().fg(ACCENT))
    }
}

pub fn draw_table(f: &mut Frame, state: &AppState, area: Rect) {
    let visible_height = area.height.saturating_sub(1) as usize;
    let show_installed = state.filter_mode == FilterMode::All;

    let mut header_cells = vec![
        Cell::from(Line::from(vec![Span::raw("  "), col_header_span("Game Name", SortColumn::Name,  state)])),
        Cell::from(Line::from(vec![col_header_span("App ID", SortColumn::AppId, state)])),
        Cell::from(Line::from(vec![col_header_span("Size",   SortColumn::Size,  state)])),
    ];
    header_cells.push(Cell::from(Line::from(vec![col_header_span("Cloud", SortColumn::Cloud, state)])));
    if show_installed {
        header_cells.push(Cell::from(Line::from(vec![col_header_span("Installed", SortColumn::Installed, state)])));
    }
    let header = Row::new(header_cells).style(Style::default().bg(BG3)).height(1);

    let rows: Vec<Row> = state.filtered_indices.iter().enumerate()
        .skip(state.scroll_offset)
        .take(visible_height)
        .map(|(disp_idx, &real_idx)| {
            let p = &state.prefixes[real_idx];
            let is_cursor = disp_idx == state.selected;
            let in_selection = !state.selection.is_empty() && state.selection.contains(&real_idx);
            let unknown = p.game.is_none();

            let row_bg = if is_cursor { SEL }
                         else if in_selection { MSEL }
                         else if disp_idx % 2 == 0 { BG } else { BG2 };

            let name_style = if is_cursor || in_selection {
                Style::default().fg(Color::White).add_modifier(Modifier::BOLD)
            } else if unknown {
                Style::default().fg(DIM)
            } else {
                Style::default().fg(FG)
            };

            let sel_marker = if is_cursor { "▶ " } else { "  " };

            let cloud_cell = if unknown {
                Cell::from("─").style(Style::default().fg(DIM))
            } else if p.has_cloud_saves() {
                Cell::from("☁").style(Style::default().fg(ACCENT))
            } else {
                Cell::from("✗").style(Style::default().fg(WARN))
            };

            let mut cells = vec![
                Cell::from(format!("{}{}", sel_marker, p.game_name())).style(name_style),
                Cell::from(p.app_id.to_string()).style(Style::default().fg(DIM)),
                Cell::from(p.size_human()).style(Style::default().fg(
                    if p.size_bytes > 1_073_741_824 { WARN } else { FG }
                )),
            ];
            cells.push(cloud_cell);
            if show_installed {
                let inst_cell = if unknown {
                    Cell::from("─").style(Style::default().fg(DIM))
                } else if p.is_installed() {
                    Cell::from("✓").style(Style::default().fg(OK))
                } else {
                    Cell::from("✗").style(Style::default().fg(DANGER))
                };
                cells.push(inst_cell);
            }

            Row::new(cells).style(Style::default().bg(row_bg)).height(1)
        })
        .collect();

    let col_widths: &[Constraint] = if show_installed { &COL_WIDTHS_ALL } else { &COL_WIDTHS_UNINSTALLED };
    let table = Table::new(rows, col_widths.to_vec())
        .header(header)
        .block(Block::default()
            .borders(Borders::NONE)
            .style(Style::default().bg(BG)));

    f.render_widget(table, area);

    if state.filtered_indices.len() > visible_height {
        let mut sb_state = ScrollbarState::new(state.filtered_indices.len())
            .position(state.scroll_offset);
        f.render_stateful_widget(
            Scrollbar::default()
                .orientation(ScrollbarOrientation::VerticalRight)
                .begin_symbol(Some("↑")).end_symbol(Some("↓"))
                .style(Style::default().fg(BG3)),
            area.inner(&Margin { horizontal: 0, vertical: 1 }),
            &mut sb_state,
        );
    }
}
