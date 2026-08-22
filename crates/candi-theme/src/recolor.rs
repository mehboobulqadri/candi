// SPDX-License-Identifier: AGPL-3.0

//! Guarded luminance LUT recolor pass.
//!
//! Maps a page bitmap's neutral tones onto a theme's `page_fg`/`page_bg`
//! (dark text → fg, paper → bg) while protecting saturated pixels — figures,
//! images. Fully saturated pixels pass through untouched; partially
//! saturated ones (anti-aliased edges of colored text) blend linearly toward
//! their mapped color so they do not leave bright halos on dark themes.
//! Pure integer math, one pass over the pixels; the only allocations are
//! stack arrays.

use crate::color::Color;

/// ΔRGB above which a pixel counts as fully saturated and is left alone.
const SAT_SKIP: u8 = 96;
/// ΔRGB up to which a pixel is neutral enough for the full remap.
const SAT_MAP: u8 = 48;

/// Recolor an RGBA8 buffer in place.
///
/// `rgba` is packed `[r, g, b, a]` quadruples; alpha bytes are never touched.
/// A trailing partial pixel (< 4 bytes) is ignored.
pub fn recolor(rgba: &mut [u8], page_bg: Color, page_fg: Color) {
    let pixels = rgba.len() / 4;

    // Luma histogram over every 4th pixel (Rec.601, integer).
    let mut hist = [0u32; 256];
    for px in (0..pixels).step_by(4) {
        let o = px * 4;
        hist[luma(rgba[o], rgba[o + 1], rgba[o + 2]) as usize] += 1;
    }

    // Percentiles of the sampled population: p2 ≈ paper floor, p95 ≈ ink
    // ceiling. A bin of 0 is never mistaken for a found percentile because
    // the thresholds require actual cumulative mass.
    let samples = hist.iter().sum::<u32>() as u64;
    let mut lo = 0usize;
    let mut hi = 255usize;
    let mut cum = 0u64;
    for (v, &count) in hist.iter().enumerate() {
        cum += count as u64;
        if lo == 0 && cum * 100 >= samples * 2 {
            lo = v;
        }
        if cum * 100 >= samples * 95 {
            hi = v;
            break;
        }
    }
    // Already dark-text-on-light-paper: stretch nothing, use the full range.
    if hi >= 235 && lo <= 20 {
        lo = 0;
        hi = 255;
    }
    // Sparse ink (< 2% of samples): p2 lands on paper white itself, so the
    // stretch range is empty and every neutral pixel would map to `page_bg`,
    // erasing what little content the page has. Map the full range instead.
    if hi <= lo {
        lo = 0;
        hi = 255;
    }

    let luts = build_luts(lo, hi, page_bg, page_fg);

    for px in rgba.chunks_exact_mut(4) {
        let (r, g, b) = (px[0], px[1], px[2]);
        let sat = r.max(g).max(b) - r.min(g).min(b);
        if sat >= SAT_SKIP {
            continue;
        }
        let l = luma(r, g, b) as usize;
        if sat <= SAT_MAP {
            px[0] = luts[0][l];
            px[1] = luts[1][l];
            px[2] = luts[2][l];
            continue;
        }
        // Between the thresholds: blend mapped toward original as saturation
        // rises, so red text's anti-aliased edges follow the theme instead of
        // staying harsh on dark backgrounds.
        let t = u32::from(sat - SAT_MAP) * 255 / u32::from(SAT_SKIP - SAT_MAP);
        let mix = |mapped: u8, orig: u8| -> u8 {
            ((u32::from(mapped) * (255 - t) + u32::from(orig) * t + 127) / 255) as u8
        };
        px[0] = mix(luts[0][l], r);
        px[1] = mix(luts[1][l], g);
        px[2] = mix(luts[2][l], b);
    }
}

/// Full-range integer Rec.601 luma: the coefficients sum to 256, so white
/// maps to exactly 255 and paper can land exactly on `page_bg`.
fn luma(r: u8, g: u8, b: u8) -> u8 {
    ((77 * r as u32 + 151 * g as u32 + 28 * b as u32) >> 8) as u8
}

/// Per-channel lookup tables mapping luma → color: t=0 → fg, t=255 → bg.
fn build_luts(lo: usize, hi: usize, bg: Color, fg: Color) -> [[u8; 256]; 3] {
    let stretch = |v: usize| {
        if hi > lo {
            (((v as i32 - lo as i32) * 255) / (hi - lo) as i32).clamp(0, 255)
        } else {
            255
        }
    };
    let mut luts = [[0u8; 256]; 3];
    let channels = [(fg.r(), bg.r()), (fg.g(), bg.g()), (fg.b(), bg.b())];
    for (lut, channel) in luts.iter_mut().zip(channels) {
        let (f, b) = (channel.0 as i32, channel.1 as i32);
        for (v, out) in lut.iter_mut().enumerate() {
            // div_euclid: +127 must round half-up for negative deltas too.
            *out = (f + (stretch(v) * (b - f) + 127).div_euclid(255)) as u8;
        }
    }
    luts
}
