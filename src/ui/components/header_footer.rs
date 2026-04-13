use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};
use crate::state::AppState;
use crate::{FilterMode, AppMode};
use crate::steam::human_size;
use crate::ui::theme::*;

pub fn draw_header(f: &mut Frame, state: &AppState, area: Rect) {
    let total: u64 = state.filtered_indices.iter()
        .filter_map(|&i| state.prefixes.get(i))
        .map(|p| p.size_bytes).sum();

    let roots_str = {
        let roots = state.all_roots();
        if roots.is_empty() {
            "No Steam install found".to_string()
        } else {
            roots.iter().map(|p| {
                let s = p.to_string_lossy();
                if s.contains("flatpak") { "Flatpak".into() }
                else { "Native".into() }
            }).collect::<std::collections::HashSet<String>>()
              .into_iter().collect::<Vec<_>>().join(" + ")
        }
    };

    let filter_badge = match state.filter_mode {
        FilterMode::UninstalledOnly => Span::styled(" UNINSTALLED ", Style::default().fg(BG).bg(ACCENT).add_modifier(Modifier::BOLD)),
        FilterMode::All             => Span::styled(" ALL ",         Style::default().fg(BG).bg(WARN).add_modifier(Modifier::BOLD)),
    };

    let search_span = if state.filter_text.is_empty() {
        Span::raw("")
    } else {
        Span::styled(format!("  /{}/", state.filter_text), Style::default().fg(ACCENT))
    };

    let freed_span = if state.total_deleted_bytes > 0 {
        Span::styled(format!("  freed {}  ", human_size(state.total_deleted_bytes)), Style::default().fg(OK))
    } else { Span::raw("") };

    let line = Line::from(vec![
        Span::styled("  PREMAN  ", Style::default().fg(BG).bg(ACCENT).add_modifier(Modifier::BOLD)),
        Span::raw("  "),
        filter_badge,
        search_span,
        Span::styled(
            format!("  {} prefixes  {}  ", state.filtered_indices.len(), human_size(total)),
            Style::default().fg(DIM),
        ),
        Span::styled(format!("roots: {}  ", roots_str), Style::default().fg(DIM)),
        freed_span,
        Span::styled("? help", Style::default().fg(BG3)),
    ]);

    f.render_widget(
        Paragraph::new(line)
            .block(Block::default().borders(Borders::BOTTOM)
                .border_style(Style::default().fg(BG3))
                .style(Style::default().bg(BG2))),
        area,
    );
}

pub fn draw_footer(f: &mut Frame, state: &AppState, area: Rect) {
    let (msg, style) = match &state.mode {
        AppMode::FilterText => (
            format!("FILTER: {}█  Enter=apply  Esc=cancel", state.filter_text),
            Style::default().fg(ACCENT),
        ),
        AppMode::Error(_) => ("".to_string(), Style::default().fg(DANGER)),
        _ => {
            if let Some(s) = &state.status_message {
                (s.clone(), Style::default().fg(OK))
            } else {
                (
                    "↑↓ navigate  ←→ sort  I invert  Del delete  E run  O open  F filter  A show all  D dirs  R reload  Q quit".to_string(),
                    Style::default().fg(DIM),
                )
            }
        }
    };
    f.render_widget(
        Paragraph::new(format!("  {}", msg)).style(style)
            .block(Block::default().borders(Borders::TOP)
                .border_style(Style::default().fg(BG3))
                .style(Style::default().bg(BG2))),
        area,
    );
}
