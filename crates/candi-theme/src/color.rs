// SPDX-License-Identifier: AGPL-3.0

//! RGBA color token with hex serialization.
//!
//! YAML form is `#RRGGBB` (alpha defaults to 255) or `#RRGGBBAA`; [`Display`]
//! emits the same shape, omitting the alpha pair when it is 255, so
//! formatting and parsing round-trip.

use std::fmt;

use serde::de::{Deserializer, Error, Visitor};

/// An opaque sRGBA color stored as `[r, g, b, a]` bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Color([u8; 4]);

impl Color {
    /// Red channel.
    pub fn r(self) -> u8 {
        self.0[0]
    }

    /// Green channel.
    pub fn g(self) -> u8 {
        self.0[1]
    }

    /// Blue channel.
    pub fn b(self) -> u8 {
        self.0[2]
    }

    /// Alpha channel.
    pub fn a(self) -> u8 {
        self.0[3]
    }

    pub(crate) fn from_hex(s: &str) -> Option<Self> {
        let hex = s.strip_prefix('#')?;
        if !hex.is_ascii() {
            return None;
        }
        let byte = |i: usize| u8::from_str_radix(&hex[i * 2..i * 2 + 2], 16).ok();
        match hex.len() {
            6 => Some(Self([byte(0)?, byte(1)?, byte(2)?, 255])),
            8 => Some(Self([byte(0)?, byte(1)?, byte(2)?, byte(3)?])),
            _ => None,
        }
    }
}

impl From<[u8; 4]> for Color {
    fn from(rgba: [u8; 4]) -> Self {
        Self(rgba)
    }
}

impl fmt::Display for Color {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let [r, g, b, a] = self.0;
        if a == 255 {
            write!(f, "#{r:02X}{g:02X}{b:02X}")
        } else {
            write!(f, "#{r:02X}{g:02X}{b:02X}{a:02X}")
        }
    }
}

impl<'de> serde::Deserialize<'de> for Color {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct HexVisitor;

        impl Visitor<'_> for HexVisitor {
            type Value = Color;

            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("a hex color like #RRGGBB or #RRGGBBAA")
            }

            fn visit_str<E: Error>(self, v: &str) -> Result<Color, E> {
                Color::from_hex(v).ok_or_else(|| E::custom(format!("invalid hex color {v:?}")))
            }
        }

        deserializer.deserialize_str(HexVisitor)
    }
}
