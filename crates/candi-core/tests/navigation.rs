// SPDX-License-Identifier: AGPL-3.0

use candi_core::ViewState;

#[test]
fn new_starts_at_first_page_with_zero_scroll() {
    let state = ViewState::new();
    assert_eq!(state.page(), 0);
    assert_eq!(state.scroll_offset(), 0);
}

#[test]
fn default_matches_new() {
    assert_eq!(ViewState::default(), ViewState::new());
}

#[test]
fn empty_document_page_navigation_is_no_op() {
    let state = ViewState::new()
        .next_page(0)
        .prev_page(0)
        .first_page(0)
        .last_page(0);
    assert_eq!(state.page(), 0);
    assert_eq!(state.scroll_offset(), 0);
}

#[test]
fn empty_document_scroll_is_no_op() {
    let state = ViewState::new().scroll_down(5, 0).scroll_up(5, 0);
    assert_eq!(state.scroll_offset(), 0);
}

#[test]
fn first_page_jumps_to_zero_and_resets_scroll() {
    let state = ViewState::new()
        .next_page(5)
        .scroll_down(3, 10)
        .first_page(5);
    assert_eq!(state.page(), 0);
    assert_eq!(state.scroll_offset(), 0);
}

#[test]
fn last_page_jumps_to_final_page_and_resets_scroll() {
    let state = ViewState::new().scroll_down(4, 20).last_page(5);
    assert_eq!(state.page(), 4);
    assert_eq!(state.scroll_offset(), 0);
}

#[test]
fn last_page_on_empty_document_stays_at_zero() {
    let state = ViewState::new().last_page(0);
    assert_eq!(state.page(), 0);
}

#[test]
fn next_page_advances_and_clamps_at_last() {
    let state = ViewState::new().next_page(3);
    assert_eq!(state.page(), 1);

    let state = state.next_page(3);
    assert_eq!(state.page(), 2);

    let state = state.next_page(3);
    assert_eq!(state.page(), 2);
}

#[test]
fn prev_page_retreats_and_clamps_at_first() {
    let state = ViewState::new().last_page(3);
    assert_eq!(state.page(), 2);

    let state = state.prev_page(3);
    assert_eq!(state.page(), 1);

    let state = state.prev_page(3).prev_page(3).prev_page(3);
    assert_eq!(state.page(), 0);
}

#[test]
fn single_page_next_and_prev_are_no_ops() {
    let state = ViewState::new().next_page(1).prev_page(1);
    assert_eq!(state.page(), 0);
}

#[test]
fn page_change_resets_scroll_offset() {
    let scrolled = ViewState::new().scroll_down(7, 20);
    assert_eq!(scrolled.scroll_offset(), 7);

    let next = scrolled.next_page(5);
    assert_eq!(next.page(), 1);
    assert_eq!(next.scroll_offset(), 0);

    let prev = next.scroll_down(4, 20).prev_page(5);
    assert_eq!(prev.page(), 0);
    assert_eq!(prev.scroll_offset(), 0);
}

#[test]
fn scroll_down_clamps_to_max_scroll() {
    let state = ViewState::new().scroll_down(3, 10);
    assert_eq!(state.scroll_offset(), 3);

    let state = state.scroll_down(100, 10);
    assert_eq!(state.scroll_offset(), 10);
}

#[test]
fn scroll_down_with_zero_max_scroll_stays_at_zero() {
    let state = ViewState::new().scroll_down(5, 0);
    assert_eq!(state.scroll_offset(), 0);
}

#[test]
fn scroll_up_clamps_at_zero() {
    let state = ViewState::new().scroll_down(5, 10).scroll_up(3, 10);
    assert_eq!(state.scroll_offset(), 2);

    let state = state.scroll_up(100, 10);
    assert_eq!(state.scroll_offset(), 0);
}

#[test]
fn scroll_up_clamps_when_max_scroll_shrinks() {
    let state = ViewState::new().scroll_down(8, 10).scroll_up(2, 5);
    assert_eq!(state.scroll_offset(), 5);
}

#[test]
fn goto_page_jumps_and_resets_scroll() {
    let state = ViewState::new().scroll_down(5, 20).goto_page(3, 5);
    assert_eq!(state.page(), 3);
    assert_eq!(state.scroll_offset(), 0);

    let state = ViewState::new().goto_page(99, 5);
    assert_eq!(state.page(), 4);
}
