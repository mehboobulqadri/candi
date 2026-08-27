use candi_theme::{Color, recolor};

fn dark_bg() -> Color {
    Color::from([0x16, 0x18, 0x1D, 0xFF])
}

fn dark_fg() -> Color {
    Color::from([0xE6, 0xE6, 0xE6, 0xFF])
}

/// A half-white half-black page: percentiles hit 0 and 255, so the clean-page
/// rule selects the full domain and mapping is exact. Blocks of four paper
/// pixels alternate with blocks of four ink pixels, so the every-4th-pixel
/// histogram samples both tones.
fn scan_page(len: usize) -> Vec<u8> {
    let mut buf = vec![255u8; len * 4];
    for (i, px) in buf.chunks_exact_mut(4).enumerate() {
        if i % 8 >= 4 {
            px[..3].fill(0);
        }
    }
    buf
}

#[test]
fn white_maps_exactly_to_page_bg() {
    let mut buf = scan_page(64);
    recolor(&mut buf, dark_bg(), dark_fg());
    let px = &buf[..4];
    assert_eq!((px[0], px[1], px[2]), (0x16, 0x18, 0x1D));
}

#[test]
fn black_maps_exactly_to_page_fg() {
    let mut buf = scan_page(64);
    recolor(&mut buf, dark_bg(), dark_fg());
    let px = &buf[16..20];
    assert_eq!((px[0], px[1], px[2]), (0xE6, 0xE6, 0xE6));
}

#[test]
fn gray_ramp_is_monotonic_per_channel() {
    // One sweep of grays 0..=255 after paper padding; no wrap-around seam.
    let ramp: Vec<u8> = (0..=255).flat_map(|v| [v, v, v, 255]).collect();
    // Pad with paper so percentiles span the ramp instead of collapsing.
    let mut page = scan_page(256);
    page.extend_from_slice(&ramp);
    recolor(&mut page, dark_bg(), dark_fg());
    let out = &page[256 * 4..];
    // Dark theme: ink (low luma) → bright fg, paper (high luma) → dark bg,
    // so output must be non-increasing as the input gray rises.
    for w in out.windows(8).step_by(4) {
        assert!(w[0] >= w[4] && w[1] >= w[5] && w[2] >= w[6], "{w:?}");
    }
}

/// Rec.601 integer luma, mirroring the pass's own formula.
fn luma_of(c: [u8; 3]) -> u8 {
    ((77 * c[0] as u32 + 151 * c[1] as u32 + 28 * c[2] as u32) >> 8) as u8
}

/// A percentile-clean page with `px` as its first pixel, so the pixel's
/// mapping is decided by the LUT rather than by the histogram.
fn page_with_pixel(px: [u8; 3]) -> Vec<u8> {
    let mut buf = scan_page(32);
    buf[..3].copy_from_slice(&px);
    buf
}

#[test]
fn fully_saturated_pixels_are_untouched() {
    // Pure red and a muted figure amber both sit at ΔRGB ≥ 96.
    let mut red = page_with_pixel([255, 0, 0]);
    recolor(&mut red, dark_bg(), dark_fg());
    assert_eq!(&red[..4], &[255, 0, 0, 255]);

    let mut amber = page_with_pixel([200, 100, 80]);
    recolor(&mut amber, dark_bg(), dark_fg());
    assert_eq!(&amber[..4], &[200, 100, 80, 255]);
}

#[test]
fn neutral_ceiling_maps_fully_and_saturation_floor_is_untouched() {
    // ΔRGB == 48 remaps exactly like the pixel's gray twin.
    let edge = [176u8, 128, 128];
    let l = luma_of(edge);
    let mut buf = page_with_pixel(edge);
    let mut twin = page_with_pixel([l, l, l]);
    recolor(&mut buf, dark_bg(), dark_fg());
    recolor(&mut twin, dark_bg(), dark_fg());
    assert_eq!(&buf[..3], &twin[..3]);

    // ΔRGB == 96 is already fully saturated and stays put.
    let mut sat = page_with_pixel([224, 128, 128]);
    recolor(&mut sat, dark_bg(), dark_fg());
    assert_eq!(&sat[..4], &[224, 128, 128, 255]);
}

#[test]
fn mid_saturation_blends_between_mapped_and_original() {
    // ΔRGB = 72: halfway through the blend band, so every channel must land
    // strictly between the original and the fully mapped gray twin. Red text's
    // anti-aliased edges follow the theme instead of leaving harsh halos.
    let px = [200u8, 128, 128];
    let l = luma_of(px);
    let mut blended = page_with_pixel(px);
    let mut twin = page_with_pixel([l, l, l]);
    recolor(&mut blended, dark_bg(), dark_fg());
    recolor(&mut twin, dark_bg(), dark_fg());
    for i in 0..3 {
        let lo = px[i].min(twin[i]);
        let hi = px[i].max(twin[i]);
        assert!(
            blended[i] > lo && blended[i] < hi,
            "channel {i}: {} not between {lo} and {hi}",
            blended[i]
        );
        assert_ne!(blended[i], px[i], "channel {i} must move off the original");
    }
}

#[test]
fn alpha_is_preserved() {
    let mut buf = scan_page(8);
    for px in buf.chunks_exact_mut(4) {
        px[3] = 7;
    }
    recolor(&mut buf, dark_bg(), dark_fg());
    assert!(buf.chunks_exact(4).all(|px| px[3] == 7));
}

#[test]
fn scanned_page_normalizes_end_to_end() {
    let mut buf = scan_page(1024);
    recolor(&mut buf, dark_bg(), dark_fg());
    for (i, px) in buf.chunks_exact(4).enumerate() {
        let expected = if i % 8 >= 4 { dark_fg() } else { dark_bg() };
        assert_eq!(
            (px[0], px[1], px[2]),
            (expected.r(), expected.g(), expected.b()),
            "pixel {i}"
        );
    }
}

#[test]
fn recolor_is_deterministic() {
    let mut a = scan_page(512);
    let mut b = scan_page(512);
    recolor(&mut a, dark_bg(), dark_fg());
    recolor(&mut b, dark_bg(), dark_fg());
    assert_eq!(a, b);
}

#[test]
fn single_pixel_page_recolors() {
    let mut buf = vec![255, 255, 255, 255];
    recolor(&mut buf, dark_bg(), dark_fg());
    assert_eq!(&buf[..3], &[0x16, 0x18, 0x1D]);
}

/// A page whose ink is under 2% of sampled pixels: p2 lands on paper white,
/// which used to collapse the stretch range and erase all content.
fn sparse_page(len: usize) -> Vec<u8> {
    let mut buf = vec![255u8; len * 4];
    // One ink pixel in 256 keeps coverage below the p2 threshold while still
    // being hit by the every-4th-pixel histogram.
    buf[128 * 4..128 * 4 + 3].fill(0);
    buf
}

#[test]
fn sparse_ink_survives_a_mostly_white_page() {
    let mut buf = sparse_page(256);
    recolor(&mut buf, dark_bg(), dark_fg());
    let px = &buf[128 * 4..128 * 4 + 4];
    assert_eq!(
        (px[0], px[1], px[2]),
        (dark_fg().r(), dark_fg().g(), dark_fg().b())
    );
}

#[test]
fn sparse_paper_stays_page_bg() {
    let mut buf = sparse_page(256);
    recolor(&mut buf, dark_bg(), dark_fg());
    let px = &buf[..4];
    assert_eq!((px[0], px[1], px[2]), (0x16, 0x18, 0x1D));
}

#[test]
fn single_black_pixel_maps_to_fg() {
    // Degenerate range again: the only sampled luma is the ink itself.
    let mut buf = vec![0u8; 4];
    recolor(&mut buf, dark_bg(), dark_fg());
    assert_eq!(&buf[..3], &[0xE6, 0xE6, 0xE6]);
}

#[test]
fn empty_buffer_is_a_no_op() {
    let mut buf: Vec<u8> = Vec::new();
    recolor(&mut buf, dark_bg(), dark_fg());
    assert!(buf.is_empty());
}

#[test]
fn trailing_partial_pixel_is_ignored() {
    let mut buf = scan_page(4);
    buf.push(9);
    recolor(&mut buf, dark_bg(), dark_fg());
    assert_eq!(buf.len(), 17);
    assert_eq!(buf[16], 9);
}
