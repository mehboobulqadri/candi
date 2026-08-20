// SPDX-License-Identifier: AGPL-3.0

/// Replace common PDF ligatures with ASCII so extracted text reads cleanly.
pub fn normalize_reader_text(text: &str) -> String {
    text.replace('\u{fb00}', "ff")
        .replace('\u{fb01}', "fi")
        .replace('\u{fb02}', "fl")
        .replace('\u{fb03}', "ffi")
        .replace('\u{fb04}', "ffl")
        .replace(['\u{fb05}', '\u{fb06}'], "st")
}
