// SPDX-License-Identifier: AGPL-3.0

//! Search prompt editing (search mode only).
//!
//! Printable characters append to the draft, Backspace deletes the last
//! character, Enter submits a new [`candi_core::SearchSession`] from the
//! current page, and Esc cancels back to reading without searching.
//! `n` / `N` operate only in reading mode with an active session.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SearchInput {
    Append(char),
    Backspace,
    Submit,
    Cancel,
}

pub fn map_search_key(key: KeyEvent) -> Option<SearchInput> {
    match key.code {
        KeyCode::Esc => Some(SearchInput::Cancel),
        KeyCode::Enter => Some(SearchInput::Submit),
        KeyCode::Backspace => Some(SearchInput::Backspace),
        KeyCode::Char(c) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
            Some(SearchInput::Append(c))
        }
        _ => None,
    }
}
