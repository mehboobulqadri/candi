// SPDX-License-Identifier: AGPL-3.0

//! Core document-view logic shared by Candi frontends.
//!
//! Navigation state (`ViewState`) tracks the current page and scroll offset with
//! clamping bounds supplied by the caller — no UI or PDF types. [`SearchSession`]
//! performs lazy, page-at-a-time text search over any [`candi_pdf::Document`].

mod navigation;
mod search;
mod state;
mod text;

pub use navigation::ViewState;
pub use search::SearchSession;
pub use state::{Error, Load, Position, load, save, sidecar_path};
pub use text::normalize_reader_text;
