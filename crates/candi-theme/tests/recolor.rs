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

#[test]
fn saturated_pixels_are_untouched() {
    let mut buf = scan_page(32);
    let red = [255u8, 0, 0, 255];
    buf[..4].copy_from_slice(&red);
    let amber = [200u8, 100, 80, 255];
    buf[4..8].copy_from_slice(&amber);
    recolor(&mut buf, dark_bg(), dark_fg());
    assert_eq!(&buf[..4], &red);
    assert_eq!(&buf[4..8], &amber);
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
