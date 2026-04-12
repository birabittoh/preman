use ratatui::{
    backend::Backend,
    layout::{Alignment, Constraint, Direction, Layout, Margin, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{
        Block, BorderType, Borders, Cell, Clear, Paragraph, Row, Scrollbar,
        ScrollbarOrientation, ScrollbarState, Table,
    },
    Frame,
};

use crate::state::{AppMode, AppState, FilterMode, SortColumn, DirModalFocus, DirModalConfirm};
use crate::steam::human_size;

// ─── Palette ──────────────────────────────────────────────────────────────────
const ACCENT: Color = Color::Rgb(103, 193, 245);
const DANGER: Color = Color::Rgb(220, 80,  80);
const WARN:   Color = Color::Rgb(220, 160, 40);
const OK:     Color = Color::Rgb(80,  200, 120);
const DIM:    Color = Color::Rgb(110, 115, 130);
const BG:     Color = Color::Rgb(14,  17,  23);
const BG2:    Color = Color::Rgb(20,  24,  33);
const BG3:    Color = Color::Rgb(32,  37,  52);
const SEL:    Color = Color::Rgb(28,  55,  95);
const MSEL:   Color = Color::Rgb(20,  60,  55);  // multi-selected (non-cursor)
const FG:     Color = Color::Rgb(215, 220, 235);

// ─── Column layout constants ─────────────────────────────────────────────────
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

// ─── Top-level draw ───────────────────────────────────────────────────────────

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
        AppMode::ConfirmDelete { step } => draw_confirm_delete(f, state, *step),
        AppMode::Deleting { .. }     => draw_loading(f, state),
        AppMode::ManageDirs          => draw_dir_modal(f, state),
        AppMode::RunExe { prefix_idx, input } => draw_run_exe(f, state, *prefix_idx, input),
        AppMode::Help                => draw_help(f),
        AppMode::Error(msg)          => { let m = msg.clone(); draw_error(f, &m); }
        _                            => {}
    }
}

// ─── Header ───────────────────────────────────────────────────────────────────

fn draw_header(f: &mut Frame, state: &AppState, area: Rect) {
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

// ─── Body ─────────────────────────────────────────────────────────────────────

fn draw_body(f: &mut Frame, state: &AppState, area: Rect) {
    draw_table(f, state, area);
}

// ─── Table ────────────────────────────────────────────────────────────────────

fn col_header_span<'a>(label: &'a str, col: SortColumn, state: &AppState) -> Span<'a> {
    let active = state.sort_col == col;
    let indicator = if active { if state.sort_asc() { " ▲" } else { " ▼" } } else { "" };
    let text = Box::leak(format!("{}{}", label, indicator).into_boxed_str()) as &str;
    if active {
        Span::styled(text, Style::default().fg(Color::White).add_modifier(Modifier::BOLD))
    } else {
        Span::styled(text, Style::default().fg(ACCENT))
    }
}

pub fn draw_table(f: &mut Frame, state: &AppState, area: Rect) {
    // area.height minus the one header row = actual visible data rows.
    let visible_height = area.height.saturating_sub(1) as usize;

    let show_installed = state.filter_mode == FilterMode::All;

    // Header row with sort indicators
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

    // Scrollbar
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

// ─── Footer ───────────────────────────────────────────────────────────────────

fn draw_footer(f: &mut Frame, state: &AppState, area: Rect) {
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

// ─── Confirm delete modal ─────────────────────────────────────────────────────

fn draw_confirm_delete(f: &mut Frame, state: &AppState, step: u8) {
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
        // Multi-selection summary
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
        // Single-item detail
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

// ─── Manage Directories modal ─────────────────────────────────────────────────

pub fn draw_dir_modal(f: &mut Frame, state: &AppState) {
    let area = f.size();
    let popup = centered_rect(70, 22, area);
    f.render_widget(Clear, popup);

    let modal = match &state.dir_modal { Some(m) => m, None => return };
    let all_roots = state.all_roots();

    // Check if we're in confirm-reset state
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
            Constraint::Min(0),     // list
            Constraint::Length(3),  // input
            Constraint::Length(3),  // buttons
        ])
        .split(inner);

    // ── Directory list ──
    let list_height = chunks[0].height as usize;
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

    let list_table = Table::new(list_rows, [
        Constraint::Length(2),
        Constraint::Length(12),
        Constraint::Min(0),
    ]).block(Block::default().borders(Borders::BOTTOM)
        .border_style(Style::default().fg(BG3))
        .title(Span::styled(" Directories (↑↓ select, Del=remove) ",
            Style::default().fg(DIM))));
    f.render_widget(list_table, chunks[0]);

    // ── Input ──
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

    // ── Buttons ──
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

// ─── Run exe modal ────────────────────────────────────────────────────────────

fn draw_run_exe(f: &mut Frame, state: &AppState, prefix_idx: usize, input: &str) {
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

// ─── Help overlay ─────────────────────────────────────────────────────────────

fn draw_help(f: &mut Frame) {
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

// ─── Deleting overlay ─────────────────────────────────────────────────────────

fn draw_loading(f: &mut Frame, state: &AppState) {
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

// ─── Error overlay ────────────────────────────────────────────────────────────

fn draw_error(f: &mut Frame, msg: &str) {
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

// ─── Geometry helpers ─────────────────────────────────────────────────────────

pub fn centered_rect(w: u16, h: u16, area: Rect) -> Rect {
    Rect {
        x: area.x + area.width.saturating_sub(w) / 2,
        y: area.y + area.height.saturating_sub(h) / 2,
        width: w.min(area.width),
        height: h.min(area.height),
    }
}

/// Given a terminal area and the body rect, return what row index was clicked
/// (0-indexed into filtered_indices, accounting for scroll_offset).
/// Returns None if click is outside the list or on the header.
pub fn hit_test_table_row(click_y: u16, body_area: Rect, scroll_offset: usize) -> Option<usize> {
    // body splits: left = table (Constraint::Min), right = detail (Length 40)
    // table has a header row at relative y=0 (height=1)
    let rel_y = click_y.checked_sub(body_area.y)?;
    if rel_y == 0 { return None; } // header row
    let row = rel_y as usize - 1 + scroll_offset;
    Some(row)
}

/// Returns which SortColumn was clicked based on x position within the table area.
pub fn hit_test_table_col(click_x: u16, table_area: Rect, show_installed: bool) -> Option<SortColumn> {
    let rel_x = click_x.checked_sub(table_area.x)? as usize;
    let available = table_area.width as usize;
    if show_installed {
        // COL_WIDTHS_ALL: Min(28), 10, 10, 11, 7  — fixed = 38
        // COL_WIDTHS_ALL: Min(28), 10, 10, 7, 11  — fixed = 38
        let name_w = available.saturating_sub(38);
        let boundaries = [name_w, name_w+10, name_w+20, name_w+27, name_w+38];
        if rel_x < boundaries[0]      { Some(SortColumn::Name) }
        else if rel_x < boundaries[1] { Some(SortColumn::AppId) }
        else if rel_x < boundaries[2] { Some(SortColumn::Size) }
        else if rel_x < boundaries[3] { Some(SortColumn::Cloud) }
        else                           { Some(SortColumn::Installed) }
    } else {
        // COL_WIDTHS_UNINSTALLED: Min(28), 10, 10, 7  — fixed = 27
        let name_w = available.saturating_sub(27);
        let boundaries = [name_w, name_w+10, name_w+20, name_w+27];
        if rel_x < boundaries[0]      { Some(SortColumn::Name) }
        else if rel_x < boundaries[1] { Some(SortColumn::AppId) }
        else if rel_x < boundaries[2] { Some(SortColumn::Size) }
        else                           { Some(SortColumn::Cloud) }
    }
}
