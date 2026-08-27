// SPDX-License-Identifier: AGPL-3.0

/// Normalize PDF presentation forms and punctuation for on-screen reading and search.
///
/// Ligatures expand to ASCII letters; NBSP becomes space; soft hyphens are removed;
/// curly quotes and dashes map to ASCII equivalents.
pub fn normalize_reader_text(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for ch in text.chars() {
        match ch {
            '\u{00a0}' => out.push(' '),
            '\u{00ad}' => {}
            '\u{2018}' | '\u{2019}' => out.push('\''),
            '\u{201c}' | '\u{201d}' => out.push('"'),
            '\u{2013}' | '\u{2014}' => out.push('-'),
            '\u{fb00}' => out.push_str("ff"),
            '\u{fb01}' => out.push_str("fi"),
            '\u{fb02}' => out.push_str("fl"),
            '\u{fb03}' => out.push_str("ffi"),
            '\u{fb04}' => out.push_str("ffl"),
            other => out.push(other),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ligature_fi_maps_to_finger() {
        assert_eq!(normalize_reader_text("\u{fb01}nger"), "finger");
    }

    #[test]
    fn common_ligatures_and_punctuation() {
        assert_eq!(normalize_reader_text("\u{fb02}ow"), "flow");
        assert_eq!(normalize_reader_text("\u{fb00}ee"), "ffee");
        assert_eq!(normalize_reader_text("\u{fb03}x"), "ffix");
        assert_eq!(normalize_reader_text("\u{fb04}y"), "ffly");
        assert_eq!(normalize_reader_text("a\u{00a0}b"), "a b");
        assert_eq!(normalize_reader_text("co\u{00ad}operate"), "cooperate");
        assert_eq!(normalize_reader_text("\u{2018}hi\u{2019}"), "'hi'");
        assert_eq!(normalize_reader_text("\u{201c}hi\u{201d}"), "\"hi\"");
        assert_eq!(normalize_reader_text("a\u{2013}b\u{2014}c"), "a-b-c");
    }
}
