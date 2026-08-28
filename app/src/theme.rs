//! Visual language for the app: fonts, palette, typography, widget styling
//! and the painted window chrome.

use std::sync::Arc;

use eframe::egui;
use egui::{
    Color32, CornerRadius, FontData, FontDefinitions, FontFamily, FontId, Margin, Stroke,
    StrokeKind, TextStyle,
};

/// Named family for semibold text. egui renders `RichText::strong()` as a
/// brighter colour, not a heavier weight, so real bold needs its own family.
pub const BOLD: &str = "ui-bold";
/// Named family for semibold figures.
pub const MONO_BOLD: &str = "mono-bold";

/// Installs Inter (UI) and JetBrains Mono (figures). Both are OFL-licensed and
/// cover Greek, which the bundled egui default does only passably.
pub fn install_fonts(ctx: &egui::Context) {
    let mut fonts = FontDefinitions::default();

    let mut add = |name: &str, bytes: &'static [u8]| {
        fonts
            .font_data
            .insert(name.to_owned(), Arc::new(FontData::from_static(bytes)));
    };
    add("inter", include_bytes!("../assets/fonts/Inter-Regular.ttf"));
    add(
        "inter-bold",
        include_bytes!("../assets/fonts/Inter-SemiBold.ttf"),
    );
    add(
        "jbmono",
        include_bytes!("../assets/fonts/JetBrainsMono-Regular.ttf"),
    );
    add(
        "jbmono-bold",
        include_bytes!("../assets/fonts/JetBrainsMono-Medium.ttf"),
    );

    // Prepend ours, keeping egui's emoji fonts as the fallback tail.
    fonts
        .families
        .entry(FontFamily::Proportional)
        .or_default()
        .insert(0, "inter".to_owned());
    fonts
        .families
        .entry(FontFamily::Monospace)
        .or_default()
        .insert(0, "jbmono".to_owned());

    let mut bold = fonts.families[&FontFamily::Proportional].clone();
    bold.insert(0, "inter-bold".to_owned());
    fonts.families.insert(FontFamily::Name(BOLD.into()), bold);

    let mut mono_bold = fonts.families[&FontFamily::Monospace].clone();
    mono_bold.insert(0, "jbmono-bold".to_owned());
    fonts
        .families
        .insert(FontFamily::Name(MONO_BOLD.into()), mono_bold);

    ctx.set_fonts(fonts);
}

/// Semibold proportional text.
pub fn bold(text: impl Into<String>, size: f32) -> egui::RichText {
    egui::RichText::new(text).font(FontId::new(size, FontFamily::Name(BOLD.into())))
}

/// Semibold monospace figures.
pub fn mono_bold(text: impl Into<String>, size: f32) -> egui::RichText {
    egui::RichText::new(text).font(FontId::new(size, FontFamily::Name(MONO_BOLD.into())))
}

/// Regular monospace figures.
pub fn mono(text: impl Into<String>, size: f32) -> egui::RichText {
    egui::RichText::new(text).font(FontId::new(size, FontFamily::Monospace))
}

// ---------------------------------------------------------------- palette

/// Every colour the UI is allowed to use, so light and dark stay in step.
#[derive(Clone, Copy)]
pub struct Palette {
    pub bg: Color32,
    pub surface: Color32,
    pub surface_alt: Color32,
    pub border: Color32,
    pub text: Color32,
    pub muted: Color32,
    pub accent: Color32,
    pub accent_soft: Color32,
    pub on_accent: Color32,
    pub pos: Color32,
    pub neg: Color32,
    pub row_alt: Color32,
    pub hover: Color32,
    pub warn: Color32,
    dark: bool,
    families: FamilySet,
}

/// The selectable themes: two light, two dark.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ThemeId {
    /// Cool white with a blue accent.
    Clean,
    /// Warm paper with a teal accent.
    Paper,
    /// Neutral charcoal.
    Night,
    /// Deep navy with a brighter blue.
    Ocean,
    /// The Dracula scheme, using its published colours.
    Dracula,
}

impl ThemeId {
    pub const ALL: [ThemeId; 5] = [
        ThemeId::Clean,
        ThemeId::Paper,
        ThemeId::Night,
        ThemeId::Ocean,
        ThemeId::Dracula,
    ];

    /// Stable identifier for settings.json — the label is free to change.
    pub fn key(self) -> &'static str {
        match self {
            ThemeId::Clean => "clean",
            ThemeId::Paper => "paper",
            ThemeId::Night => "night",
            ThemeId::Ocean => "ocean",
            ThemeId::Dracula => "dracula",
        }
    }

    pub fn from_key(key: &str) -> Option<ThemeId> {
        ThemeId::ALL.into_iter().find(|t| t.key() == key)
    }

    pub fn label(self) -> &'static str {
        match self {
            ThemeId::Clean => "Καθαρό",
            ThemeId::Paper => "Χαρτί",
            ThemeId::Night => "Νύχτα",
            ThemeId::Ocean => "Ωκεανός",
            ThemeId::Dracula => "Dracula",
        }
    }

    pub fn is_dark(self) -> bool {
        matches!(self, ThemeId::Night | ThemeId::Ocean | ThemeId::Dracula)
    }
}

pub fn palette(theme: ThemeId) -> Palette {
    match theme {
        ThemeId::Clean => CLEAN,
        ThemeId::Paper => PAPER,
        ThemeId::Night => NIGHT,
        ThemeId::Ocean => OCEAN,
        ThemeId::Dracula => DRACULA,
    }
}

impl Palette {
    pub fn is_dark(&self) -> bool {
        self.dark
    }
}

const CLEAN: Palette = Palette {
    bg: Color32::from_rgb(0xF3, 0xF5, 0xF9),
    surface: Color32::from_rgb(0xFF, 0xFF, 0xFF),
    surface_alt: Color32::from_rgb(0xEA, 0xEE, 0xF4),
    border: Color32::from_rgb(0xDC, 0xE1, 0xEA),
    text: Color32::from_rgb(0x1A, 0x1F, 0x2B),
    muted: Color32::from_rgb(0x69, 0x73, 0x87),
    accent: Color32::from_rgb(0x2F, 0x6C, 0xE8),
    accent_soft: Color32::from_rgb(0xE2, 0xEB, 0xFD),
    on_accent: Color32::from_rgb(0xFF, 0xFF, 0xFF),
    pos: Color32::from_rgb(0x0D, 0x7A, 0x55),
    neg: Color32::from_rgb(0xC0, 0x36, 0x2B),
    row_alt: Color32::from_rgb(0xF7, 0xF9, 0xFC),
    hover: Color32::from_rgb(0xE8, 0xEF, 0xFD),
    warn: Color32::from_rgb(0xB4, 0x54, 0x09),
    dark: false,
    families: FamilySet::Light,
};

/// Warm off-white, the colour of ledger paper, with a teal accent.
const PAPER: Palette = Palette {
    bg: Color32::from_rgb(0xF4, 0xF0, 0xE6),
    surface: Color32::from_rgb(0xFF, 0xFC, 0xF5),
    surface_alt: Color32::from_rgb(0xEB, 0xE4, 0xD5),
    border: Color32::from_rgb(0xDE, 0xD4, 0xC0),
    text: Color32::from_rgb(0x2B, 0x26, 0x1F),
    muted: Color32::from_rgb(0x79, 0x6E, 0x5C),
    accent: Color32::from_rgb(0x0F, 0x76, 0x6E),
    accent_soft: Color32::from_rgb(0xD8, 0xEC, 0xE8),
    on_accent: Color32::from_rgb(0xFF, 0xFF, 0xFF),
    pos: Color32::from_rgb(0x1B, 0x74, 0x4A),
    neg: Color32::from_rgb(0xB0, 0x3C, 0x2A),
    row_alt: Color32::from_rgb(0xFA, 0xF6, 0xEC),
    hover: Color32::from_rgb(0xE6, 0xF0, 0xED),
    warn: Color32::from_rgb(0xA1, 0x5C, 0x0B),
    dark: false,
    families: FamilySet::Light,
};

const NIGHT: Palette = Palette {
    bg: Color32::from_rgb(0x11, 0x13, 0x18),
    surface: Color32::from_rgb(0x1A, 0x1D, 0x24),
    surface_alt: Color32::from_rgb(0x23, 0x27, 0x30),
    border: Color32::from_rgb(0x2C, 0x31, 0x3C),
    text: Color32::from_rgb(0xE6, 0xE9, 0xEF),
    muted: Color32::from_rgb(0x93, 0x9E, 0xB0),
    accent: Color32::from_rgb(0x66, 0xA1, 0xFF),
    accent_soft: Color32::from_rgb(0x1D, 0x2B, 0x44),
    on_accent: Color32::from_rgb(0x0B, 0x12, 0x1F),
    pos: Color32::from_rgb(0x3F, 0xCF, 0x8E),
    neg: Color32::from_rgb(0xF2, 0x7C, 0x76),
    row_alt: Color32::from_rgb(0x1E, 0x21, 0x29),
    hover: Color32::from_rgb(0x25, 0x2D, 0x3D),
    warn: Color32::from_rgb(0xE8, 0xA1, 0x4C),
    dark: true,
    families: FamilySet::Dark,
};

/// Deep navy — the same structure as `NIGHT`, but blue-tinted throughout.
const OCEAN: Palette = Palette {
    bg: Color32::from_rgb(0x0B, 0x12, 0x20),
    surface: Color32::from_rgb(0x12, 0x1B, 0x2C),
    surface_alt: Color32::from_rgb(0x1B, 0x26, 0x3B),
    border: Color32::from_rgb(0x24, 0x33, 0x4D),
    text: Color32::from_rgb(0xE2, 0xE9, 0xF5),
    muted: Color32::from_rgb(0x84, 0x96, 0xB4),
    accent: Color32::from_rgb(0x4C, 0xA6, 0xFF),
    accent_soft: Color32::from_rgb(0x16, 0x30, 0x4F),
    on_accent: Color32::from_rgb(0x05, 0x10, 0x1F),
    pos: Color32::from_rgb(0x34, 0xD9, 0xA4),
    neg: Color32::from_rgb(0xFF, 0x7E, 0x85),
    row_alt: Color32::from_rgb(0x16, 0x20, 0x31),
    hover: Color32::from_rgb(0x1E, 0x2E, 0x48),
    warn: Color32::from_rgb(0xE8, 0xA9, 0x4C),
    dark: true,
    families: FamilySet::Dark,
};

/// Dracula, using the scheme's published colours: background `#282A36`,
/// foreground `#F8F8F2`, comment `#6272A4`, current-line `#44475A`, and purple
/// `#BD93F9` as the accent. `bg` uses the darker `#21222C` so panels on
/// `#282A36` still read as raised.
const DRACULA: Palette = Palette {
    bg: Color32::from_rgb(0x21, 0x22, 0x2C),
    surface: Color32::from_rgb(0x28, 0x2A, 0x36),
    surface_alt: Color32::from_rgb(0x34, 0x37, 0x46),
    border: Color32::from_rgb(0x44, 0x47, 0x5A),
    text: Color32::from_rgb(0xF8, 0xF8, 0xF2),
    muted: Color32::from_rgb(0x62, 0x72, 0xA4),
    accent: Color32::from_rgb(0xBD, 0x93, 0xF9),
    accent_soft: Color32::from_rgb(0x3B, 0x2E, 0x58),
    on_accent: Color32::from_rgb(0x21, 0x22, 0x2C),
    pos: Color32::from_rgb(0x50, 0xFA, 0x7B),
    neg: Color32::from_rgb(0xFF, 0x55, 0x55),
    row_alt: Color32::from_rgb(0x2D, 0x2F, 0x3D),
    hover: Color32::from_rgb(0x44, 0x47, 0x5A),
    warn: Color32::from_rgb(0xFF, 0xB8, 0x6C),
    dark: true,
    families: FamilySet::Dracula,
};

/// Account families, taken from the chart of accounts rather than from Greek
/// name prefixes, so the colouring works for any book.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Kind {
    Pagia,
    Apothemata,
    Apaitiseis,
    Kefalaio,
    Ypoxreoseis,
    Fpa,
    Ejoda,
    Esoda,
    Anorgana,
    Other,
}

impl Kind {
    /// The chart returns every matching category; the most specific one wins.
    pub fn from_types(types: &[String]) -> Kind {
        let mut kind = Kind::Other;
        for t in types {
            kind = match t.as_str() {
                "pagia" => Kind::Pagia,
                "apothemata" => Kind::Apothemata,
                "apaitiseis" => Kind::Apaitiseis,
                "kefalaio" => Kind::Kefalaio,
                "ypoxreoseis" => Kind::Ypoxreoseis,
                "fpa" => Kind::Fpa,
                "ejoda" => Kind::Ejoda,
                "esoda" => Kind::Esoda,
                "anorgana" => Kind::Anorgana,
                _ => kind,
            };
        }
        kind
    }

    /// Position in the `FAMILY_*` tables. `Other` has no colour of its own.
    fn slot(self) -> Option<usize> {
        Some(match self {
            Kind::Pagia => 0,
            Kind::Apothemata => 1,
            Kind::Apaitiseis => 2,
            Kind::Kefalaio => 3,
            Kind::Ypoxreoseis => 4,
            Kind::Fpa => 5,
            Kind::Ejoda => 6,
            Kind::Esoda => 7,
            Kind::Anorgana => 8,
            Kind::Other => return None,
        })
    }

    pub fn color(self, p: &Palette) -> Color32 {
        let Some(slot) = self.slot() else {
            return p.muted;
        };
        let table = match p.families {
            FamilySet::Light => &FAMILY_LIGHT,
            FamilySet::Dark => &FAMILY_DARK,
            FamilySet::Dracula => &FAMILY_DRACULA,
        };
        let rgb = table[slot];
        Color32::from_rgb((rgb >> 16) as u8, (rgb >> 8) as u8, rgb as u8)
    }
}

/// Which set of account-family hues a theme uses. Most themes only need a
/// light or a dark variant; Dracula brings its own named palette.
#[derive(Clone, Copy, PartialEq, Eq)]
enum FamilySet {
    Light,
    Dark,
    Dracula,
}

// Ordered by `Kind::slot`: pagia, apothemata, apaitiseis, kefalaio,
// ypoxreoseis, fpa, ejoda, esoda, anorgana.
const FAMILY_LIGHT: [u32; 9] = [
    0x7A54D8, 0x9A6210, 0x0B8FD1, 0x4B47D6, 0xC97A0A, 0xC42B7B, 0xE0553F, 0x0E9F6E, 0x5C6B80,
];
const FAMILY_DARK: [u32; 9] = [
    0xA98BF0, 0xD3A059, 0x54B9F0, 0x8F8BF2, 0xE9AE55, 0xEE7BB3, 0xF08A78, 0x40D79E, 0x92A2B8,
];
/// Dracula's own accents: purple, orange, cyan, pink, yellow, light pink, red,
/// green and comment blue.
const FAMILY_DRACULA: [u32; 9] = [
    0xBD93F9, 0xFFB86C, 0x8BE9FD, 0xFF79C6, 0xF1FA8C, 0xFF92D0, 0xFF5555, 0x50FA7B, 0x7B8AC4,
];

// ---------------------------------------------------------------- style

/// Rebuilds the whole `Style` for the chosen mode. Called only when the mode
/// actually changes, not every frame.
pub fn apply(ctx: &egui::Context, theme: ThemeId) {
    let dark = theme.is_dark();
    let p = palette(theme);
    let mut style = (*ctx.style()).clone();

    style.text_styles = [
        (
            TextStyle::Heading,
            FontId::new(18.0, FontFamily::Name(BOLD.into())),
        ),
        (TextStyle::Body, FontId::new(14.0, FontFamily::Proportional)),
        (
            TextStyle::Button,
            FontId::new(13.5, FontFamily::Proportional),
        ),
        (
            TextStyle::Small,
            FontId::new(11.5, FontFamily::Proportional),
        ),
        (
            TextStyle::Monospace,
            FontId::new(13.0, FontFamily::Monospace),
        ),
    ]
    .into();

    style.spacing.item_spacing = egui::vec2(8.0, 6.0);
    style.spacing.button_padding = egui::vec2(11.0, 5.0);
    style.spacing.window_margin = Margin::same(14);
    style.spacing.menu_margin = Margin::same(8);
    style.spacing.interact_size.y = 26.0;
    style.spacing.scroll.bar_width = 10.0;
    style.spacing.scroll.floating = false;

    let mut v = if dark {
        egui::Visuals::dark()
    } else {
        egui::Visuals::light()
    };
    v.panel_fill = p.bg;
    v.window_fill = p.surface;
    v.extreme_bg_color = p.surface_alt;
    v.faint_bg_color = p.row_alt;
    v.override_text_color = Some(p.text);
    v.hyperlink_color = p.accent;
    v.selection.bg_fill = p.accent_soft;
    v.selection.stroke = Stroke::new(1.0_f32, p.accent);
    v.window_stroke = Stroke::new(1.0_f32, p.border);
    v.window_corner_radius = CornerRadius::same(12);
    v.menu_corner_radius = CornerRadius::same(9);
    v.window_shadow = egui::epaint::Shadow {
        offset: [0, 10],
        blur: 28,
        spread: 0,
        color: Color32::from_black_alpha(if dark { 140 } else { 30 }),
    };
    v.popup_shadow = v.window_shadow;

    let r = CornerRadius::same(7);
    let w = &mut v.widgets;

    w.noninteractive.bg_fill = p.surface;
    w.noninteractive.weak_bg_fill = p.surface;
    w.noninteractive.bg_stroke = Stroke::new(1.0_f32, p.border);
    w.noninteractive.fg_stroke = Stroke::new(1.0_f32, p.text);
    w.noninteractive.corner_radius = r;

    w.inactive.bg_fill = p.surface_alt;
    w.inactive.weak_bg_fill = p.surface_alt;
    w.inactive.bg_stroke = Stroke::new(1.0_f32, p.border);
    w.inactive.fg_stroke = Stroke::new(1.0_f32, p.text);
    w.inactive.corner_radius = r;

    w.hovered.bg_fill = p.hover;
    w.hovered.weak_bg_fill = p.hover;
    w.hovered.bg_stroke = Stroke::new(1.0_f32, p.accent);
    w.hovered.fg_stroke = Stroke::new(1.0_f32, p.text);
    w.hovered.corner_radius = r;
    w.hovered.expansion = 0.0;

    w.active.bg_fill = p.accent_soft;
    w.active.weak_bg_fill = p.accent_soft;
    w.active.bg_stroke = Stroke::new(1.0_f32, p.accent);
    w.active.fg_stroke = Stroke::new(1.0_f32, p.text);
    w.active.corner_radius = r;
    w.active.expansion = 0.0;

    w.open.bg_fill = p.surface_alt;
    w.open.weak_bg_fill = p.surface_alt;
    w.open.bg_stroke = Stroke::new(1.0_f32, p.border);
    w.open.fg_stroke = Stroke::new(1.0_f32, p.text);
    w.open.corner_radius = r;

    style.visuals = v;
    ctx.set_style(style);
}

// ---------------------------------------------------------------- pieces

/// A raised content card.
pub fn card(p: &Palette) -> egui::Frame {
    egui::Frame::new()
        .fill(p.surface)
        .stroke(Stroke::new(1.0_f32, p.border))
        .corner_radius(CornerRadius::same(11))
        .inner_margin(Margin::symmetric(16, 13))
}

/// A flat bar that fills the width of a panel (toolbar, status bar).
pub fn bar(p: &Palette, vertical_pad: i8) -> egui::Frame {
    egui::Frame::new()
        .fill(p.surface)
        .inner_margin(Margin::symmetric(16, vertical_pad))
}

/// iOS-style segmented picker. Returns true when the selection changed.
pub fn segmented<T: PartialEq + Copy>(
    ui: &mut egui::Ui,
    p: &Palette,
    current: &mut T,
    options: &[(T, &str)],
) -> bool {
    let mut changed = false;
    egui::Frame::new()
        .fill(p.surface_alt)
        .corner_radius(CornerRadius::same(9))
        .inner_margin(Margin::same(3))
        .show(ui, |ui| {
            ui.spacing_mut().item_spacing.x = 2.0;
            // `ui.horizontal` inherits the parent's direction, which keeps the
            // frame shrunk to its content. In a right-to-left parent (the title
            // bar) that would emit the options back to front, so walk them in
            // reverse there and let the layout flip them back.
            ui.horizontal(|ui| {
                let rtl = ui.layout().main_dir() == egui::Direction::RightToLeft;
                let ordered: Vec<&(T, &str)> = if rtl {
                    options.iter().rev().collect()
                } else {
                    options.iter().collect()
                };
                for (value, label) in ordered {
                    let selected = *current == *value;
                    let text = if selected {
                        bold(*label, 13.0).color(p.on_accent)
                    } else {
                        egui::RichText::new(*label).color(p.muted)
                    };
                    let button = egui::Button::new(text)
                        .fill(if selected {
                            p.accent
                        } else {
                            Color32::TRANSPARENT
                        })
                        .stroke(Stroke::NONE)
                        .corner_radius(CornerRadius::same(7));
                    if ui.add(button).clicked() && !selected {
                        *current = *value;
                        changed = true;
                    }
                }
            });
        });
    changed
}

/// A small rounded label used for counts and states.
pub fn chip(ui: &mut egui::Ui, text: &str, fg: Color32, bg: Color32) {
    egui::Frame::new()
        .fill(bg)
        .corner_radius(CornerRadius::same(20))
        .inner_margin(Margin::symmetric(9, 2))
        .show(ui, |ui| {
            ui.label(egui::RichText::new(text).small().color(fg));
        });
}

/// Label above a number in the stat strip, right-aligned as a block.
///
/// The width is fixed: a plain `ui.vertical` here would claim all the
/// remaining width and push the neighbouring stats out of the row.
pub fn stat(ui: &mut egui::Ui, p: &Palette, label: &str, value: &str, color: Color32, big: bool) {
    let width = if big { 156.0 } else { 108.0 };
    let value_text = if big {
        mono_bold(value, 21.0).color(color)
    } else {
        mono(value, 15.0).color(color)
    };

    ui.allocate_ui_with_layout(
        egui::vec2(width, 44.0),
        egui::Layout::top_down(egui::Align::Max),
        |ui| {
            ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Extend);
            ui.spacing_mut().item_spacing.y = 3.0;
            ui.label(egui::RichText::new(label).small().color(p.muted));
            ui.label(value_text);
        },
    );
}

/// One row of the theme picker: a swatch of the target theme's own colours
/// plus its name, drawn in the colours of the theme currently in use.
pub fn theme_row(
    ui: &mut egui::Ui,
    host: &Palette,
    theme: ThemeId,
    current: ThemeId,
) -> egui::Response {
    let target = palette(theme);
    let selected = theme == current;
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(178.0, 30.0), egui::Sense::click());

    if selected {
        ui.painter()
            .rect_filled(rect, CornerRadius::same(7), host.accent_soft);
    } else if resp.hovered() {
        ui.painter()
            .rect_filled(rect, CornerRadius::same(7), host.hover);
    }

    // swatch: the theme's background with its surface and accent sitting on it
    let swatch = egui::Rect::from_min_size(
        egui::pos2(rect.left() + 9.0, rect.center().y - 9.0),
        egui::vec2(34.0, 18.0),
    );
    ui.painter()
        .rect_filled(swatch, CornerRadius::same(5), target.bg);
    ui.painter().circle_filled(
        egui::pos2(swatch.left() + 12.0, swatch.center().y),
        5.0,
        target.surface,
    );
    ui.painter().circle_filled(
        egui::pos2(swatch.right() - 11.0, swatch.center().y),
        5.0,
        target.accent,
    );
    ui.painter().rect_stroke(
        swatch,
        CornerRadius::same(5),
        Stroke::new(1.0_f32, host.border),
        StrokeKind::Inside,
    );

    let family = if selected {
        FontFamily::Name(BOLD.into())
    } else {
        FontFamily::Proportional
    };
    ui.painter().text(
        egui::pos2(swatch.right() + 11.0, rect.center().y),
        egui::Align2::LEFT_CENTER,
        theme.label(),
        FontId::new(13.5, family),
        if selected { host.accent } else { host.text },
    );

    resp
}

/// A filled dot. Used instead of a glyph like `●`, which the bundled fonts do
/// not cover — it would render as a tofu box.
pub fn dot(ui: &mut egui::Ui, color: Color32, radius: f32) {
    let (rect, _) =
        ui.allocate_exact_size(egui::vec2(radius * 2.0, radius * 2.0), egui::Sense::hover());
    ui.painter().circle_filled(rect.center(), radius, color);
}

/// A small triangle pointing right (collapsed) or down (expanded), painted
/// rather than drawn from a glyph, for the same font-coverage reason.
pub fn chevron(painter: &egui::Painter, center: egui::Pos2, collapsed: bool, color: Color32) {
    let pts = if collapsed {
        vec![
            egui::pos2(center.x - 2.5, center.y - 4.0),
            egui::pos2(center.x + 3.0, center.y),
            egui::pos2(center.x - 2.5, center.y + 4.0),
        ]
    } else {
        vec![
            egui::pos2(center.x - 4.0, center.y - 2.5),
            egui::pos2(center.x + 4.0, center.y - 2.5),
            egui::pos2(center.x, center.y + 3.0),
        ]
    };
    painter.add(egui::Shape::convex_polygon(pts, color, Stroke::NONE));
}

// ------------------------------------------------------------ window chrome

/// The outer frame of the borderless window. `radius` comes from the OS (see
/// `adopt_os_window_corners`) so our background follows the same curve the
/// compositor clips the window to. Corners are square while maximised, where a
/// rounded edge would show desktop through the screen edge.
pub fn window_frame(p: &Palette, maximized: bool, radius: u8) -> egui::Frame {
    let rounded = !maximized && radius > 0;
    egui::Frame::new()
        .fill(p.bg)
        .stroke(Stroke::new(1.0_f32, p.border))
        .corner_radius(if rounded {
            CornerRadius::same(radius)
        } else {
            CornerRadius::ZERO
        })
        .outer_margin(Margin::same(if rounded { 1 } else { 0 }))
}

/// Toolbar glyphs. Painted rather than typeset: the bundled fonts have no
/// icon coverage, so a character like `📂` would render as a tofu box.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ToolIcon {
    OpenFolder,
    CheckCircle,
    BarChart,
}

/// An icon-only toolbar button.
pub fn icon_button(
    ui: &mut egui::Ui,
    p: &Palette,
    icon: ToolIcon,
    enabled: bool,
    tooltip: &str,
) -> egui::Response {
    let sense = if enabled {
        egui::Sense::click()
    } else {
        egui::Sense::hover()
    };
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(34.0, 30.0), sense);

    if enabled && resp.hovered() {
        ui.painter()
            .rect_filled(rect, CornerRadius::same(7), p.hover);
    }
    let fg = if !enabled {
        p.muted.gamma_multiply(0.4)
    } else if resp.hovered() {
        p.accent
    } else {
        p.text
    };
    let stroke = Stroke::new(1.4_f32, fg);
    let c = rect.center();
    let painter = ui.painter();

    match icon {
        ToolIcon::OpenFolder => {
            // tab, then body — reads as a folder at this size
            painter.rect_stroke(
                egui::Rect::from_min_max(
                    egui::pos2(c.x - 8.0, c.y - 7.5),
                    egui::pos2(c.x - 1.5, c.y - 3.5),
                ),
                CornerRadius::same(1),
                stroke,
                StrokeKind::Inside,
            );
            painter.rect_stroke(
                egui::Rect::from_min_max(
                    egui::pos2(c.x - 8.0, c.y - 4.5),
                    egui::pos2(c.x + 8.0, c.y + 7.0),
                ),
                CornerRadius::same(2),
                stroke,
                StrokeKind::Inside,
            );
        }
        ToolIcon::CheckCircle => {
            painter.circle_stroke(c, 8.0, stroke);
            painter.line_segment(
                [
                    egui::pos2(c.x - 4.0, c.y + 0.2),
                    egui::pos2(c.x - 1.2, c.y + 3.4),
                ],
                stroke,
            );
            painter.line_segment(
                [
                    egui::pos2(c.x - 1.2, c.y + 3.4),
                    egui::pos2(c.x + 4.4, c.y - 3.2),
                ],
                stroke,
            );
        }
        ToolIcon::BarChart => {
            // three bars of rising height, sitting on a common baseline
            for (left, top) in [(-7.5_f32, 0.5_f32), (-1.8, -4.0), (3.9, -7.5)] {
                painter.rect_stroke(
                    egui::Rect::from_min_max(
                        egui::pos2(c.x + left, c.y + top),
                        egui::pos2(c.x + left + 3.6, c.y + 7.0),
                    ),
                    CornerRadius::same(1),
                    stroke,
                    StrokeKind::Inside,
                );
            }
        }
    }

    resp.on_hover_text(tooltip)
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum WinBtn {
    Minimize,
    Maximize,
    Restore,
    Close,
}

/// Header row for a popup window built with `title_bar(false)`: the title on
/// the left and the very same close button the main window uses on the right,
/// so every window in the app closes through an identical control. Returns
/// true when it was clicked.
///
/// egui's own window title bar draws a close cross of its own, styled by the
/// widget visuals rather than by us — hence the hand-rolled header.
pub fn popup_header(ui: &mut egui::Ui, p: &Palette, title: &str) -> bool {
    let mut closed = false;
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 8.0;
        ui.label(bold(title, 14.0).color(p.text));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            closed = window_button(ui, p, WinBtn::Close).clicked();
        });
    });
    ui.add_space(4.0);
    let line_y = ui.cursor().top();
    ui.painter()
        .hline(ui.max_rect().x_range(), line_y, Stroke::new(1.0_f32, p.border));
    ui.add_space(8.0);
    closed
}

/// A Windows-style caption button, drawn with strokes so it needs no glyphs.
pub fn window_button(ui: &mut egui::Ui, p: &Palette, kind: WinBtn) -> egui::Response {
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(40.0, 30.0), egui::Sense::click());
    let closing = kind == WinBtn::Close;

    if resp.hovered() {
        let bg = if closing {
            Color32::from_rgb(0xE8, 0x11, 0x23)
        } else {
            p.hover
        };
        ui.painter().rect_filled(rect, CornerRadius::same(7), bg);
    }
    let fg = if resp.hovered() && closing {
        Color32::WHITE
    } else {
        p.muted
    };
    let stroke = Stroke::new(1.2_f32, fg);
    let c = rect.center();
    let s = 4.5;
    let painter = ui.painter();

    match kind {
        WinBtn::Minimize => {
            painter.line_segment([egui::pos2(c.x - s, c.y), egui::pos2(c.x + s, c.y)], stroke);
        }
        WinBtn::Maximize => {
            painter.rect_stroke(
                egui::Rect::from_center_size(c, egui::vec2(s * 2.0, s * 2.0)),
                CornerRadius::same(2),
                stroke,
                StrokeKind::Inside,
            );
        }
        WinBtn::Restore => {
            let d = 2.0;
            painter.rect_stroke(
                egui::Rect::from_center_size(
                    egui::pos2(c.x - d / 2.0, c.y + d / 2.0),
                    egui::vec2(s * 2.0, s * 2.0),
                ),
                CornerRadius::same(2),
                stroke,
                StrokeKind::Inside,
            );
            painter.rect_stroke(
                egui::Rect::from_center_size(
                    egui::pos2(c.x + d / 2.0, c.y - d / 2.0),
                    egui::vec2(s * 2.0, s * 2.0),
                ),
                CornerRadius::same(2),
                stroke,
                StrokeKind::Inside,
            );
        }
        WinBtn::Close => {
            painter.line_segment(
                [egui::pos2(c.x - s, c.y - s), egui::pos2(c.x + s, c.y + s)],
                stroke,
            );
            painter.line_segment(
                [egui::pos2(c.x + s, c.y - s), egui::pos2(c.x - s, c.y + s)],
                stroke,
            );
        }
    }
    resp
}
