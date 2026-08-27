// SPDX-License-Identifier: AGPL-3.0

//! Theme schema and page-recoloring for Candi frontends.
//!
//! [`Theme`] is a strict YAML document (`deny_unknown_fields`; every field
//! except `name` defaults to the Light palette). Built-in themes are embedded
//! as YAML strings and go through exactly the same [`parse`] path as user
//! files. [`recolor`] maps page bitmaps onto a theme's page colors with pure
//! integer math — no UI or PDF types.

mod builtin;
mod color;
mod error;
mod recolor;
mod retint;
mod theme;

pub use builtin::{BUILTIN_NAMES, builtin};
pub use color::Color;
pub use error::ThemeError;
pub use recolor::recolor;
pub use retint::retint;
pub use theme::{Theme, parse, to_yaml};
