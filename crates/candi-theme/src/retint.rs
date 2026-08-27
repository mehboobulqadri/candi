// SPDX-License-Identifier: AGPL-3.0

//! Accent tinting for chrome surfaces.
//!
//! [`retint`] blends a small fraction of the accent into `panel_bg` and
//! `ui_bg` so a chosen accent seeps into the surrounding chrome, while the
//! page colors stay untouched. The blend factor is luma-gated against both
//! the accent and the foreground (integer Rec.601, same math as
//! [`crate::recolor`]) and backs off to zero if either contrast margin would
//! collapse, so readable themes can never be tinted into mud.

use crate::color::Color;
use crate::recolor::luma;
use crate::theme::Theme;

/// Blend numerator out of 255 for dark chrome (~12%).
const TINT_DARK: u32 = 30;
/// Blend numerator out of 255 for light chrome (~6%).
const TINT_LIGHT: u32 = 16;
/// Minimum luma the tinted surface must keep from the accent.
const MIN_ACCENT_GAP: i32 = 24;
/// Minimum luma the tinted surface must keep from the foreground.
const MIN_FG_GAP: i32 = 24;

/// Blend numerator out of 255 toward black for the canvas surround on dark
/// chrome (~20%) — it must read visibly darker than the sidebar/panel.
const CANVAS_DARK: u32 = 51;
/// The same blend on light chrome (~10%), enough for white pages to pop.
const CANVAS_LIGHT: u32 = 25;

/// The theme with `accent` blended into its chrome surfaces.
///
/// `page_bg`/`page_fg`, `selection`, `name`, and `accent` itself pass
/// through unchanged; `panel_bg` and `ui_bg` move a few percent toward the
/// accent — more on dark themes than light ones — unless that would crowd
/// the accent or the text luma, in which case the surface stays as-is.
pub fn retint(theme: &Theme, accent: Color) -> Theme {
    let mut tinted = theme.clone();
    let factor = if luma(theme.ui_bg.r(), theme.ui_bg.g(), theme.ui_bg.b()) < 128 {
        TINT_DARK
    } else {
        TINT_LIGHT
    };
    tinted.panel_bg = tint(theme.panel_bg, theme.ui_fg, accent, factor);
    tinted.ui_bg = tint(theme.ui_bg, theme.ui_fg, accent, factor);
    tinted
}

/// The darker surround the page canvas paints on: `panel_bg` pulled toward
/// black so pages and the sidebar (which keep `panel_bg`) both stand apart
/// from it. Dark chrome takes a stronger pull than light chrome.
pub fn canvas_bg(theme: &Theme) -> Color {
    let factor = if luma(theme.ui_bg.r(), theme.ui_bg.g(), theme.ui_bg.b()) < 128 {
        CANVAS_DARK
    } else {
        CANVAS_LIGHT
    };
    let mix = |channel: u8| -> u8 { ((u32::from(channel) * (255 - factor) + 127) / 255) as u8 };
    Color::from([
        mix(theme.panel_bg.r()),
        mix(theme.panel_bg.g()),
        mix(theme.panel_bg.b()),
        theme.panel_bg.a(),
    ])
}

/// Blend `factor`/255 of `accent` into `bg` when the result keeps safe luma
/// distance from both the accent and `fg`; otherwise return `bg`.
fn tint(bg: Color, fg: Color, accent: Color, factor: u32) -> Color {
    let mix = |a: u8, b: u8| -> u8 {
        ((u32::from(a) * (255 - factor) + u32::from(b) * factor + 127) / 255) as u8
    };
    let out = Color::from([
        mix(bg.r(), accent.r()),
        mix(bg.g(), accent.g()),
        mix(bg.b(), accent.b()),
        bg.a(),
    ]);
    let out_luma = i32::from(luma(out.r(), out.g(), out.b()));
    let accent_luma = i32::from(luma(accent.r(), accent.g(), accent.b()));
    let fg_luma = i32::from(luma(fg.r(), fg.g(), fg.b()));
    if (out_luma - accent_luma).abs() < MIN_ACCENT_GAP || (out_luma - fg_luma).abs() < MIN_FG_GAP {
        return bg;
    }
    out
}
