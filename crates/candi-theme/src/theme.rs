// SPDX-License-Identifier: AGPL-3.0

//! Theme document schema and the shared [`parse`] entry point.
//!
//! The schema is strict: unknown keys are rejected, keys are snake_case, and
//! every field except `name` defaults to the Light palette so a theme file can
//! override as little as a single token.

use serde::Deserialize;

use crate::color::Color;
use crate::error::ThemeError;

/// Semantic UI + page colors for a Candi frontend.
///
/// `page_bg`/`page_fg` drive bitmap recoloring ([`crate::recolor`]); the rest
/// style chrome.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Theme {
    pub name: String,
    #[serde(default = "light_page_bg")]
    pub page_bg: Color,
    #[serde(default = "light_page_fg")]
    pub page_fg: Color,
    #[serde(default = "light_ui_bg")]
    pub ui_bg: Color,
    #[serde(default = "light_panel_bg")]
    pub panel_bg: Color,
    #[serde(default = "light_ui_fg")]
    pub ui_fg: Color,
    #[serde(default = "light_accent")]
    pub accent: Color,
    #[serde(default = "light_selection")]
    pub selection: Color,
    #[serde(default = "light_search")]
    pub search: Color,
}

fn light_page_bg() -> Color {
    Color::from([0xFF, 0xFF, 0xFF, 0xFF])
}

fn light_page_fg() -> Color {
    Color::from([0x1A, 0x1A, 0x1A, 0xFF])
}

fn light_ui_bg() -> Color {
    Color::from([0xF5, 0xF5, 0xF4, 0xFF])
}

fn light_panel_bg() -> Color {
    Color::from([0xEC, 0xEC, 0xEB, 0xFF])
}

fn light_ui_fg() -> Color {
    Color::from([0x26, 0x26, 0x26, 0xFF])
}

fn light_accent() -> Color {
    Color::from([0x25, 0x63, 0xEB, 0xFF])
}

fn light_selection() -> Color {
    Color::from([0x25, 0x63, 0xEB, 0x40])
}

fn light_search() -> Color {
    Color::from([0xFB, 0xBF, 0x24, 0xFF])
}

/// Parse a theme document (user YAML or embedded built-in).
pub fn parse(input: &str) -> Result<Theme, ThemeError> {
    let value = serde_yaml::from_str(input).map_err(|e| ThemeError::Yaml(e.to_string()))?;
    serde_yaml::from_value(value).map_err(|e| ThemeError::Schema(e.to_string()))
}
