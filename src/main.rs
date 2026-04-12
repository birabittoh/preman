mod steam;
mod state;
mod ui;

use std::io;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use anyhow::Result;
use crossterm::{
    event::{
        self, DisableMouseCapture, EnableMouseCapture,
        Event, KeyCode, KeyEventKind, KeyModifiers,
        MouseButton, MouseEvent, MouseEventKind,
    },
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};

use state::{AppMode, AppState, FilterMode, SortColumn, DirModalFocus, DirModalConfirm};
use ui::{centered_rect, hit_test_table_row, hit_test_table_col};

fn main() -> Result<()> {
    let extra_dirs: Vec<PathBuf> = std::env::args().skip(1).map(PathBuf::from).collect();

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let result = run_app(&mut terminal, extra_dirs);

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen, DisableMouseCapture)?;
    terminal.show_cursor()?;

    if let Err(e) = result { eprintln!("Error: {}", e); std::process::exit(1); }
    Ok(())
}

fn run_app(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>, extra_dirs: Vec<PathBuf>) -> Result<()> {
    let mut app = AppState::new(extra_dirs);
    let tick = Duration::from_millis(200);
    let mut last_tick = Instant::now();

    loop {
        terminal.draw(|f| ui::draw(f, &app))?;

        // Drive deletion one item per tick (frame was already drawn with current name).
        if let AppMode::Deleting { pending, .. } = &app.mode {
            if pending.is_empty() {
                app.finish_delete();
                app.mode = AppMode::Normal;
                app.status_message = Some("Deleted.".into());
                continue;
            }
            let (path, _, size) = pending[0].clone();
            match std::fs::remove_dir_all(&path) {
                Ok(()) => {
                    app.total_deleted_bytes += size;
                    if let AppMode::Deleting { pending, current } = &mut app.mode {
                        pending.remove(0);
                        if let Some((_, next_name, _)) = pending.first() {
                            *current = next_name.clone();
                        }
                    }
                }
                Err(e) => {
                    app.mode = AppMode::Error(format!("Failed to delete '{}': {}", path.display(), e));
                }
            }
            continue;
        }

        let timeout = tick.checked_sub(last_tick.elapsed()).unwrap_or_default();
        if event::poll(timeout)? {
            match event::read()? {
                Event::Key(key) if key.kind == KeyEventKind::Press => {
                    // Clear status on any keypress (unless in a confirm dialog)
                    if !matches!(app.mode, AppMode::ConfirmDelete { .. }) {
                        app.status_message = None;
                    }
                    handle_key(&mut app, key.code, key.modifiers, terminal)?;
                    if matches!(app.mode, AppMode::Normal) && app.mode == AppMode::Normal {
                        // check quit flag via special sentinel
                    }
                }
                Event::Mouse(me) => {
                    handle_mouse(&mut app, me, terminal)?;
                }
                _ => {}
            }
        }
        if last_tick.elapsed() >= tick { last_tick = Instant::now(); }

        // Check quit sentinel
        if app.mode == AppMode::Normal && app.status_message.as_deref() == Some("__QUIT__") {
            break;
        }
    }
    Ok(())
}

fn handle_key(
    app: &mut AppState,
    code: KeyCode,
    modifiers: KeyModifiers,
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
) -> Result<()> {
    let vis_h = terminal.size().map(|s| s.height.saturating_sub(7) as usize).unwrap_or(20);
    let shift = modifiers.contains(KeyModifiers::SHIFT);
    let ctrl  = modifiers.contains(KeyModifiers::CONTROL);

    match &app.mode.clone() {
        // ── Loading — ignore all input ──────────────────────────────────────
        AppMode::Deleting { .. } => {}

        // ── Error / Help dismissal ──────────────────────────────────────────
        AppMode::Error(_) => { app.mode = AppMode::Normal; }
        AppMode::Help     => { app.mode = AppMode::Normal; }

        // ── Run exe modal ───────────────────────────────────────────────────
        AppMode::RunExe { prefix_idx, input } => {
            let prefix_idx = *prefix_idx;
            let mut input = input.clone();
            match code {
                KeyCode::Esc => { app.mode = AppMode::Normal; }
                KeyCode::Backspace => {
                    input.pop();
                    app.mode = AppMode::RunExe { prefix_idx, input };
                }
                KeyCode::Char(c) => {
                    input.push(c);
                    app.mode = AppMode::RunExe { prefix_idx, input };
                }
                KeyCode::Enter if !input.is_empty() => {
                    let prefix_path = app.prefixes[prefix_idx].path.clone();
                    let result = std::process::Command::new("wine")
                        .env("WINEPREFIX", &prefix_path)
                        .arg(&input)
                        .stdout(std::process::Stdio::null())
                        .stderr(std::process::Stdio::null())
                        .spawn();
                    app.mode = AppMode::Normal;
                    match result {
                        Ok(_)  => app.status_message = Some(format!("Launched: {}", input)),
                        Err(e) => app.mode = AppMode::Error(format!("Failed to launch '{}': {}", input, e)),
                    }
                }
                _ => {}
            }
        }

        // ── Text filter ─────────────────────────────────────────────────────
        AppMode::FilterText => match code {
            KeyCode::Esc => {
                app.mode = AppMode::Normal;
                app.filter_text.clear();
                app.apply_sort_and_filter();
            }
            KeyCode::Enter => {
                app.mode = AppMode::Normal;
                app.apply_sort_and_filter();
                app.selected = 0; app.scroll_offset = 0;
            }
            KeyCode::Backspace => { app.filter_text.pop(); app.apply_sort_and_filter(); }
            KeyCode::Char(c)   => { app.filter_text.push(c); app.apply_sort_and_filter(); }
            _ => {}
        },

        // ── Confirm delete ──────────────────────────────────────────────────
        AppMode::ConfirmDelete { step } => {
            let step = *step;
            match code {
                KeyCode::Char('y') | KeyCode::Char('Y') => {
                    let needs_second = app.any_selected_unsafe();
                    if needs_second && step == 1 {
                        app.mode = AppMode::ConfirmDelete { step: 2 };
                    } else {
                        app.begin_delete();
                    }
                }
                KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                    app.mode = AppMode::Normal;
                    app.status_message = Some("Deletion cancelled.".into());
                }
                _ => {}
            }
        }

        // ── Manage directories modal ────────────────────────────────────────
        AppMode::ManageDirs => {
            let modal = match &mut app.dir_modal {
                Some(m) => m,
                None => { app.mode = AppMode::Normal; return Ok(()); }
            };

            // Sub-state: confirm reset
            if modal.confirm == DirModalConfirm::ResetToDefaults {
                match code {
                    KeyCode::Char('y') | KeyCode::Char('Y') => {
                        app.reset_to_default_roots();
                        app.dir_modal = None;
                        app.mode = AppMode::Normal;
                        app.status_message = Some("Reset to default Steam directories.".into());
                    }
                    _ => {
                        if let Some(m) = &mut app.dir_modal {
                            m.confirm = DirModalConfirm::None;
                        }
                    }
                }
                return Ok(());
            }

            match code {
                KeyCode::Esc => {
                    app.dir_modal = None;
                    app.mode = AppMode::Normal;
                }
                KeyCode::Tab => {
                    let modal = app.dir_modal.as_mut().unwrap();
                    modal.focus = match modal.focus {
                        DirModalFocus::List  => DirModalFocus::Input,
                        DirModalFocus::Input => DirModalFocus::List,
                    };
                }
                KeyCode::Up | KeyCode::Char('k') => {
                    let modal = app.dir_modal.as_mut().unwrap();
                    if modal.focus == DirModalFocus::List && modal.selected > 0 {
                        modal.selected -= 1;
                    }
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    let max_idx = app.all_roots().len().saturating_sub(1);
                    let modal = app.dir_modal.as_mut().unwrap();
                    if modal.focus == DirModalFocus::List {
                        if modal.selected < max_idx { modal.selected += 1; }
                    }
                }
                KeyCode::Delete | KeyCode::Backspace => {
                    let modal = app.dir_modal.as_mut().unwrap();
                    if modal.focus == DirModalFocus::Input {
                        modal.input.pop();
                    } else {
                        // Delete selected dir (only custom ones)
                        let sel = modal.selected;
                        drop(modal);
                        match app.remove_custom_root(sel) {
                            Ok(()) => {
                                app.status_message = Some("Directory removed.".into());
                                let max = app.all_roots().len().saturating_sub(1);
                                if let Some(m) = &mut app.dir_modal {
                                    if m.selected > max { m.selected = max; }
                                }
                            }
                            Err(e) => { app.mode = AppMode::Error(e); app.dir_modal = None; }
                        }
                        return Ok(());
                    }
                }
                KeyCode::Char('d') | KeyCode::Char('D') => {
                    let modal = app.dir_modal.as_mut().unwrap();
                    if modal.focus != DirModalFocus::Input {
                        modal.confirm = DirModalConfirm::ResetToDefaults;
                        return Ok(());
                    }
                    modal.input.push(if code == KeyCode::Char('d') { 'd' } else { 'D' });
                }
                KeyCode::Char(c) => {
                    let modal = app.dir_modal.as_mut().unwrap();
                    if modal.focus == DirModalFocus::Input {
                        modal.input.push(c);
                    }
                }
                KeyCode::Enter => {
                    let modal = app.dir_modal.as_mut().unwrap();
                    if modal.focus == DirModalFocus::Input && !modal.input.is_empty() {
                        let path = modal.input.clone();
                        modal.input.clear();
                        drop(modal);
                        match app.add_custom_root(&path) {
                            Ok(()) => app.status_message = Some(format!("Added: {}", path)),
                            Err(e) => { app.mode = AppMode::Error(e); app.dir_modal = None; return Ok(()); }
                        }
                    }
                }
                _ => {}
            }
        }

        // ── Normal mode ─────────────────────────────────────────────────────
        AppMode::Normal => match code {
            KeyCode::Char('a') | KeyCode::Char('A') if ctrl => {
                app.selection = app.filtered_indices.iter().copied().collect();
                if !app.filtered_indices.is_empty() {
                    app.shift_anchor = app.filtered_indices.len() - 1;
                }
            }
            KeyCode::Char('q') | KeyCode::Char('Q') => {
                app.status_message = Some("__QUIT__".into());
            }
            KeyCode::Esc => {
                if !app.filter_text.is_empty() {
                    app.filter_text.clear();
                    app.filter_mode = FilterMode::All;
                    app.apply_sort_and_filter();
                } else {
                    app.status_message = Some("__QUIT__".into());
                }
            }
            KeyCode::Up    | KeyCode::Char('k') => {
                if shift { app.extend_up(); } else { app.move_up(); }
            }
            KeyCode::Down  | KeyCode::Char('j') => {
                if shift { app.extend_down(vis_h); } else { app.move_down(vis_h); }
            }
            KeyCode::Left  | KeyCode::Char('h') => {
                let mut prev = app.sort_col.prev();
                if app.filter_mode != FilterMode::All && prev == SortColumn::Installed { prev = prev.prev(); }
                app.sort_by_col(prev);
            }
            KeyCode::Right | KeyCode::Char('l') => {
                let mut next = app.sort_col.next();
                if app.filter_mode != FilterMode::All && next == SortColumn::Installed { next = next.next(); }
                app.sort_by_col(next);
            }
            KeyCode::Char('i') | KeyCode::Char('I') => {
                app.reverse_sort();
            }
            KeyCode::PageUp   => { for _ in 0..vis_h { app.move_up(); } }
            KeyCode::PageDown => { for _ in 0..vis_h { app.move_down(vis_h); } }
            KeyCode::Home => {
                app.selected = 0; app.scroll_offset = 0;
                app.selection.clear(); app.shift_anchor = 0;
            }
            KeyCode::End  => {
                if !app.filtered_indices.is_empty() {
                    app.selected = app.filtered_indices.len() - 1;
                    app.scroll_offset = app.selected.saturating_sub(vis_h - 1);
                    app.selection.clear(); app.shift_anchor = app.selected;
                }
            }
            KeyCode::Delete => {
                if !app.effective_selection().is_empty() {
                    app.mode = AppMode::ConfirmDelete { step: 1 };
                }
            }
            KeyCode::Char('f') | KeyCode::Char('F') | KeyCode::Char('/') => {
                app.filter_text.clear();
                app.mode = AppMode::FilterText;
            }
            KeyCode::Char('a') | KeyCode::Char('A') if !ctrl => app.toggle_filter_mode(),
            KeyCode::Char('r') | KeyCode::Char('R') => {
                app.reload();
                app.status_message = Some(format!(
                    "Reloaded — {} prefixes across {} root(s).",
                    app.prefixes.len(), app.all_roots().len()
                ));
            }
            KeyCode::Char('e') | KeyCode::Char('E') => {
                if let Some(&idx) = app.filtered_indices.get(app.selected) {
                    let start_dir = app.prefixes[idx].path.clone();
                    match try_file_dialog(&start_dir) {
                        DialogResult::Picked(path) => {
                            let result = std::process::Command::new("wine")
                                .env("WINEPREFIX", &start_dir)
                                .arg(&path)
                                .stdout(std::process::Stdio::null())
                                .stderr(std::process::Stdio::null())
                                .spawn();
                            match result {
                                Ok(_)  => app.status_message = Some(format!("Launched: {}", path)),
                                Err(e) => app.mode = AppMode::Error(format!("Failed to launch '{}': {}", path, e)),
                            }
                        }
                        DialogResult::Cancelled  => {} // user dismissed — do nothing
                        DialogResult::Unavailable => {
                            app.mode = AppMode::RunExe { prefix_idx: idx, input: String::new() };
                        }
                    }
                }
            }
            KeyCode::Char('d') | KeyCode::Char('D') => {
                app.open_dir_modal();
            }
            KeyCode::Char('o') | KeyCode::Char('O') => {
                open_selection(app);
            }
            KeyCode::Char('?') => { app.mode = AppMode::Help; }
            _ => {}
        },
    }
    Ok(())
}

fn handle_mouse(
    app: &mut AppState,
    me: MouseEvent,
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
) -> Result<()> {
    let size = terminal.size()?;
    let vis_h = size.height.saturating_sub(7) as usize;

    // Layout mirrors draw(): header(3) + body(min) + footer(3)
    let header_h: u16 = 3;
    let footer_h: u16 = 3;
    let body_y   = header_h;
    let body_h   = size.height.saturating_sub(header_h + footer_h);
    let body_area = ratatui::layout::Rect {
        x: 0, y: body_y, width: size.width, height: body_h,
    };
    let detail_w: u16 = 40;
    let table_area = ratatui::layout::Rect {
        x: 0, y: body_y,
        width: size.width.saturating_sub(detail_w),
        height: body_h,
    };

    match app.mode.clone() {
        // ── Loading — ignore all input ──────────────────────────────────────
        AppMode::Deleting { .. } => {}

        // ── Modals: keyboard-only, swallow all mouse input ─────────────────
        AppMode::ConfirmDelete { .. } => {}

        AppMode::ManageDirs => {
            let full = ratatui::layout::Rect { x: 0, y: 0, width: size.width, height: size.height };
            let popup = centered_rect(70, 22, full);
            match me.kind {
                MouseEventKind::Down(MouseButton::Left) => {
                    let x = me.column; let y = me.row;
                    if !rect_contains(popup, x, y) {
                        // Click outside closes modal
                        app.dir_modal = None;
                        app.mode = AppMode::Normal;
                        return Ok(());
                    }
                    // Check if "Reset to Defaults" button area was clicked
                    let btn_y = popup.y + popup.height.saturating_sub(3);
                    if y >= btn_y {
                        // Left half = reset button, right half = close
                        if x < popup.x + popup.width / 2 {
                            handle_key(app, KeyCode::Char('d'), KeyModifiers::NONE, terminal)?;
                        } else {
                            handle_key(app, KeyCode::Esc, KeyModifiers::NONE, terminal)?;
                        }
                        return Ok(());
                    }
                    // Check if input area was clicked
                    let input_y_start = popup.y + popup.height.saturating_sub(6);
                    let input_y_end = popup.y + popup.height.saturating_sub(3);
                    if y >= input_y_start && y < input_y_end {
                        if let Some(m) = &mut app.dir_modal {
                            m.focus = DirModalFocus::Input;
                        }
                        return Ok(());
                    }
                    // Otherwise click in list area
                    let max_roots = app.all_roots().len().saturating_sub(1);
                    if let Some(m) = &mut app.dir_modal {
                        m.focus = DirModalFocus::List;
                        let list_rel = y.saturating_sub(popup.y + 1) as usize;
                        m.selected = list_rel.min(max_roots);
                    }
                }
                MouseEventKind::ScrollUp => {
                    if let Some(m) = &mut app.dir_modal {
                        if m.selected > 0 { m.selected -= 1; }
                    }
                }
                MouseEventKind::ScrollDown => {
                    let max_roots2 = app.all_roots().len().saturating_sub(1);
                    if let Some(m) = &mut app.dir_modal {
                        if m.selected < max_roots2 { m.selected += 1; }
                    }
                }
                _ => {}
            }
        }

        AppMode::Normal | AppMode::FilterText => {
            let ctrl = me.modifiers.contains(crossterm::event::KeyModifiers::CONTROL);
            match me.kind {
                MouseEventKind::Down(MouseButton::Left) => {
                    let x = me.column; let y = me.row;
                    // Click in header row → sort column (never multi-select)
                    if y == body_y && x < table_area.x + table_area.width {
                        if let Some(col) = hit_test_table_col(x, table_area, app.filter_mode == FilterMode::All) {
                            app.sort_by_col(col);
                        }
                        return Ok(());
                    }
                    // Click in table body
                    if y > body_y && y < body_y + body_h && x < table_area.x + table_area.width {
                        if let Some(row_idx) = hit_test_table_row(y, body_area, app.scroll_offset) {
                            if row_idx < app.filtered_indices.len() {
                                let now = Instant::now();
                                let is_double = app.last_click
                                    .map(|(r, t)| r == row_idx && now.duration_since(t) < Duration::from_millis(400))
                                    .unwrap_or(false);
                                if is_double {
                                    app.last_click = None;
                                    let real_idx = app.filtered_indices[row_idx];
                                    open_prefix(app, real_idx);
                                } else if ctrl {
                                    app.last_click = None;
                                    app.ctrl_toggle(row_idx);
                                } else {
                                    app.last_click = Some((row_idx, now));
                                    app.drag_start(row_idx);
                                }
                            }
                        }
                        return Ok(());
                    }
                }
                MouseEventKind::Drag(MouseButton::Left) => {
                    let x = me.column; let y = me.row;
                    if y > body_y && y < body_y + body_h && x < table_area.x + table_area.width {
                        if let Some(row_idx) = hit_test_table_row(y, body_area, app.scroll_offset) {
                            app.drag_to(row_idx);
                        }
                    }
                }
                MouseEventKind::Up(MouseButton::Left) => {
                    app.drag_anchor = None;
                }
                MouseEventKind::ScrollUp => {
                    app.move_up();
                }
                MouseEventKind::ScrollDown => {
                    app.move_down(vis_h);
                }
                _ => {}
            }
        }

        _ => {}
    }
    Ok(())
}

fn rect_contains(r: ratatui::layout::Rect, x: u16, y: u16) -> bool {
    x >= r.x && x < r.x + r.width && y >= r.y && y < r.y + r.height
}

enum DialogResult {
    Picked(String),
    Cancelled,
    Unavailable,
}

/// Try to pick a file via a GUI dialog (zenity or kdialog).
/// - Picked(path)  — user selected a file
/// - Cancelled     — dialog opened but user dismissed it
/// - Unavailable   — no supported dialog tool found; fall back to TUI
fn try_file_dialog(start_dir: &std::path::Path) -> DialogResult {
    let start = start_dir.to_str().unwrap_or("/");
    let candidates = [
        ("zenity", vec!["--file-selection", "--title=Select executable",
                        "--file-filter=Windows executables (*.exe) | *.exe"]),
        ("kdialog", vec!["--getopenfilename", start, "*.exe|Windows executables"]),
    ];
    for (cmd, args) in &candidates {
        match std::process::Command::new(cmd).args(args).output() {
            Err(_) => continue, // not installed — try next
            Ok(out) => {
                return if out.status.success() {
                    let path = String::from_utf8_lossy(&out.stdout).trim().to_string();
                    if path.is_empty() { DialogResult::Cancelled } else { DialogResult::Picked(path) }
                } else {
                    DialogResult::Cancelled // dialog opened, user hit Cancel
                };
            }
        }
    }
    DialogResult::Unavailable
}

/// Open one or more paths with xdg-open.
fn open_paths(paths: impl IntoIterator<Item = std::path::PathBuf>) {
    for p in paths {
        let _ = std::process::Command::new("xdg-open").arg(&p).spawn();
    }
}

/// Open the currently selected prefix(es) in the file manager.
fn open_selection(app: &AppState) {
    let sel = app.effective_selection();
    if sel.len() == 1 {
        open_paths([app.prefixes[sel[0]].path.clone()]);
    } else if sel.len() > 1 {
        let mut dirs: Vec<std::path::PathBuf> = sel.iter()
            .filter_map(|&i| app.prefixes[i].path.parent().map(|p| p.to_path_buf()))
            .collect();
        dirs.sort();
        dirs.dedup();
        open_paths(dirs);
    }
}

/// Open the prefix at the given real index in the file manager.
fn open_prefix(app: &AppState, real_idx: usize) {
    if let Some(p) = app.prefixes.get(real_idx) {
        open_paths([p.path.clone()]);
    }
}
