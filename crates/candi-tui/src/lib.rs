// SPDX-License-Identifier: AGPL-3.0

//! Keyboard-first terminal reader for Candi.
//!
//! Reading-mode keys follow project.md §6. Search prompt mode accepts printable
//! input, Backspace, Enter (starts a [`candi_core::SearchSession`] on the
//! current page), and Esc (cancel without searching). `n` / `N` jump results
//! only while reading with an active session.

mod app;
mod keymap;
mod search;
mod ui;

use std::io::{self, stdout};
use std::time::Duration;

use candi_core::ViewState;
use candi_pdf::Document;
use crossterm::event::{
    self, DisableMouseCapture, EnableMouseCapture, Event, KeyEvent, MouseEventKind,
};
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use crossterm::{ExecutableCommand, terminal::ClearType};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;

pub use app::{App, Mode};
pub use keymap::Action;
pub use ui::draw;

/// Returned from [`handle_key`] when the user presses `q`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Quit;

/// Terminal or I/O failure while running the interactive loop.
#[derive(Debug)]
pub enum RunError {
    Io(io::Error),
    Terminal(String),
}

impl From<io::Error> for RunError {
    fn from(err: io::Error) -> Self {
        Self::Io(err)
    }
}

impl std::fmt::Display for RunError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(err) => write!(f, "{err}"),
            Self::Terminal(msg) => write!(f, "{msg}"),
        }
    }
}

impl std::error::Error for RunError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(err) => Some(err),
            Self::Terminal(_) => None,
        }
    }
}

struct TerminalGuard;

impl TerminalGuard {
    fn new() -> io::Result<Self> {
        enable_raw_mode()?;
        Ok(Self)
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let mut stdout = io::stdout();
        let _ = stdout.execute(DisableMouseCapture);
        let _ = disable_raw_mode();
        let _ = stdout.execute(LeaveAlternateScreen);
        let _ = stdout.execute(crossterm::cursor::Show);
        let _ = stdout.execute(crossterm::terminal::Clear(ClearType::All));
    }
}

/// Apply one key event. Returns [`Quit`] when the user presses `q`.
pub fn handle_key<D: Document + ?Sized>(
    app: &mut App<'_, D>,
    key: KeyEvent,
) -> Result<Option<Quit>, candi_pdf::Error> {
    if let Mode::Searching { .. } = app.mode() {
        if let Some(input) = search::map_search_key(key) {
            app.apply_search_input(input)?;
        }
        return Ok(None);
    }

    let action = keymap::map_reading_key(key);
    if action == Action::Quit {
        app.apply_action(action)?;
        return Ok(Some(Quit));
    }
    app.apply_action(action)?;
    Ok(None)
}

/// Run the interactive TUI over `document`. Returns the final view on quit, or
/// an error for unsupported terminals (including `TERM=dumb`) or I/O failures;
/// never busy-loops.
pub fn run(
    document: Box<dyn Document>,
    filename: &str,
    initial: ViewState,
) -> Result<ViewState, RunError> {
    if std::env::var("TERM").as_deref() == Ok("dumb") {
        return Err(RunError::Terminal(
            "TERM=dumb: interactive terminal required".into(),
        ));
    }

    let _guard = TerminalGuard::new()?;
    let mut stdout = stdout();
    stdout
        .execute(EnterAlternateScreen)?
        .execute(crossterm::cursor::Hide)?
        .execute(EnableMouseCapture)?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend).map_err(RunError::Io)?;

    let mut app = App::new(document.as_ref(), filename, initial);
    let size = terminal.size().map_err(RunError::Io)?;
    app.resize(size.width, size.height);
    let _ = app.reload_page();

    run_loop(&mut terminal, &mut app)?;
    Ok(app.view())
}

fn run_loop(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App<'_, dyn Document>,
) -> Result<(), RunError> {
    loop {
        terminal
            .draw(|frame| draw(frame, app))
            .map_err(RunError::Io)?;

        if app.should_quit() {
            break;
        }

        if event::poll(Duration::from_millis(250)).map_err(RunError::Io)? {
            match event::read().map_err(RunError::Io)? {
                Event::Key(key) => {
                    if let Err(err) = handle_key(app, key) {
                        app.set_error(err.to_string());
                    }
                }
                Event::Resize(width, height) => {
                    app.resize(width, height);
                }
                Event::Mouse(mouse) if matches!(app.mode(), Mode::Reading) => match mouse.kind {
                    MouseEventKind::ScrollDown => {
                        if let Err(err) = app.scroll_down() {
                            app.set_error(err.to_string());
                        }
                    }
                    MouseEventKind::ScrollUp => {
                        if let Err(err) = app.scroll_up() {
                            app.set_error(err.to_string());
                        }
                    }
                    _ => {}
                },
                _ => {}
            }
        }
    }
    Ok(())
}
