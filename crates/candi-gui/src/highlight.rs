// SPDX-License-Identifier: AGPL-3.0

//! Syntax highlighting for the center-pane theme editor.
//!
//! Tokenization comes from [`egui_code_editor`]'s lexer; coloring stays here
//! so the palette can follow the live theme without static hex strings.

use candi_theme::Theme;
use eframe::egui;
use egui::FontId;
use egui::text::{LayoutJob, TextFormat};
use egui_code_editor::{Syntax, Token, TokenType};

/// Editor font size, matching the previous plain code editor look.
const FONT_SIZE: f32 = 13.0;

/// Every schema key the editor accepts; keys are the accent-colored tokens.
const SCHEMA_KEYS: [&str; 8] = [
    "name",
    "page_bg",
    "page_fg",
    "ui_bg",
    "panel_bg",
    "ui_fg",
    "accent",
    "selection",
];

/// YAML dialect for theme buffers: `#` comments and the exact schema keys.
pub(crate) fn yaml_syntax() -> Syntax {
    Syntax::simple("#")
        .with_case_sensitive(true)
        .with_keywords(SCHEMA_KEYS)
}

/// Highlight `text` with the theme's palette: schema keys and numbers take
/// the accent, comments fall back to a muted foreground, everything else to
/// the plain foreground.
pub(crate) fn yaml_job(text: &str, theme: &Theme) -> LayoutJob {
    let accent = color_of(theme.accent);
    let fg = color_of(theme.ui_fg);
    let muted = fg.gamma_multiply(0.55);
    let font = FontId::monospace(FONT_SIZE);

    let mut job = LayoutJob::default();
    for token in Token::default().tokens(&yaml_syntax(), text) {
        let color = match token.ty() {
            TokenType::Keyword | TokenType::Type | TokenType::Special | TokenType::Function => {
                accent
            }
            TokenType::Comment(_) => muted,
            _ => fg,
        };
        job.append(token.buffer(), 0.0, TextFormat::simple(font.clone(), color));
    }
    job
}

fn color_of(color: candi_theme::Color) -> egui::Color32 {
    egui::Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), color.a())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kinds(text: &str) -> Vec<(TokenType, String)> {
        Token::default()
            .tokens(&yaml_syntax(), text)
            .into_iter()
            .map(|token| (token.ty(), token.buffer().to_owned()))
            .collect()
    }

    #[test]
    fn schema_keys_classify_as_keywords() {
        let tokens = kinds("accent:");
        assert!(
            tokens.contains(&(TokenType::Keyword, "accent".into())),
            "{tokens:?}"
        );
    }

    #[test]
    fn unknown_words_stay_plain_literals() {
        let tokens = kinds("Light");
        assert_eq!(tokens[0].0, TokenType::Literal);
    }

    #[test]
    fn quoted_values_classify_as_strings() {
        let tokens = kinds("\"#101010\"");
        assert_eq!(
            tokens,
            vec![(TokenType::Str('"'), "\"#101010\"".to_owned())]
        );
    }

    #[test]
    fn hash_starts_a_comment_to_end_of_line() {
        let tokens = kinds("# a note\nname:");
        assert!(
            tokens.contains(&(TokenType::Comment(false), "# a note".into())),
            "{tokens:?}"
        );
        assert!(
            tokens.contains(&(TokenType::Keyword, "name".into())),
            "{tokens:?}"
        );
    }

    #[test]
    fn hex_digits_inside_a_string_are_not_numerics() {
        assert!(
            !kinds("\"#AABBCC\"")
                .iter()
                .any(|t| matches!(t.0, TokenType::Numeric(_)))
        );
    }

    #[test]
    fn case_sensitivity_is_preserved_for_yaml() {
        assert!(!kinds("ACCENT:").iter().any(|t| t.0 == TokenType::Keyword));
    }
}
