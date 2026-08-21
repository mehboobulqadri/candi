// SPDX-License-Identifier: AGPL-3.0

//! Built-in themes.
//!
//! Each theme is an embedded YAML document parsed through the same [`parse`]
//! path as user files, so built-ins can never drift from the schema.

use crate::parse;
use crate::theme::Theme;

/// Names of the built-in themes, in cycling order.
pub const BUILTIN_NAMES: [&str; 5] = ["Light", "Sepia", "Warm Dark", "Dark", "True Dark"];

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
        "Warm Dark" => include_str!("themes/warm-dark.yaml"),
        "Dark" => include_str!("themes/dark.yaml"),
        "True Dark" => include_str!("themes/true-dark.yaml"),
        _ => return None,
    };
    Some(parse(src).unwrap_or_else(|e| panic!("built-in theme {name:?} does not parse: {e}")))
}
