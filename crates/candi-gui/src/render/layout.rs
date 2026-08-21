// SPDX-License-Identifier: AGPL-3.0

//! Pure layout geometry for the continuous page canvas.
//!
//! Computes per-page rectangles in content coordinates (origin top-left of the
//! content block) from page sizes in PDF points, a zoom mode, and the available
//! canvas width. No egui types here — the GUI maps these rects onto screen
//! space, so all of the math stays unit-testable.

use std::ops::Range;

use candi_core::ZoomMode;

/// Vertical gap between consecutive pages, in logical points.
pub const GAP: f32 = 8.0;
/// Margin around the content block on every side, in logical points.
pub const MARGIN: f32 = 12.0;
/// Zoom quantization step, in percent.
const ZOOM_STEP: f32 = 5.0;
/// Lowest and highest supported zoom percent.
pub const MIN_ZOOM_PERCENT: u16 = 25;
pub const MAX_ZOOM_PERCENT: u16 = 800;

/// Axis-aligned rectangle in content coordinates.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
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
    /// width of `avail_w`. Fit-width resolves against the widest page and is
    /// floored to a quantized step so pages never overflow horizontally.
    pub fn build(sizes: &[(f32, f32)], zoom: ZoomMode, avail_w: f32) -> Layout {
        if sizes.is_empty() {
            return Layout::default();
        }
        let scale = match zoom {
            ZoomMode::FitWidth => fit_width_percent(sizes, avail_w) as f32 / 100.0,
            ZoomMode::Percent(p) => p as f32 / 100.0,
        };
        let content_w = usable_width(avail_w);
        let mut rects = Vec::with_capacity(sizes.len());
        let mut y = MARGIN;
        for &(w_pt, h_pt) in sizes {
            let w = (w_pt * scale).max(1.0);
            let h = (h_pt * scale).max(1.0);
            rects.push(Rect {
                x: MARGIN + (content_w - w) / 2.0,
                y,
                w,
                h,
            });
            y += h + GAP;
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

    /// Page whose rect contains content-space `y`. Between-page positions
    /// resolve to the earlier page; positions past either end clamp to the
    /// nearest page so viewport-center tracking stays stable at the extremes.
    pub fn page_at(&self, y: f32) -> Option<usize> {
        if self.rects.is_empty() {
            return None;
        }
        let started = self.rects.partition_point(|r| r.y <= y);
        let page = started.saturating_sub(1);
        Some(page.min(self.rects.len() - 1))
    }
}

fn usable_width(avail_w: f32) -> f32 {
    (avail_w - 2.0 * MARGIN).max(1.0)
}

fn widest_page(sizes: &[(f32, f32)]) -> f32 {
    sizes.iter().map(|&(w, _)| w.max(1.0)).fold(1.0, f32::max)
}

/// Fit-width zoom percent for the widest page, floored to a quantized step.
pub fn fit_width_percent(sizes: &[(f32, f32)], avail_w: f32) -> u16 {
    quantize_floor(usable_width(avail_w) / widest_page(sizes) * 100.0)
}

/// Round `percent` to the nearest quantized step, clamped to the supported
/// range. Float error from the step math is absorbed by rounding to `u16`.
pub fn quantize_nearest(percent: f32) -> u16 {
    clamp_percent((percent / ZOOM_STEP).round() * ZOOM_STEP)
}

/// Floor `percent` to a quantized step, clamped to the supported range.
pub fn quantize_floor(percent: f32) -> u16 {
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
        Layout::build(&sizes, zoom, avail_w)
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
        // Content width is 800 - 24 = 776; centered: x = 12 + (776-306)/2.
        assert!((rect.x - (MARGIN + (776.0 - rect.w) / 2.0)).abs() < 1e-3);

        let wide = Layout::build(&[(1200.0, 400.0)], ZoomMode::Percent(100), 800.0);
        // Overflowing pages stay horizontally centered around the usable area
        // (both edges clip symmetrically, as in SumatraPDF).
        let rect = wide.rects[0];
        let content_center = MARGIN + (776.0 / 2.0);
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
        let mixed = Layout::build(&sizes, ZoomMode::FitWidth, 640.0);
        assert_eq!(mixed.zoom, 1.0);
        assert!((mixed.rects[1].w - 300.0).abs() < 1e-3);
        assert!(mixed.rects[1].x > mixed.rects[0].x, "narrower page centers");
    }

    #[test]
    fn visible_range_windows_the_document() {
        let layout = letter_layout(ZoomMode::Percent(100), 800.0, 10);
        // Page height 792; gap 8; stride 800. Band fully inside page 2.
        assert_eq!(layout.visible_range(MARGIN + 1600.0 + 100.0, 100.0), 2..3);
        // Band straddling the page 1 → page 2 boundary (y = 12 + 800 + 792).
        assert_eq!(layout.visible_range(1600.0, 20.0), 1..3);
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
        assert_eq!(layout.page_at(MARGIN + 800.0 + 1.0), Some(1));
        assert_eq!(
            layout.page_at(layout.total_height + 1.0),
            Some(2),
            "clamps below the last page"
        );
    }

    #[test]
    fn empty_and_degenerate_inputs_stay_finite() {
        let empty = Layout::build(&[], ZoomMode::FitWidth, 800.0);
        assert!(empty.rects.is_empty());
        assert_eq!(empty.total_height, 0.0);

        let degenerate = Layout::build(&[(0.0, 0.0)], ZoomMode::Percent(100), 800.0);
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
    fn fit_width_percent_uses_usable_width_over_widest_page() {
        // Usable = 800 - 24 = 776; 776/612 = 126.79…% → floor to 125.
        assert_eq!(fit_width_percent(&[LETTER], 800.0), 125);
        assert_eq!(fit_width_percent(&[LETTER, (300.0, 500.0)], 800.0), 125);
    }
}
