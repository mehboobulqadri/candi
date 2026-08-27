// SPDX-License-Identifier: AGPL-3.0

//! Reading-mode keybindings per project.md §6.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Quit,
    ScrollDown,
    ScrollUp,
    NextPage,
    PrevPage,
    FirstPage,
    LastPage,
    EnterSearch,
    SearchNext,
    SearchPrev,
    None,
}

pub fn map_reading_key(key: KeyEvent) -> Action {
    if key.modifiers.contains(KeyModifiers::CONTROL) || key.modifiers.contains(KeyModifiers::ALT) {
        return Action::None;
    }

    match key.code {
        KeyCode::Char('q') => Action::Quit,
        KeyCode::Char('j') | KeyCode::Down => Action::ScrollDown,
        KeyCode::Char('k') | KeyCode::Up => Action::ScrollUp,
        KeyCode::Char('h') | KeyCode::Left => Action::PrevPage,
        KeyCode::Char('l') | KeyCode::Right => Action::NextPage,
        KeyCode::Char('g') => Action::FirstPage,
        KeyCode::Char('G') => Action::LastPage,
        KeyCode::Char('/') => Action::EnterSearch,
        KeyCode::Char('n') => Action::SearchNext,
        KeyCode::Char('N') => Action::SearchPrev,
        _ => Action::None,
    }
}
