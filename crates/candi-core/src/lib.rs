// SPDX-License-Identifier: AGPL-3.0

//! Core document-view logic shared by Candi frontends.
//!
//! Navigation state (`ViewState`) tracks the current page and scroll offset with
//! clamping bounds supplied by the caller — no UI or PDF types.

mod navigation;

pub use navigation::ViewState;
