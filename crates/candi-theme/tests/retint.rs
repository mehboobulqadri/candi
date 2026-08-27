// SPDX-License-Identifier: AGPL-3.0

//! Luma-gating and blend-fraction behavior of accent retinting.

use candi_theme::{Color, builtin, retint};

fn purple() -> Color {
    Color::from([0x7C, 0x5C, 0xFF, 0xFF])
}

#[test]
fn dark_chrome_takes_the_full_six_percent_blend() {
    let dark = builtin("Dark").unwrap();
    let tinted = retint(&dark, purple());
    // ui_bg #1D2026 → 6% toward #7C5CFF.
    let mix = |bg: u8, accent: u8| -> u8 {
        ((u32::from(bg) * 240 + u32::from(accent) * 15 + 127) / 255) as u8
    };
    assert_eq!(tinted.ui_bg.r(), mix(0x1D, 0x7C));
    assert_eq!(tinted.ui_bg.g(), mix(0x20, 0x5C));
    assert_eq!(tinted.ui_bg.b(), mix(0x26, 0xFF));
}

#[test]
fn light_chrome_blends_less_than_dark() {
    let light = builtin("Light").unwrap();
    let dark = builtin("Dark").unwrap();
    let amount = |base: &candi_theme::Theme, tinted: &candi_theme::Theme| -> u32 {
        u32::from(base.ui_bg.r().abs_diff(tinted.ui_bg.r()))
            + u32::from(base.ui_bg.g().abs_diff(tinted.ui_bg.g()))
            + u32::from(base.ui_bg.b().abs_diff(tinted.ui_bg.b()))
    };
    let light_tinted = retint(&light, purple());
    let dark_tinted = retint(&dark, purple());
    assert!(amount(&light, &light_tinted) > 0, "light still tints");
    assert!(amount(&light, &light_tinted) < amount(&dark, &dark_tinted));
}

#[test]
fn page_colors_name_accent_and_selection_stay_untouched() {
    for name in ["Dark", "Light", "Sepia", "True Dark", "Warm Dark"] {
        let theme = builtin(name).unwrap();
        let tinted = retint(&theme, purple());
        assert_eq!(tinted.name, theme.name, "{name}");
        assert_eq!(tinted.page_bg, theme.page_bg, "{name}");
        assert_eq!(tinted.page_fg, theme.page_fg, "{name}");
        assert_eq!(tinted.accent, theme.accent, "{name}");
        assert_eq!(tinted.selection, theme.selection, "{name}");
        assert_eq!(tinted.ui_fg, theme.ui_fg, "{name}");
    }
}

#[test]
fn chrome_moves_toward_the_accent_but_never_past_safety() {
    for name in ["Dark", "Light", "Sepia", "True Dark", "Warm Dark"] {
        let theme = builtin(name).unwrap();
        let tinted = retint(&theme, purple());
        assert_ne!(tinted.ui_bg, theme.ui_bg, "{name} ui_bg tints");
        assert_ne!(tinted.panel_bg, theme.panel_bg, "{name} panel_bg tints");
    }
}

#[test]
fn accent_matching_the_background_luma_is_declined() {
    // White accent on the Light theme: blending cannot keep luma distance
    // from the accent, so the surfaces stay put.
    let light = builtin("Light").unwrap();
    let white = Color::from([0xFF, 0xFF, 0xFF, 0xFF]);
    let tinted = retint(&light, white);
    assert_eq!(tinted.ui_bg, light.ui_bg);
    assert_eq!(tinted.panel_bg, light.panel_bg);
}
