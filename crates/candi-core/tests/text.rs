// SPDX-License-Identifier: AGPL-3.0

use candi_core::normalize_reader_text;

#[test]
fn ligatures_become_ascii() {
    assert_eq!(normalize_reader_text("o\u{fb01}ce"), "ofice");
    assert_eq!(normalize_reader_text("\u{fb02}ow"), "flow");
    assert_eq!(normalize_reader_text("\u{fb03}n"), "ffin");
}

#[test]
fn plain_text_unchanged() {
    assert_eq!(normalize_reader_text("hello"), "hello");
}
