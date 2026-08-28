// SPDX-License-Identifier: AGPL-3.0

//! Built-in themes.
//!
//! Each theme is an embedded YAML document parsed through the same [`parse`]
//! path as user files, so built-ins can never drift from the schema.

use crate::parse;
use crate::theme::Theme;

/// Names of the built-in themes, in cycling order.
pub const BUILTIN_NAMES: [&str; 11] = [
    "Light",
    "Sepia",
    "Solarized Light",
    "Warm Dark",
    "Cyberpunk",
    "Catppuccin",
    "Nord",
    "Dracula",
    "Gruvbox Dark",
    "Dark",
    "True Dark",
];

/// Look up a built-in theme by name (case-sensitive, as in [`BUILTIN_NAMES`]).
///
/// # Panics
///
/// Never for shipped data — but a malformed embedded YAML is a programmer
/// error, so it panics instead of returning a broken theme.
pub fn builtin(name: &str) -> Option<Theme> {
    let src = match name {
        "Light" => include_str!("themes/light.yaml"),
        "Sepia" => include_str!("themes/sepia.yaml"),
        "Solarized Light" => include_str!("themes/solarized-light.yaml"),
        "Warm Dark" => include_str!("themes/warm-dark.yaml"),
        "Cyberpunk" => include_str!("themes/cyberpunk.yaml"),
        "Catppuccin" => include_str!("themes/catppuccin.yaml"),
        "Nord" => include_str!("themes/nord.yaml"),
        "Dracula" => include_str!("themes/dracula.yaml"),
        "Gruvbox Dark" => include_str!("themes/gruvbox-dark.yaml"),
        "Dark" => include_str!("themes/dark.yaml"),
        "True Dark" => include_str!("themes/true-dark.yaml"),
        _ => return None,
    };
    Some(parse(src).unwrap_or_else(|e| panic!("built-in theme {name:?} does not parse: {e}")))
}
