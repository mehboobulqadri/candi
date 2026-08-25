// SPDX-License-Identifier: AGPL-3.0

//! Painter-drawn chrome icons, Lucide-style strokes.
//!
//! Chrome never goes through text glyphs: font fallbacks render some marks
//! as tofu boxes and mirror others (`‹`/`›`), so every icon is drawn with
//! strokes here — crisp at any DPI, tintable by theme, no fonts involved.

use eframe::egui;
use egui::{Color32, CursorIcon, Painter, Pos2, Rect, Response, Sense, Shape, Stroke, Ui, Vec2};

/// A chrome icon; `draw` paints it inside a square-ish cell.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Icon {
    Menu,
    Dots,
    Panel,
    Search,
    Focus,
    List,
    Flag,
    Gear,
    Sun,
    Moon,
    ChevronLeft,
    ChevronRight,
    Plus,
    Minus,
    X,
}

impl Icon {
    /// Stroke width scaled to the cell so icons stay airy at any size.
    fn stroke_width(&self, cell: f32) -> f32 {
        (cell * 0.09).clamp(1.5, 2.2)
    }

    fn draw(&self, p: &Painter, cell: Rect, stroke: Stroke) {
        let r = cell.shrink(cell.height() * 0.16);
        let (l, t, ri, b) = (r.left(), r.top(), r.right(), r.bottom());
        let (w, h) = (r.width(), r.height());
        let c = r.center();
        let m = w.min(h);
        let pt = |fx: f32, fy: f32| Pos2::new(l + w * fx, t + h * fy);
        let ray = |p: &Painter, i: u32, r0: f32, r1: f32| {
            let a = std::f32::consts::TAU / 8.0 * i as f32;
            let (dx, dy) = (a.sin(), a.cos());
            p.line_segment(
                [
                    c + egui::vec2(dx * m * r0, dy * m * r0),
                    c + egui::vec2(dx * m * r1, dy * m * r1),
                ],
                stroke,
            );
        };
        match self {
            Icon::Menu => {
                for fy in [0.2, 0.5, 0.8] {
                    p.line_segment([pt(0.0, fy), pt(1.0, fy)], stroke);
                }
            }
            Icon::Dots => {
                for fy in [0.2, 0.5, 0.8] {
                    p.circle_filled(pt(0.5, fy), m * 0.07, stroke.color);
                }
            }
            Icon::Panel => {
                p.rect_stroke(r, 0.0, stroke);
                let x = l + w * 0.38;
                p.line_segment([Pos2::new(x, t), Pos2::new(x, b)], stroke);
            }
            Icon::Search => {
                p.add(Shape::circle_stroke(pt(0.4, 0.4), m * 0.26, stroke));
                p.line_segment([pt(0.6, 0.6), pt(0.88, 0.88)], stroke);
            }
            Icon::Focus => {
                for (sx, sy) in [(0.0_f32, 0.0), (1.0, 0.0), (0.0, 1.0), (1.0, 1.0)] {
                    let (x, y) = (l + w * sx, t + h * sy);
                    let dx = if sx == 0.0 { 1.0 } else { -1.0 };
                    let dy = if sy == 0.0 { 1.0 } else { -1.0 };
                    p.add(Shape::line(
                        vec![
                            Pos2::new(x + w * 0.34 * dx, y),
                            Pos2::new(x, y),
                            Pos2::new(x, y + h * 0.34 * dy),
                        ],
                        stroke,
                    ));
                }
            }
            Icon::List => {
                for fy in [0.2, 0.5, 0.8] {
                    p.circle_filled(pt(0.08, fy), m * 0.05, stroke.color);
                    p.line_segment([pt(0.26, fy), pt(0.95, fy)], stroke);
                }
            }
            Icon::Flag => {
                p.line_segment([pt(0.28, 0.12), pt(0.28, 0.9)], stroke);
                p.add(Shape::convex_polygon(
                    vec![pt(0.28, 0.12), pt(0.82, 0.32), pt(0.28, 0.52)],
                    stroke.color,
                    Stroke::NONE,
                ));
            }
            Icon::Gear => {
                p.add(Shape::circle_stroke(c, m * 0.2, stroke));
                for i in 0..8 {
                    ray(p, i, 0.3, 0.42);
                }
            }
            Icon::Sun => {
                p.circle_filled(c, m * 0.15, stroke.color);
                for i in 0..8 {
                    ray(p, i, 0.24, 0.4);
                }
            }
            Icon::Moon => {
                p.add(Shape::circle_stroke(c, m * 0.32, stroke));
                let half: Vec<Pos2> = (0..=8)
                    .map(|i| {
                        let a =
                            -std::f32::consts::FRAC_PI_2 + std::f32::consts::PI * i as f32 / 8.0;
                        c + egui::vec2(a.sin() * m * 0.32, a.cos() * m * 0.32)
                    })
                    .collect();
                p.add(Shape::convex_polygon(half, stroke.color, Stroke::NONE));
            }
            Icon::ChevronLeft => {
                p.add(Shape::line(
                    vec![pt(0.62, 0.18), pt(0.34, 0.5), pt(0.62, 0.82)],
                    stroke,
                ));
            }
            Icon::ChevronRight => {
                p.add(Shape::line(
                    vec![pt(0.38, 0.18), pt(0.66, 0.5), pt(0.38, 0.82)],
                    stroke,
                ));
            }
            Icon::Plus => {
                p.line_segment([pt(0.5, 0.15), pt(0.5, 0.85)], stroke);
                p.line_segment([pt(0.15, 0.5), pt(0.85, 0.5)], stroke);
            }
            Icon::Minus => {
                p.line_segment([pt(0.15, 0.5), pt(0.85, 0.5)], stroke);
            }
            Icon::X => {
                p.line_segment([pt(0.18, 0.18), pt(0.82, 0.82)], stroke);
                p.line_segment([pt(0.82, 0.18), pt(0.18, 0.82)], stroke);
            }
        }
        let _ = (ri, b);
    }
}

/// Flat icon button: painted strokes, subtle hover wash, no borders.
pub fn icon_button(ui: &mut Ui, icon: Icon, side: f32, color: Color32) -> Response {
    icon_button_sized(ui, Vec2::splat(side), icon, color)
}

/// Icon button with an explicit cell size; the icon centers in the cell.
pub fn icon_button_sized(ui: &mut Ui, size: Vec2, icon: Icon, color: Color32) -> Response {
    let (rect, resp) = ui.allocate_exact_size(size, Sense::click());
    let visuals = ui.style().interact(&resp);
    let stroke_color = if resp.enabled() {
        color
    } else {
        visuals.fg_stroke.color
    };
    if resp.hovered() || resp.has_focus() {
        let wash = if resp.is_pointer_button_down_on() {
            ui.visuals().widgets.active.weak_bg_fill
        } else {
            ui.visuals().widgets.hovered.weak_bg_fill
        };
        ui.painter().rect_filled(rect, visuals.rounding, wash);
    }
    let stroke = Stroke::new(icon.stroke_width(size.y), stroke_color);
    icon.draw(&ui.painter_at(rect), rect, stroke);
    resp.on_hover_cursor(CursorIcon::PointingHand)
}
