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
    Appearance,
}

/// One visible contents row; `page` is 0-based and `dest_top` is the
/// destination's landing height in points from the page's top edge, when
/// the outline entry carries one.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct TocRow {
    pub title: String,
    pub page: usize,
    pub dest_top: Option<f32>,
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
            dest_top: item.dest_top,
            depth,
        });
        walk(&item.children, depth + 1, rows);
    }
}

/// Row index of the single deepest outline entry containing the reading
/// position `pos = (page, height-in-points within the page)`. An entry spans
/// from its landing point to the landing point of the next entry at its own
/// depth or shallower; past that it is superseded. Entries without a landing
/// point start at the page top and never supersede a same-page predecessor
/// (earlier sibling wins). Ancestors of the reading position stay unaccented
/// so stale siblings never light up together with the live entry. `None`
/// when the position precedes the first heading.
pub(crate) fn active_toc_row(rows: &[TocRow], pos: Option<(usize, f32)>) -> Option<usize> {
    let (page, y) = pos?;
    // A row begins once the reading position reaches its page (page-top for
    // entries without a landing point) or drops to its landing height.
    let started = |row: &TocRow| {
        row.page < page || (row.page == page && row.dest_top.is_none_or(|top| top <= y))
    };
    // A row supersedes its predecessor at the same depth once its landing
    // point is reached; without a landing point it only supersedes on a
    // later page, so same-page siblings without a y never skip ahead.
    let supersedes = |row: &TocRow| {
        row.page < page || (row.page == page && row.dest_top.is_some_and(|top| top <= y))
    };
    let mut chain: Vec<Option<usize>> = Vec::new();
    for (idx, row) in rows.iter().enumerate() {
        if !started(row) {
            break;
        }
        if chain.len() <= row.depth as usize {
            chain.resize(row.depth as usize + 1, None);
        }
        let slot = &mut chain[row.depth as usize];
        let same_page_stale =
            slot.is_some_and(|prev| rows[prev].page == row.page) && !supersedes(row);
        if !same_page_stale {
            *slot = Some(idx);
        }
    }
    chain
        .into_iter()
        .flatten()
        .filter(|&idx| {
            let depth = rows[idx].depth;
            rows[idx + 1..]
                .iter()
                .find(|row| row.depth <= depth)
                .is_none_or(|bound| !supersedes(bound))
        })
        .max_by_key(|&idx| rows[idx].depth)
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
        toc_at(title, page, None, children)
    }

    fn toc_at(title: &str, page: usize, dest_top: Option<f32>, children: Vec<TocItem>) -> TocItem {
        TocItem {
            title: title.to_owned(),
            page,
            dest_top,
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
                    dest_top: None,
                    depth: 0
                },
                TocRow {
                    title: "Part I".into(),
                    page: 1,
                    dest_top: None,
                    depth: 0
                },
                TocRow {
                    title: "Chapter 1".into(),
                    page: 2,
                    dest_top: None,
                    depth: 1
                },
                TocRow {
                    title: "Part II".into(),
                    page: 4,
                    dest_top: None,
                    depth: 0
                },
                TocRow {
                    title: "Appendix".into(),
                    page: 6,
                    dest_top: None,
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
    fn active_toc_row_is_the_deepest_containing_section() {
        let rows = flatten_toc(&[
            toc(
                "Part I",
                1,
                vec![toc("1. Intro", 2, vec![]), toc("2. Data", 5, vec![])],
            ),
            toc("Part II", 9, vec![]),
        ]);
        // Reading page 6 (0-based 5): inside 2. Data; Part I and the
        // superseded 1. Intro stay unaccented.
        assert_eq!(active_toc_row(&rows, Some((5, 300.0))), Some(2));
        // Past every heading: only Part II — the already-ended 2. Data never
        // outshines it.
        assert_eq!(active_toc_row(&rows, Some((100, 300.0))), Some(3));
        // Between headings the deeper sibling keeps the accent.
        assert_eq!(active_toc_row(&rows, Some((3, 300.0))), Some(1));
    }

    #[test]
    fn active_toc_row_is_none_before_the_first_heading() {
        let rows = flatten_toc(&[toc("Chapter 1", 4, vec![])]);
        assert_eq!(active_toc_row(&rows, Some((0, 300.0))), None);
    }

    #[test]
    fn same_page_siblings_resolve_by_destination_height() {
        // Two depth-1 entries starting on the same page with distinct
        // landing points: the accent follows the reading depth.
        let rows = flatten_toc(&[toc(
            "Cover",
            1,
            vec![
                toc_at("Left page", 1, Some(50.0), vec![]),
                toc_at("Right page", 1, Some(400.0), vec![]),
            ],
        )]);
        assert_eq!(
            active_toc_row(&rows, Some((0, 100.0))),
            Some(1),
            "above the later heading the earlier one is active"
        );
        assert_eq!(
            active_toc_row(&rows, Some((0, 450.0))),
            Some(2),
            "past its landing point the later heading takes over"
        );
        // Siblings without landing points keep the earlier entry active so a
        // duplicate-page outline never skips ahead.
        let plain = flatten_toc(&[toc(
            "Cover",
            1,
            vec![toc("Left page", 1, vec![]), toc("Right page", 1, vec![])],
        )]);
        assert_eq!(active_toc_row(&plain, Some((0, 300.0))), Some(1));
    }

    #[test]
    fn a_landing_point_supersedes_the_same_page_predecessor() {
        // The regression case: two top-level sections starting on the same
        // page. Reading between their landing points accents the earlier
        // one even though the later sibling already started by page number.
        let rows = flatten_toc(&[
            toc("Training", 7, vec![]),
            toc_at("Regression", 7, Some(550.0), vec![]),
            toc("Results", 9, vec![]),
        ]);
        assert_eq!(active_toc_row(&rows, Some((6, 300.0))), Some(0));
        assert_eq!(active_toc_row(&rows, Some((6, 600.0))), Some(1));
        // The later sibling's span carries into the following pages.
        assert_eq!(active_toc_row(&rows, Some((7, 100.0))), Some(1));

        // When the later sibling instead lands mid-page on the NEXT page,
        // the predecessor keeps the accent until its landing point.
        let rows = flatten_toc(&[
            toc("Training", 7, vec![]),
            toc_at("Regression", 8, Some(550.0), vec![]),
            toc("Results", 10, vec![]),
        ]);
        assert_eq!(
            active_toc_row(&rows, Some((7, 100.0))),
            Some(0),
            "above the landing on its start page"
        );
        assert_eq!(active_toc_row(&rows, Some((7, 600.0))), Some(1));
    }

    #[test]
    fn active_toc_row_climbs_back_when_deep_sections_end() {
        let rows = flatten_toc(&[toc(
            "Part I",
            1,
            vec![
                toc(
                    "Ch. 1",
                    2,
                    vec![toc("1.1", 3, vec![]), toc("1.2", 5, vec![])],
                ),
                toc("Ch. 2", 7, vec![]),
            ],
        )]);
        assert_eq!(active_toc_row(&rows, Some((3, 300.0))), Some(2));
        assert_eq!(active_toc_row(&rows, Some((5, 300.0))), Some(3));
        // Inside Ch. 2 the finished 1.x sections are no longer accentable;
        // the highlight climbs back to the chapter that is actually open.
        assert_eq!(active_toc_row(&rows, Some((8, 300.0))), Some(4));
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
