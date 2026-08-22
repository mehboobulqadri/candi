// SPDX-License-Identifier: AGPL-3.0

//! Sidebar data model: flattened table-of-contents rows, search-result rows,
//! and row display helpers. Pure logic, no egui — rendering lives in
//! [`crate::app`].

use candi_pdf::TocItem;

/// Which sidebar panel is active.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum SidebarSection {
    Contents,
    Bookmarks,
    Search,
}

/// One visible contents row; `page` is 0-based.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TocRow {
    pub title: String,
    pub page: usize,
    pub depth: u32,
}

/// Depth-first flattening of an outline tree; one indentation level per
/// nesting depth.
pub(crate) fn flatten_toc(items: &[TocItem]) -> Vec<TocRow> {
    let mut rows = Vec::new();
    walk(items, 0, &mut rows);
    rows
}

fn walk(items: &[TocItem], depth: u32, rows: &mut Vec<TocRow>) {
    for item in items {
        rows.push(TocRow {
            title: item.title.clone(),
            page: item.page - 1,
            depth,
        });
        walk(&item.children, depth + 1, rows);
    }
}

/// One search result: the page it points at plus an excerpt of the page's
/// normalized text around the match.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SearchHit {
    pub page: usize,
    pub snippet: String,
}

/// Target excerpt length around a match, in bytes.
const SNIPPET_CHARS: usize = 40;

/// Excerpt of at most ~[`SNIPPET_CHARS`] characters centered on the match at
/// byte range `[offset, offset + needle_len)` of the normalized page text.
/// Whitespace collapses to single spaces and `…` marks cut-off text.
pub(crate) fn extract_snippet(text: &str, offset: usize, needle_len: usize) -> String {
    let end = (offset + needle_len).min(text.len());
    let ctx = SNIPPET_CHARS.saturating_sub(needle_len) / 2;
    let start = floor_boundary(text, offset.saturating_sub(ctx));
    let stop = ceil_boundary(text, (end + ctx).min(text.len()));

    let mut out = String::new();
    if start > 0 {
        out.push('…');
    }
    out.push_str(
        &text[start..stop]
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" "),
    );
    if stop < text.len() {
        out.push('…');
    }
    out
}

fn floor_boundary(text: &str, mut i: usize) -> usize {
    while !text.is_char_boundary(i) {
        i -= 1;
    }
    i
}

fn ceil_boundary(text: &str, mut i: usize) -> usize {
    while !text.is_char_boundary(i) {
        i += 1;
    }
    i
}

/// Date part (`YYYY-MM-DD`) of a stored RFC-3339 timestamp; falls back to the
/// raw string when it is too short to hold a date.
pub(crate) fn date_only(created_at: &str) -> &str {
    created_at.get(..10).unwrap_or(created_at)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn toc(title: &str, page: usize, children: Vec<TocItem>) -> TocItem {
        TocItem {
            title: title.to_owned(),
            page,
            children,
        }
    }

    #[test]
    fn flatten_orders_depth_first_with_zero_based_pages_and_indent() {
        let items = vec![
            toc("Intro", 1, Vec::new()),
            toc("Part I", 2, vec![toc("Chapter 1", 3, Vec::new())]),
            toc("Part II", 5, vec![toc("Appendix", 7, Vec::new())]),
        ];
        assert_eq!(
            flatten_toc(&items),
            vec![
                TocRow {
                    title: "Intro".into(),
                    page: 0,
                    depth: 0
                },
                TocRow {
                    title: "Part I".into(),
                    page: 1,
                    depth: 0
                },
                TocRow {
                    title: "Chapter 1".into(),
                    page: 2,
                    depth: 1
                },
                TocRow {
                    title: "Part II".into(),
                    page: 4,
                    depth: 0
                },
                TocRow {
                    title: "Appendix".into(),
                    page: 6,
                    depth: 1
                },
            ]
        );
    }

    #[test]
    fn flatten_of_an_empty_outline_is_empty() {
        assert!(flatten_toc(&[]).is_empty());
    }

    #[test]
    fn long_text_snippet_is_centered_on_the_match_and_cut_with_ellipses() {
        let text = "lorem ipsum dolor sit amet consectetur adipiscing elit sed do eiusmod tempor";
        let needle = "consectetur";
        let offset = text.find(needle).unwrap();
        let snippet = extract_snippet(text, offset, needle.len());
        assert!(
            snippet.starts_with('…') && snippet.ends_with('…'),
            "{snippet}"
        );
        assert!(snippet.contains(needle), "{snippet}");
        assert!(snippet.chars().count() <= SNIPPET_CHARS + 2, "{snippet}");
    }

    #[test]
    fn short_text_yields_one_ellipsis_free_snippet() {
        assert_eq!(extract_snippet("hello world", 6, 5), "hello world");
    }

    #[test]
    fn match_at_the_start_has_no_leading_ellipsis() {
        let text = "start middle end with plenty of trailing padding text here";
        assert_eq!(extract_snippet(text, 0, 5), "start middle end with…");
    }

    #[test]
    fn match_at_the_end_has_no_trailing_ellipsis() {
        let text = "plenty of leading padding text here before the very final words";
        let offset = text.find("final").unwrap();
        assert_eq!(
            extract_snippet(text, offset, 5),
            "…before the very final words"
        );
    }

    #[test]
    fn snippet_collapses_whitespace_runs() {
        let text = "alpha   beta\ngamma\tdelta";
        let offset = text.find("gamma").unwrap();
        assert_eq!(extract_snippet(text, offset, 5), "alpha beta gamma delta");
    }

    #[test]
    fn snippet_cuts_on_character_boundaries_not_inside_code_points() {
        let text = "ωωωωωω ".repeat(10) + "target " + &"ωωωωωω ".repeat(10);
        let offset = text.find("target").unwrap();
        let snippet = extract_snippet(&text, offset, 6);
        assert!(
            snippet.starts_with('…') && snippet.ends_with('…'),
            "{snippet}"
        );
        assert!(snippet.contains("target"), "{snippet}");
    }

    #[test]
    fn needle_longer_than_the_budget_still_appears_in_full() {
        let needle = "a very long query that exceeds the snippet budget on its own";
        let text = format!("prefix {needle} suffix");
        let offset = text.find(needle).unwrap();
        let snippet = extract_snippet(&text, offset, needle.len());
        assert!(snippet.contains(needle));
        assert_eq!(snippet.matches('…').count(), 2);
    }

    #[test]
    fn date_only_takes_the_calendar_day_from_rfc3339() {
        assert_eq!(date_only("2026-08-22T10:30:00Z"), "2026-08-22");
    }

    #[test]
    fn date_only_falls_back_to_the_raw_string_when_too_short() {
        assert_eq!(date_only("unknown"), "unknown");
    }
}
