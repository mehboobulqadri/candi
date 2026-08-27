// SPDX-License-Identifier: AGPL-3.0

//! Error taxonomy for theme loading.
//!
//! Messages are for humans only — never matched on (same convention as
//! `candi-pdf`). Code branches on the kind via `matches!` or `match`.

use std::fmt;

/// Errors produced by parsing theme documents.
#[derive(Debug, Clone, PartialEq)]
pub enum ThemeError {
    /// The input is not well-formed YAML.
    Yaml(String),
    /// Well-formed YAML that violates the theme schema: unknown key,
    /// missing `name`, wrong type, or an invalid hex color.
    Schema(String),
}

impl fmt::Display for ThemeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ThemeError::Yaml(m) => write!(f, "invalid YAML: {m}"),
            ThemeError::Schema(m) => write!(f, "invalid theme: {m}"),
        }
    }
}

impl std::error::Error for ThemeError {}
