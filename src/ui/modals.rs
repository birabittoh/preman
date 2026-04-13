use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Modifier, Style, Color},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, Paragraph},
    Frame,
};
use crate::state::AppState;
use crate::{AppMode, DirModalConfirm, DirModalFocus};
use crate::ui::theme::*;
use crate::ui::utils::centered_rect;

pub fn draw_confirm_delete(f: &mut Frame, state: &AppState, step: u8) {
    let area = f.size();
    let selection = state.effective_selection();
    let multi = selection.len() > 1;
    let unsafe_delete = state.any_selected_unsafe();

    let popup = centered_rect(64, if multi { 14 } else { 13 }, area);
    f.render_widget(Clear, popup);

    let title = match (step, unsafe_delete) {
        (_, false) => Span::styled(" ⚠  Confirm Delete ", Style::default().fg(DANGER).add_modifier(Modifier::BOLD)),
        (1, true)  => Span::styled(" ⚠  Confirm Delete – Saves at Risk ", Style::default().fg(WARN).add_modifier(Modifier::BOLD)),
        _          => Span::styled(" ⛔  FINAL WARNING – Saves Will Be Lost ", Style::default().fg(DANGER).add_modifier(Modifier::BOLD)),
    };

    let block = Block::default().title(title).borders(Borders::ALL)
        .border_type(if step == 2 { BorderType::Double } else { BorderType::Rounded })
        .border_style(Style::default().fg(if step == 2 { DANGER } else { WARN }))
        .style(Style::default().bg(Color::Rgb(28, 12, 12)));

    let inner = block.inner(popup);
    f.render_widget(block, popup);

    let mut lines = vec![Line::from("")];

    if multi {
        let total_bytes: u64 = selection.iter().map(|&i| state.prefixes[i].size_bytes).sum();
        let with_cloud: usize = selection.iter().filter(|&&i| state.prefixes[i].has_cloud_saves()).count();
        let no_cloud = selection.len() - with_cloud;
        lines.push(Line::from(vec![
            Span::styled("  Items  ", Style::default().fg(DIM)),
            Span::styled(format!("{} prefixes", selection.len()), Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
        ]));
        lines.push(Line::from(vec![
            Span::styled("  Size   ", Style::default().fg(DIM)),
            Span::styled(crate::steam::human_size(total_bytes), Style::default().fg(WARN)),
        ]));
        let cloud_col = if no_cloud > 0 { WARN } else { OK };
        let cloud_lbl = if no_cloud > 0 {
            format!("{} without cloud saves", no_cloud)
        } else {
            "All have cloud saves ☁".to_string()
        };
        lines.push(Line::from(vec![
            Span::styled("  Cloud  ", Style::default().fg(DIM)),
            Span::styled(cloud_lbl, Style::default().fg(cloud_col)),
        ]));
    } else {
        let Some(prefix) = state.selected_prefix() else { return };
        let cloud_line = if prefix.game.is_none() {
            Line::from(vec![
                Span::styled("  Cloud  ", Style::default().fg(DIM)),
                Span::styled("Unknown game — cloud saves unverified", Style::default().fg(WARN)),
            ])
        } else if prefix.has_cloud_saves() {
            Line::from(vec![
                Span::styled("  Cloud  ", Style::default().fg(DIM)),
                Span::styled("Detected ☁ — your progress is safe", Style::default().fg(OK)),
            ])
        } else {
            Line::from(vec![
                Span::styled("  Cloud  ", Style::default().fg(DIM)),
                Span::styled("Not detected ✗ — local saves only!", Style::default().fg(WARN).add_modifier(Modifier::BOLD)),
            ])
        };
        lines.push(Line::from(vec![
            Span::styled("  Game   ", Style::default().fg(DIM)),
            Span::styled(prefix.game_name(), Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
        ]));
        lines.push(Line::from(vec![
            Span::styled("  Size   ", Style::default().fg(DIM)),
            Span::styled(prefix.size_human(), Style::default().fg(WARN)),
        ]));
        lines.push(cloud_line);
    }

    lines.push(Line::from(""));

    if unsafe_delete && step == 1 {
        lines.push(Line::from(Span::styled(
            "  ⚠  Deleting may permanently erase local save data.",
            Style::default().fg(WARN),
        )));
        lines.push(Line::from(Span::styled(
            "     Confirm once more to proceed.",
            Style::default().fg(DIM),
        )));
        lines.push(Line::from(""));
    } else if unsafe_delete && step == 2 {
        lines.push(Line::from(Span::styled(
            "  ⛔  THIS CANNOT BE UNDONE. ALL LOCAL SAVES WILL BE GONE.",
            Style::default().fg(DANGER).add_modifier(Modifier::BOLD),
        )));
        lines.push(Line::from(""));
    }

    lines.push(Line::from(vec![
        Span::styled("  [Y] ", Style::default().fg(DANGER).add_modifier(Modifier::BOLD)),
        Span::styled("Confirm    ", Style::default().fg(FG)),
        Span::styled("[N] / [Esc] ", Style::default().fg(OK).add_modifier(Modifier::BOLD)),
        Span::styled("Cancel", Style::default().fg(FG)),
    ]));

    f.render_widget(Paragraph::new(lines), inner);
}

pub fn draw_dir_modal(f: &mut Frame, state: &AppState) {
    let area = f.size();
    let popup = centered_rect(70, 22, area);
    f.render_widget(Clear, popup);

    let modal = match &state.dir_modal { Some(m) => m, None => return };
    let all_roots = state.all_roots();

    if modal.confirm == DirModalConfirm::ResetToDefaults {
        let cpopup = centered_rect(58, 10, area);
        f.render_widget(Clear, cpopup);
        let cblock = Block::default()
            .title(Span::styled(" Reset to Defaults ", Style::default().fg(WARN).add_modifier(Modifier::BOLD)))
            .borders(Borders::ALL).border_type(BorderType::Double)
            .border_style(Style::default().fg(WARN))
            .style(Style::default().bg(Color::Rgb(28, 24, 10)));
        let cinner = cblock.inner(cpopup);
        f.render_widget(cblock, cpopup);
        f.render_widget(Paragraph::new(vec![
            Line::from(""),
            Line::from(Span::styled(
                "  Remove all custom directories and re-detect",
                Style::default().fg(FG),
            )),
            Line::from(Span::styled("  Steam roots from the filesystem?", Style::default().fg(FG))),
            Line::from(""),
            Line::from(vec![
                Span::styled("  [Y] ", Style::default().fg(DANGER).add_modifier(Modifier::BOLD)),
                Span::styled("Yes, reset    ", Style::default().fg(FG)),
                Span::styled("[N] / [Esc] ", Style::default().fg(OK).add_modifier(Modifier::BOLD)),
                Span::styled("Cancel", Style::default().fg(FG)),
            ]),
        ]), cinner);
        return;
    }

    let block = Block::default()
        .title(Span::styled(" Manage Steam Directories ", Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)))
        .borders(Borders::ALL).border_type(BorderType::Rounded)
        .border_style(Style::default().fg(ACCENT))
        .style(Style::default().bg(BG2));
    let inner = block.inner(popup);
    f.render_widget(block, popup);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(0),
            Constraint::Length(3),
            Constraint::Length(3),
        ])
        .split(inner);

    let list_rows: Vec<Row> = all_roots.iter().enumerate()
        .map(|(i, path)| {
            let is_default = state.default_roots.contains(path);
            let is_sel = i == modal.selected && modal.focus == DirModalFocus::List;
            let tag = if is_default { " [default] " } else { " [custom]  " };
            let tag_col = if is_default { DIM } else { ACCENT };
            let bg = if is_sel { SEL } else if i % 2 == 0 { BG } else { BG2 };
            let path_str = path.to_string_lossy().to_string();
            Row::new(vec![
                Cell::from(if is_sel { "▶" } else { " " }).style(Style::default().fg(ACCENT)),
                Cell::from(tag).style(Style::default().fg(tag_col)),
                Cell::from(path_str).style(Style::default().fg(if is_default { DIM } else { FG })),
            ]).style(Style::default().bg(bg)).height(1)
        })
        .collect();

    use ratatui::widgets::{Cell, Row, Table};
    let list_table = Table::new(list_rows, [
        Constraint::Length(2),
        Constraint::Length(12),
        Constraint::Min(0),
    ]).block(Block::default().borders(Borders::BOTTOM)
        .border_style(Style::default().fg(BG3))
        .title(Span::styled(" Directories (↑↓ select, Del=remove) ",
            Style::default().fg(DIM))));
    f.render_widget(list_table, chunks[0]);

    let input_active = modal.focus == DirModalFocus::Input;
    let input_border_col = if input_active { ACCENT } else { BG3 };
    let input_content = if input_active {
        format!("{}█", modal.input)
    } else {
        modal.input.clone()
    };
    f.render_widget(
        Paragraph::new(format!(" {}", input_content))
            .style(Style::default().fg(if input_active { Color::White } else { DIM }))
            .block(Block::default().borders(Borders::ALL)
                .border_style(Style::default().fg(input_border_col))
                .title(Span::styled(" Add custom path (Tab to focus, Enter to add) ",
                    Style::default().fg(DIM)))),
        chunks[1],
    );

    let btn_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(chunks[2]);

    f.render_widget(
        Paragraph::new(Span::styled("  [D] Reset to Defaults", Style::default().fg(WARN).add_modifier(Modifier::BOLD)))
            .block(Block::default().borders(Borders::TOP).border_style(Style::default().fg(BG3))),
        btn_chunks[0],
    );
    f.render_widget(
        Paragraph::new(Span::styled("  [Esc] Close", Style::default().fg(DIM)))
            .alignment(Alignment::Right)
            .block(Block::default().borders(Borders::TOP).border_style(Style::default().fg(BG3))),
        btn_chunks[1],
    );
}

pub fn draw_run_exe(f: &mut Frame, state: &AppState, prefix_idx: usize, input: &str) {
    let area = f.size();
    let popup = centered_rect(70, 7, area);
    f.render_widget(Clear, popup);

    let prefix = &state.prefixes[prefix_idx];
    let title = format!(" Run executable in: {} ", prefix.game_name());

    let block = Block::default()
        .title(Span::styled(title, Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)))
        .borders(Borders::ALL).border_type(BorderType::Rounded)
        .border_style(Style::default().fg(ACCENT))
        .style(Style::default().bg(BG2));
    let inner = block.inner(popup);
    f.render_widget(block, popup);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(2)])
        .split(inner);

    f.render_widget(
        Paragraph::new(format!(" {}█", input))
            .style(Style::default().fg(Color::White))
            .block(Block::default().borders(Borders::ALL)
                .border_style(Style::default().fg(ACCENT))
                .title(Span::styled(" Path to .exe (relative or absolute) ",
                    Style::default().fg(DIM)))),
        chunks[0],
    );

    f.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("  [Enter] ", Style::default().fg(OK).add_modifier(Modifier::BOLD)),
            Span::styled("Launch    ", Style::default().fg(FG)),
            Span::styled("[Esc] ", Style::default().fg(DIM).add_modifier(Modifier::BOLD)),
            Span::styled("Cancel", Style::default().fg(DIM)),
        ])),
        chunks[1],
    );
}

pub fn draw_help(f: &mut Frame) {
    let area = f.size();
    let popup = centered_rect(68, 28, area);
    f.render_widget(Clear, popup);
    let block = Block::default()
        .title(Span::styled(" Help ", Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)))
        .borders(Borders::ALL).border_type(BorderType::Rounded)
        .border_style(Style::default().fg(ACCENT)).style(Style::default().bg(BG2));
    let inner = block.inner(popup);
    f.render_widget(block, popup);

    let entries: &[(&str, &str)] = &[
        ("↑/↓  j/k",    "Navigate prefix list"),
        ("←/→  h/l",    "Change sort column (click header too)"),
        ("i",            "Invert current sort order"),
        ("PgUp/PgDn",   "Scroll page"),
        ("Home/End",    "Jump to first/last"),
        ("Del",         "Delete selected prefix"),
        ("E",           "Run .exe in prefix"),
        ("O",           "Open parent dir(s) in file manager"),
        ("F   /",       "Text filter"),
        ("A",           "Toggle Uninstalled-only / All"),
        ("D",           "Manage Steam directories"),
        ("F5",          "Reload — rescan all roots"),
        ("?",           "This help"),
        ("Q / Esc",     "Quit"),
        ("", ""),
        ("MOUSE SUPPORT", ""),
        ("", "Click row          Select prefix"),
        ("", "Click column hdr  Sort by that column"),
        ("", "Click buttons     Activate action"),
        ("", "Scroll wheel      Scroll list"),
        ("", ""),
        ("CLOUD SAVE SAFETY", ""),
        ("", "Games with no detected cloud saves require"),
        ("", "TWO confirmations before the prefix is deleted."),
        ("", ""),
        ("STEAM PATHS DETECTED", ""),
        ("", "Native ~/.steam/steam  or  ~/.local/share/Steam"),
        ("", "Flatpak ~/.var/app/com.valvesoftware.Steam"),
        ("", "Custom paths via [D] → Add path"),
    ];

    let lines: Vec<Line> = entries.iter().map(|(k, v)| {
        if k.is_empty() && v.is_empty() { Line::from("") }
        else if v.is_empty() {
            Line::from(Span::styled(format!("  {}", k),
                Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)))
        } else if k.is_empty() {
            Line::from(Span::styled(format!("    {}", v), Style::default().fg(DIM)))
        } else {
            Line::from(vec![
                Span::styled(format!("  {:14}", k), Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)),
                Span::styled(format!("  {}", v), Style::default().fg(FG)),
            ])
        }
    }).collect();

    f.render_widget(Paragraph::new(lines), inner);
}

pub fn draw_startup(f: &mut Frame) {
    let area = f.size();
    let popup = centered_rect(50, 7, area);
    f.render_widget(Clear, popup);
    let block = Block::default()
        .title(Span::styled(" Initializing… ", Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)))
        .borders(Borders::ALL).border_type(BorderType::Rounded)
        .border_style(Style::default().fg(ACCENT))
        .style(Style::default().bg(BG2));
    let inner = block.inner(popup);
    f.render_widget(block, popup);

    f.render_widget(Paragraph::new(vec![
        Line::from(""),
        Line::from(Span::styled("  Scanning Steam directories for prefixes…", Style::default().fg(FG))),
        Line::from(""),
        Line::from(Span::styled("  Please wait…", Style::default().fg(DIM))),
    ]), inner);
}

pub fn draw_loading(f: &mut Frame, state: &AppState) {
    let area = f.size();
    let popup = centered_rect(50, 7, area);
    f.render_widget(Clear, popup);
    let block = Block::default()
        .title(Span::styled(" Deleting… ", Style::default().fg(DANGER).add_modifier(Modifier::BOLD)))
        .borders(Borders::ALL).border_type(BorderType::Rounded)
        .border_style(Style::default().fg(DANGER))
        .style(Style::default().bg(Color::Rgb(28, 12, 12)));
    let inner = block.inner(popup);
    f.render_widget(block, popup);

    let (current, remaining) = match &state.mode {
        AppMode::Deleting { pending, current } => (current.as_str(), pending.len()),
        _ => ("", 0),
    };

    let label = format!("  Removing: {}", current);
    let sub = if remaining > 1 {
        format!("  {} more after this…", remaining - 1)
    } else {
        "  Please wait…".to_string()
    };

    f.render_widget(Paragraph::new(vec![
        Line::from(""),
        Line::from(Span::styled(label, Style::default().fg(FG))),
        Line::from(""),
        Line::from(Span::styled(sub, Style::default().fg(DIM))),
    ]), inner);
}

pub fn draw_error(f: &mut Frame, msg: &str) {
    let area = f.size();
    let popup = centered_rect(64, 8, area);
    f.render_widget(Clear, popup);
    let block = Block::default()
        .title(Span::styled(" Error ", Style::default().fg(DANGER).add_modifier(Modifier::BOLD)))
        .borders(Borders::ALL).border_type(BorderType::Double)
        .border_style(Style::default().fg(DANGER))
        .style(Style::default().bg(Color::Rgb(28, 10, 10)));
    let inner = block.inner(popup);
    f.render_widget(block, popup);
    f.render_widget(Paragraph::new(vec![
        Line::from(""),
        Line::from(Span::styled(format!("  {}", msg), Style::default().fg(FG))),
        Line::from(""),
        Line::from(Span::styled("  [Any key] Dismiss", Style::default().fg(DIM))),
    ]), inner);
}
