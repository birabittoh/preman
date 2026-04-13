mod steam;
mod state;
mod ui;
mod handlers;
pub mod app_types;

use std::io;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use anyhow::Result;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};

use state::AppState;
pub use crate::app_types::{AppMode, SortColumn, FilterMode, DirModalState, DirModalFocus, DirModalConfirm};
use handlers::{handle_key, handle_mouse};

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

    if let Err(e) = result {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
    Ok(())
}

fn run_app(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>, extra_dirs: Vec<PathBuf>) -> Result<()> {
    let mut app = AppState::new(extra_dirs);
    let tick = Duration::from_millis(200);
    let mut last_tick = Instant::now();

    loop {
        terminal.draw(|f| ui::draw(f, &app))?;

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
                    if !matches!(app.mode, AppMode::ConfirmDelete { .. }) {
                        app.status_message = None;
                    }
                    handle_key(&mut app, key.code, key.modifiers, terminal)?;
                }
                Event::Mouse(me) => {
                    handle_mouse(&mut app, me, terminal)?;
                }
                _ => {}
            }
        }
        if last_tick.elapsed() >= tick { last_tick = Instant::now(); }

        if app.mode == AppMode::Normal && app.status_message.as_deref() == Some("__QUIT__") {
            break;
        }
    }
    Ok(())
}
