// SPDX-License-Identifier: AGPL-3.0

//! Pure layout geometry for the page canvas.
//!
//! Computes per-page rectangles in content coordinates (origin top-left of the
//! content block) from page sizes in PDF points, a zoom mode, the available
//! canvas width, and the page flow. No egui types here — the GUI maps these
//! rects onto screen space, so all of the math stays unit-testable.

use std::ops::Range;

use candi_core::ZoomMode;

/// Vertical gap between consecutive pages, in logical points.
pub const GAP: f32 = 12.0;
/// Margin around the content block on every side, in logical points.
pub const MARGIN: f32 = 12.0;
/// Zoom quantization step, in percent.
const ZOOM_STEP: f32 = 5.0;

use candi_core::MAX_ZOOM_PERCENT;
/// Lowest supported zoom percent; the bounds live in candi-core so the
/// sidecar parser clamps to exactly the same range.
pub use candi_core::MIN_ZOOM_PERCENT;

/// Axis-aligned rectangle in content coordinates.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

/// Page-flow arrangement: how pages are grouped into rows on the canvas.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Flow {
    /// All pages stacked vertically in reading order.
    Continuous,
    /// One page per row (1-up).
    Single,
    /// Two pages side by side per row (2-up spreads; last row may be short).
    Dual,
}

/// Pages laid out side by side within one row for `flow`.
pub fn pages_per_row(flow: Flow) -> usize {
    match flow {
        Flow::Continuous | Flow::Single => 1,
        Flow::Dual => 2,
    }
}

/// Precomputed canvas geometry: one rect per page plus the total height.
///
/// Content height comes from page aspect ratios times zoom alone, so the
/// visible range is a clip-rect intersection rather than a scan.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Layout {
    /// One rect per page, same order as the input sizes.
    pub rects: Vec<Rect>,
    /// Total content height including margins and gaps.
    pub total_height: f32,
    /// Effective zoom scale applied to point sizes (1.0 = 100%).
    pub zoom: f32,
}

impl Layout {
    /// Lay out `sizes` (page `(width, height)` in points) inside an available
    /// width of `avail_w`, grouped into rows of [`pages_per_row`] pages. Rows
    /// are separated vertically by [`GAP`]; pages within a row sit side by
    /// side, top-aligned, with the row centered on the usable width and its
    /// height set by its tallest page. Fit-width resolves against the widest
    /// row and is floored to a quantized step so no row overflows
    /// horizontally.
    pub fn build(sizes: &[(f32, f32)], zoom: ZoomMode, avail_w: f32, flow: Flow) -> Layout {
        if sizes.is_empty() {
            return Layout::default();
        }
        let per_row = pages_per_row(flow);
        let scale = match zoom {
            ZoomMode::FitWidth => fit_width_percent(sizes, avail_w, per_row) as f32 / 100.0,
            ZoomMode::Percent(p) => p as f32 / 100.0,
        };
        let content_w = usable_width(avail_w);
        let mut rects = Vec::with_capacity(sizes.len());
        let mut y = MARGIN;
        for row in sizes.chunks(per_row) {
            let inner_w: f32 = row
                .iter()
                .map(|&(w_pt, _)| (w_pt * scale).max(1.0))
                .sum::<f32>()
                + GAP * (row.len() - 1) as f32;
            let row_h = row
                .iter()
                .map(|&(_, h_pt)| (h_pt * scale).max(1.0))
                .fold(0.0_f32, f32::max);
            let mut x = MARGIN + (content_w - inner_w) / 2.0;
            for &(w_pt, h_pt) in row {
                let w = (w_pt * scale).max(1.0);
                let h = (h_pt * scale).max(1.0);
                rects.push(Rect { x, y, w, h });
                x += w + GAP;
            }
            y += row_h + GAP;
        }
        let total_height = if rects.is_empty() {
            0.0
        } else {
            y - GAP + MARGIN
        };
        Layout {
            rects,
            total_height,
            zoom: scale,
        }
    }

    /// Indices of pages overlapping the vertical band `[top, top + height)`.
    /// Requires rects sorted by `y`, which [`Layout::build`] guarantees.
    pub fn visible_range(&self, top: f32, height: f32) -> Range<usize> {
        let bottom = top + height;
        let start = self.rects.partition_point(|r| r.y + r.h <= top);
        let end = self.rects[start..].partition_point(|r| r.y < bottom);
        start..start + end
    }

    /// First page of the row containing content-space `y` — the left page of
    /// a spread in dual flow, so position tracking stays page-granular.
    /// Between-row positions resolve to the earlier row; positions past either
    /// end clamp to the nearest row so viewport-center tracking stays stable
    /// at the extremes.
    pub fn page_at(&self, y: f32) -> Option<usize> {
        if self.rects.is_empty() {
            return None;
        }
        let started = self.rects.partition_point(|r| r.y <= y);
        let mut page = started.saturating_sub(1);
        let row_y = self.rects[page].y;
        while page > 0 && self.rects[page - 1].y == row_y {
            page -= 1;
        }
        Some(page)
    }
}

fn usable_width(avail_w: f32) -> f32 {
    (avail_w - 2.0 * MARGIN).max(1.0)
}

/// Width of the widest row — pages side by side with [`GAP`] between them.
fn widest_row(sizes: &[(f32, f32)], per_row: usize) -> f32 {
    sizes
        .chunks(per_row)
        .map(|row| row.iter().map(|&(w, _)| w.max(1.0)).sum::<f32>() + GAP * (row.len() - 1) as f32)
        .fold(1.0, f32::max)
}

/// Fit-width zoom percent for the widest row, floored to a quantized step.
fn fit_width_percent(sizes: &[(f32, f32)], avail_w: f32, per_row: usize) -> u16 {
    quantize_floor(usable_width(avail_w) / widest_row(sizes, per_row) * 100.0)
}

/// Fit-page zoom percent: whichever of fit-width and fit-height is smaller,
/// floored to a quantized step so no axis overflows after rounding. Width
/// resolves against the widest row; height against the tallest page.
pub fn fit_page_percent(sizes: &[(f32, f32)], avail_w: f32, avail_h: f32, per_row: usize) -> u16 {
    let width_pct = usable_width(avail_w) / widest_row(sizes, per_row) * 100.0;
    let tallest = sizes.iter().map(|&(_, h)| h.max(1.0)).fold(1.0, f32::max);
    let height_pct = (avail_h.max(1.0)) / tallest * 100.0;
    quantize_floor(width_pct.min(height_pct))
}

/// Round `percent` to the nearest quantized step, clamped to the supported
/// range. Float error from the step math is absorbed by rounding to `u16`.
pub fn quantize_nearest(percent: f32) -> u16 {
    clamp_percent((percent / ZOOM_STEP).round() * ZOOM_STEP)
}

/// Floor `percent` to a quantized step, clamped to the supported range.
fn quantize_floor(percent: f32) -> u16 {
    clamp_percent((percent / ZOOM_STEP).floor() * ZOOM_STEP)
}

fn clamp_percent(percent: f32) -> u16 {
    (percent.round() as u16).clamp(MIN_ZOOM_PERCENT, MAX_ZOOM_PERCENT)
}

#[cfg(test)]
mod tests {
    use super::*;

    const LETTER: (f32, f32) = (612.0, 792.0);

    fn letter_layout(zoom: ZoomMode, avail_w: f32, pages: usize) -> Layout {
        let sizes = vec![LETTER; pages];
        Layout::build(&sizes, zoom, avail_w, Flow::Continuous)
    }

    #[test]
    fn rects_fill_total_height_with_gaps_and_margins() {
        let layout = letter_layout(ZoomMode::Percent(100), 800.0, 5);
        assert_eq!(layout.rects.len(), 5);
        let expected = 2.0 * MARGIN + 5.0 * (792.0 + GAP) - GAP;
        assert!((layout.total_height - expected).abs() < 1e-3);
        for pair in layout.rects.windows(2) {
            assert!((pair[1].y - (pair[0].y + pair[0].h + GAP)).abs() < 1e-3);
        }
    }

    #[test]
    fn zoom_scales_page_points_and_centers_horizontally() {
        let layout = letter_layout(ZoomMode::Percent(50), 800.0, 1);
        let rect = layout.rects[0];
        assert_eq!(layout.zoom, 0.5);
        assert!((rect.w - 306.0).abs() < 1e-3);
        assert!((rect.h - 396.0).abs() < 1e-3);
        // Content width is 800 - 2*MARGIN = 760; centered: x = MARGIN + (760-w)/2.
        let usable = 800.0 - 2.0 * MARGIN;
        assert!((rect.x - (MARGIN + (usable - rect.w) / 2.0)).abs() < 1e-3);

        let wide = Layout::build(
            &[(1200.0, 400.0)],
            ZoomMode::Percent(100),
            800.0,
            Flow::Continuous,
        );
        // Overflowing pages stay horizontally centered around the usable area
        // (both edges clip symmetrically, as in SumatraPDF).
        let rect = wide.rects[0];
        let content_center = MARGIN + (usable / 2.0);
        assert!((rect.x + rect.w / 2.0 - content_center).abs() < 1e-3);
    }

    #[test]
    fn fit_width_fills_widest_page_without_overflowing() {
        // 640 window: usable 616; 616/612 = 100.65% → floor to 100%.
        let layout = letter_layout(ZoomMode::FitWidth, 640.0, 1);
        assert_eq!(layout.zoom, 1.0);
        assert!(layout.rects[0].w <= 640.0 - 2.0 * MARGIN);

        // Mixed sizes resolve against the widest page so both fit.
        let sizes = vec![LETTER, (300.0, 500.0)];
        let mixed = Layout::build(&sizes, ZoomMode::FitWidth, 640.0, Flow::Continuous);
        assert_eq!(mixed.zoom, 1.0);
        assert!((mixed.rects[1].w - 300.0 * mixed.zoom).abs() < 1e-3);
        assert!(mixed.rects[1].x > mixed.rects[0].x, "narrower page centers");
    }

    #[test]
    fn visible_range_windows_the_document() {
        let layout = letter_layout(ZoomMode::Percent(100), 800.0, 10);
        // Page height 792; stride 792 + GAP.
        assert_eq!(
            layout.visible_range(MARGIN + 2.0 * (792.0 + GAP) + 100.0, 100.0),
            2..3
        );
        // Band straddling the page 1 → page 2 boundary: starts inside page 1,
        // extends past page 2's top edge across the gap.
        let boundary = MARGIN + (792.0 + GAP) + 792.0;
        assert_eq!(layout.visible_range(boundary - 2.0, 20.0), 1..3);
        // Whole document at once.
        let full = layout.visible_range(0.0, layout.total_height);
        assert_eq!(full, 0..10);
        // Past the end and before the start yield empty ranges.
        assert!(
            layout
                .visible_range(layout.total_height + 10.0, 50.0)
                .is_empty()
        );
        assert!(layout.visible_range(-50.0, 5.0).is_empty());
    }

    #[test]
    fn page_at_maps_content_y_to_pages() {
        let layout = letter_layout(ZoomMode::Percent(100), 800.0, 3);
        assert_eq!(layout.page_at(-5.0), Some(0), "clamps above the first page");
        assert_eq!(layout.page_at(MARGIN + 1.0), Some(0));
        // In the gap after page 0 resolves to page 0.
        assert_eq!(layout.page_at(MARGIN + 792.0 + GAP / 2.0), Some(0));
        assert_eq!(layout.page_at(MARGIN + (792.0 + GAP) + 1.0), Some(1));
        assert_eq!(
            layout.page_at(layout.total_height + 1.0),
            Some(2),
            "clamps below the last page"
        );
    }

    #[test]
    fn empty_and_degenerate_inputs_stay_finite() {
        let empty = Layout::build(&[], ZoomMode::FitWidth, 800.0, Flow::Continuous);
        assert!(empty.rects.is_empty());
        assert_eq!(empty.total_height, 0.0);

        let degenerate = Layout::build(
            &[(0.0, 0.0)],
            ZoomMode::Percent(100),
            800.0,
            Flow::Continuous,
        );
        assert_eq!(degenerate.rects[0].w, 1.0);
        assert_eq!(degenerate.rects[0].h, 1.0);
    }

    #[test]
    fn quantization_is_stable_and_bounded() {
        assert_eq!(quantize_nearest(103.9), 105);
        assert_eq!(quantize_nearest(102.4), 100);
        assert_eq!(quantize_floor(109.9), 105);
        assert_eq!(quantize_floor(100.0), 100);
        assert_eq!(quantize_nearest(1.0), MIN_ZOOM_PERCENT);
        assert_eq!(quantize_nearest(99_999.0), MAX_ZOOM_PERCENT);
        // Idempotent once quantized.
        let q = quantize_nearest(137.0);
        assert_eq!(quantize_nearest(q as f32), q);
    }

    #[test]
    fn fit_width_percent_uses_usable_width_over_the_widest_row() {
        // Usable = 800 - 2*MARGIN = 776; 776/612 = 126.79…% → floor to 125.
        assert_eq!(fit_width_percent(&[LETTER], 800.0, 1), 125);
        assert_eq!(fit_width_percent(&[LETTER, (300.0, 500.0)], 800.0, 1), 125);
        // In dual flow the whole spread must fit: widest row is 2·612 + GAP.
        assert_eq!(fit_width_percent(&[LETTER, LETTER], 1300.0, 2), 100);
    }

    #[test]
    fn fit_page_is_capped_by_height_in_a_short_window() {
        // Width alone would allow 125%; height 600/792 = 75.7% wins.
        assert_eq!(fit_page_percent(&[LETTER], 800.0, 600.0, 1), 75);
        assert_eq!(fit_page_percent(&[LETTER], 1600.0, 600.0, 1), 75);
    }

    #[test]
    fn fit_page_is_capped_by_width_in_a_narrow_window() {
        // Height alone would allow ~101%; width floors to 100.
        assert_eq!(fit_page_percent(&[LETTER], 800.0, 800.0, 1), 100);
    }

    #[test]
    fn fit_page_resolves_against_the_tallest_page() {
        let sizes = [LETTER, (300.0, 1200.0)];
        // Tallest page: height 500/1200 = 41.6% → floor to 40.
        assert_eq!(fit_page_percent(&sizes, 2000.0, 500.0, 1), 40);
    }

    #[test]
    fn fit_page_never_exceeds_fit_width_and_stays_bounded() {
        for &(w, h) in &[(300.0, 300.0), (800.0, 600.0), (2000.0, 1500.0)] {
            let page = fit_page_percent(&[LETTER], w, h, 1);
            let layout = letter_layout(ZoomMode::Percent(page), w.max(24.0), 1);
            assert!(layout.rects[0].w <= w.max(2.0 * MARGIN) + 1.0, "{w}");
            assert!(
                (MIN_ZOOM_PERCENT..=MAX_ZOOM_PERCENT).contains(&page),
                "{page}"
            );
        }
    }

    #[test]
    fn fit_page_clamps_a_tiny_viewport_to_the_minimum_zoom() {
        assert_eq!(fit_page_percent(&[LETTER], 30.0, 30.0, 1), MIN_ZOOM_PERCENT);
    }

    #[test]
    fn dual_flow_groups_pages_into_spreads() {
        // 1300 window: usable 1276; widest row = 2·612 + GAP = 1236 → 103%
        // floors to 100 (a widest-page basis would allow far more).
        let sizes = [LETTER; 3];
        let dual = Layout::build(&sizes, ZoomMode::FitWidth, 1300.0, Flow::Dual);
        assert_eq!(dual.zoom, 1.0, "fit-width resolves against the widest row");
        // Row (0, 1): a centered pair sharing one band.
        assert!((dual.rects[1].x - (dual.rects[0].x + 612.0 + GAP)).abs() < 1e-3);
        assert_eq!(dual.rects[0].y, MARGIN);
        assert_eq!(dual.rects[1].y, MARGIN, "spread pages share the row band");
        let expected = 2.0 * MARGIN + 2.0 * 792.0 + GAP;
        assert!((dual.total_height - expected).abs() < 1e-3);

        // Short last row: page 2 sits alone, centered like any 1-up row.
        let alone = dual.rects[2];
        assert!((alone.y - (MARGIN + 792.0 + GAP)).abs() < 1e-3);
        let centered = MARGIN + (1300.0 - 2.0 * MARGIN - 612.0) / 2.0;
        assert!((alone.x - centered).abs() < 1e-3);

        // page_at yields the row's FIRST page so session.page stays on the
        // left page of a spread.
        assert_eq!(dual.page_at(MARGIN + 1.0), Some(0));
        assert_eq!(
            dual.page_at(MARGIN + 396.0),
            Some(0),
            "mid-spread depth resolves to the left page"
        );
        assert_eq!(
            dual.page_at(MARGIN + 792.0 + GAP / 2.0),
            Some(0),
            "the between-row gap resolves to the earlier row"
        );
        assert_eq!(dual.page_at(MARGIN + 792.0 + GAP + 1.0), Some(2));
        assert_eq!(
            dual.page_at(dual.total_height + 1.0),
            Some(2),
            "(2) is alone in its row and therefore its own row-first"
        );
    }
}
