// SPDX-License-Identifier: AGPL-3.0

//! Lucide chrome icons (<https://lucide.dev>, ISC license — see
//! `assets/icons/LICENSE-NOTE`), embedded as SVG and rasterized once per
//! icon to a texture; buttons tint them per state.
//!
//! Chrome never goes through font glyphs: fallbacks rendered some marks as
//! tofu boxes and mirrored the page-nav chevrons.

use std::collections::HashMap;

use eframe::egui;
use egui::{Color32, TextureHandle, Ui, Vec2};

/// Embedded Lucide SVG sources; `currentColor` is swapped for white at load
/// so the texture multiplies cleanly with any tint.
const SOURCES: &[(Icon, &str)] = &[
    (Icon::Menu, include_str!("../assets/icons/menu.svg")),
    (
        Icon::Dots,
        include_str!("../assets/icons/ellipsis-vertical.svg"),
    ),
    (Icon::Panel, include_str!("../assets/icons/panel-left.svg")),
    (Icon::Search, include_str!("../assets/icons/search.svg")),
    (Icon::Focus, include_str!("../assets/icons/maximize.svg")),
    (Icon::List, include_str!("../assets/icons/list.svg")),
    (Icon::Flag, include_str!("../assets/icons/flag.svg")),
    (Icon::Gear, include_str!("../assets/icons/settings.svg")),
    (Icon::Sun, include_str!("../assets/icons/sun.svg")),
    (Icon::Moon, include_str!("../assets/icons/moon.svg")),
    (
        Icon::ChevronLeft,
        include_str!("../assets/icons/chevron-left.svg"),
    ),
    (
        Icon::ChevronRight,
        include_str!("../assets/icons/chevron-right.svg"),
    ),
    (Icon::Plus, include_str!("../assets/icons/plus.svg")),
    (Icon::Minus, include_str!("../assets/icons/minus.svg")),
    (Icon::X, include_str!("../assets/icons/x.svg")),
    (Icon::Page, include_str!("../assets/icons/file-text.svg")),
    (Icon::Book, include_str!("../assets/icons/book-open.svg")),
    (
        Icon::Columns2,
        include_str!("../assets/icons/columns-2.svg"),
    ),
    (Icon::Save, include_str!("../assets/icons/save.svg")),
    (Icon::Info, include_str!("../assets/icons/info.svg")),
    (Icon::Pen, include_str!("../assets/icons/pen.svg")),
    (
        Icon::ChevronDown,
        include_str!("../assets/icons/chevron-down.svg"),
    ),
];

/// A chrome icon.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
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
    Page,
    Book,
    Columns2,
    Save,
    Info,
    Pen,
    ChevronDown,
}

impl Icon {
    fn source(self) -> &'static str {
        SOURCES
            .iter()
            .find(|(candidate, _)| *candidate == self)
            .map_or("menu", |(_, source)| source)
    }
}

/// Texture cache — one rasterization per icon for the whole session.
#[derive(Default)]
pub struct IconRender {
    textures: HashMap<Icon, TextureHandle>,
}

impl IconRender {
    fn texture(&mut self, ctx: &egui::Context, icon: Icon) -> TextureHandle {
        if let Some(tex) = self.textures.get(&icon) {
            return tex.clone();
        }
        let svg = icon.source().replace("currentColor", "#ffffff");
        let image = egui_extras::image::load_svg_bytes_with_size(
            svg.as_bytes(),
            Some(egui::SizeHint::Width(96)),
        )
        .expect("embedded Lucide SVG parses");
        let tex = ctx.load_texture(
            format!("lucide-{}", icon.source()),
            image,
            egui::TextureOptions::LINEAR,
        );
        self.textures.insert(icon, tex.clone());
        tex
    }

    /// Square icon button of `side` logical pixels.
    pub fn button(&mut self, ui: &mut Ui, icon: Icon, side: f32, color: Color32) -> egui::Response {
        let tex = self.texture(ui.ctx(), icon);
        let tint = if ui.is_enabled() {
            color
        } else {
            ui.visuals().widgets.inactive.fg_stroke.color
        };
        let image = egui::Image::new((tex.id(), Vec2::splat((side - 10.0).max(8.0)))).tint(tint);
        ui.add(egui::Button::image(image))
    }

    /// Pre-tinted image for composite buttons (icon + text).
    pub fn image(&mut self, ui: &mut Ui, icon: Icon, side: f32, color: Color32) -> egui::Image<'_> {
        let tex = self.texture(ui.ctx(), icon);
        egui::Image::new((tex.id(), Vec2::splat(side))).tint(color)
    }

    /// Paint an icon texture at an arbitrary rect (nav rows, menu items).
    pub fn paint_at(&mut self, ui: &mut Ui, rect: egui::Rect, icon: Icon, color: Color32) {
        let tex = self.texture(ui.ctx(), icon);
        ui.painter().image(
            tex.id(),
            rect,
            egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
            color,
        );
    }
}
