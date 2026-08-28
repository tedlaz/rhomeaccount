// No console window behind the GUI. Debug builds keep it, so panics and log
// output still have somewhere to go while developing.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

//! rhomeaccount — personal home accounting (Rust/egui port of the PySide6 app).
//!
//! Opens a "book" folder of text journal files (same format as the Python
//! version), shows the trial balance as a collapsible account tree on the
//! left, the selected account's card (kartella) plus a chart on the right,
//! date filters and a balance-check report.
//!
//! The window is borderless: the title bar, its buttons and the resize edges
//! are all drawn by us (see `draw_title_bar` / `draw_resize_handles`).

mod theme;

use std::collections::HashSet;
use std::path::PathBuf;

use chrono::{NaiveDate, Utc};
use eframe::egui;
use egui::{Color32, RichText};
use egui_extras::{Column, TableBuilder};
use egui_plot::{Bar, BarChart, HLine, Line, Plot, PlotPoints};
use image::GenericImageView;
use rhomeaccount_core::book::{Book, YearResult};
use rhomeaccount_core::date_groups::Grouping;
use rhomeaccount_core::transaction::Transaction;
use rhomeaccount_core::utils::{f2gr, grup, round2};
use serde::{Deserialize, Serialize};

use theme::{Kind, Palette, ThemeId, WinBtn};

fn main() -> eframe::Result {
    let mut viewport = egui::ViewportBuilder::default()
        .with_inner_size([1320.0, 830.0])
        .with_min_inner_size([900.0, 560.0])
        .with_decorations(false) // we draw our own title bar
        .with_transparent(true) // so the rounded corners are really rounded
        .with_resizable(true)
        // Wayland and X11 match a window to its .desktop file by app id, so
        // this has to be the packaged id or the dock shows a generic icon.
        .with_app_id("io.github.tedlaz.rhomeaccount");
    if let Some(icon) = load_icon() {
        viewport = viewport.with_icon(icon);
    }
    let options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };
    eframe::run_native(
        "rhomeaccount",
        options,
        Box::new(|cc| {
            theme::install_fonts(&cc.egui_ctx);
            let corner_radius = adopt_os_window_corners(cc);
            Ok(Box::new(QHomeAccApp::new(corner_radius)))
        }),
    )
}

/// Asks the OS to round the window the way it rounds its own, and reports the
/// radius we should paint with so our background lines up with that clip.
///
/// A borderless window gets no rounding for free, so we opt in explicitly.
/// Whether the call succeeds is also the cleanest available answer to "does
/// this OS round windows at all" — Windows 10 rejects the attribute, Windows 11
/// accepts it — which beats sniffing build numbers.
#[cfg(windows)]
fn adopt_os_window_corners(handle: &dyn raw_window_handle::HasWindowHandle) -> u8 {
    use raw_window_handle::RawWindowHandle;

    // Declared here rather than pulling in a bindings crate: it is one stable
    // Win32 entry point and the release binary is size-sensitive.
    #[link(name = "dwmapi")]
    extern "system" {
        fn DwmSetWindowAttribute(
            hwnd: *mut core::ffi::c_void,
            attribute: u32,
            value: *const core::ffi::c_void,
            size: u32,
        ) -> i32;
    }
    const DWMWA_WINDOW_CORNER_PREFERENCE: u32 = 33;
    const DWMWCP_ROUND: i32 = 2;
    /// The radius Windows 11 itself uses for a standard window.
    const WIN11_RADIUS: u8 = 8;

    let Ok(window) = handle.window_handle() else {
        return WIN11_RADIUS;
    };
    let RawWindowHandle::Win32(win32) = window.as_raw() else {
        return WIN11_RADIUS;
    };

    let preference = DWMWCP_ROUND;
    let hresult = unsafe {
        DwmSetWindowAttribute(
            win32.hwnd.get() as *mut core::ffi::c_void,
            DWMWA_WINDOW_CORNER_PREFERENCE,
            std::ptr::addr_of!(preference).cast(),
            std::mem::size_of_val(&preference) as u32,
        )
    };
    // S_OK means the compositor is rounding it; anything else means this
    // Windows draws square corners and so should we.
    if hresult == 0 {
        WIN11_RADIUS
    } else {
        0
    }
}

#[cfg(not(windows))]
fn adopt_os_window_corners(_handle: &dyn raw_window_handle::HasWindowHandle) -> u8 {
    10
}

/// Loads the window icon; a decode failure is not worth aborting startup for.
fn load_icon() -> Option<egui::IconData> {
    let png = include_bytes!("../assets/homeacc.png");
    let image = image::load_from_memory(png).ok()?;
    let size = image.dimensions();
    let rgba = image.to_rgba8().into_raw();
    Some(egui::IconData {
        width: size.0,
        height: size.1,
        rgba,
    })
}

// ---------------------------------------------------------------- settings

#[derive(Serialize, Deserialize, Default)]
struct Settings {
    filename: String,
    /// Theme key. Absent in files written before themes were named.
    #[serde(default)]
    theme: Option<String>,
    /// Legacy light/dark flag, still written so an older build downgrades
    /// gracefully, and read when `theme` is missing.
    #[serde(default)]
    dark_mode: bool,
}

impl Settings {
    fn theme_id(&self) -> ThemeId {
        self.theme
            .as_deref()
            .and_then(ThemeId::from_key)
            .unwrap_or(if self.dark_mode {
                ThemeId::Night
            } else {
                ThemeId::Clean
            })
    }

    fn set_theme(&mut self, theme: ThemeId) {
        self.theme = Some(theme.key().to_owned());
        self.dark_mode = theme.is_dark();
    }

    fn path() -> Option<PathBuf> {
        dirs::config_dir().map(|d| {
            d.join("TedLazaros")
                .join("rhomeaccount")
                .join("settings.json")
        })
    }

    fn load() -> Settings {
        Settings::path()
            .and_then(|p| std::fs::read_to_string(p).ok())
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }

    fn save(&self) {
        if let Some(path) = Settings::path() {
            if let Some(dir) = path.parent() {
                let _ = std::fs::create_dir_all(dir);
            }
            if let Ok(json) = serde_json::to_string_pretty(self) {
                let _ = std::fs::write(path, json);
            }
        }
    }
}

// ---------------------------------------------------------------- view data

/// One line of the trial-balance tree, prepared for painting.
#[derive(Clone)]
struct AccountRow {
    name: String,
    /// Just the last dot-separated segment — the tree shows the hierarchy.
    leaf: String,
    depth: usize,
    has_children: bool,
    value: f64,
    kind: Kind,
}

/// One kartella row, pre-formatted so the table does no work per frame.
#[derive(Clone)]
struct KartRow {
    id: i32,
    id_str: String,
    date: String,
    debit: String,
    credit: String,
    balance: String,
    balance_value: f64,
    perigrafi: String,
    sxolio: String,
    parastatiko: String,
}

#[derive(PartialEq, Clone, Copy)]
enum Period {
    Minas,
    Trimino,
    Ejamino,
    Etos,
}

const KARTELLA_HEADERS: [&str; 8] = [
    "Νο",
    "Ημ/νία",
    "Χρέωση",
    "Πίστωση",
    "Υπόλοιπο",
    "Περιγραφή",
    "Σχόλιο",
    "Παρ/κό",
];

const RIGHT_ALIGNED: [bool; 8] = [false, false, true, true, true, false, false, false];

/// Accounts whose chart shows the running balance rather than the period
/// movement (assets and liabilities, as opposed to income and expenses).
const CUMULATIVE_KINDS: [Kind; 5] = [
    Kind::Pagia,
    Kind::Apaitiseis,
    Kind::Ypoxreoseis,
    Kind::Kefalaio,
    Kind::Apothemata,
];

/// How tall a popup's scrolling body may grow before it starts scrolling
/// rather than pushing the window past the screen.
const POPUP_MAX_H: f32 = 420.0;

const TITLE_BAR_H: f32 = 40.0;
/// Space the title bar keeps clear on the right for the theme toggle, the
/// period picker and the caption buttons.
const RIGHT_GROUP_W: f32 = 580.0;

// ---------------------------------------------------------------- app state

struct QHomeAccApp {
    settings: Settings,
    book_path: String,
    book: Option<Book>,
    /// Bumped on every successful open so the caches know to rebuild.
    book_gen: u64,

    selected_account: String,
    account_search: String,
    collapsed: HashSet<String>,

    filter_enabled: bool,
    filter_date: String, // "YYYY-MM-DD"
    period: Period,

    theme: ThemeId,
    applied_theme: Option<ThemeId>,

    /// Double-clicked kartella row; resolved to a transaction next frame.
    pending_transaction: Option<i32>,
    show_transaction: Option<Transaction>,
    show_validations: bool,
    validation_rows: Vec<(String, String, f64, f64, f64, bool)>,
    show_results: bool,
    result_rows: Vec<YearResult>,
    parse_errors: Vec<String>,
    show_errors: bool,
    fatal_error: Option<String>,

    // cached derived views, rebuilt only when their inputs change
    accounts_cache: Vec<AccountRow>,
    accounts_key: (u64, Option<String>),
    kart_cache: Vec<KartRow>,
    kart_key: (u64, Option<String>, String),
    kart_totals: (f64, f64, f64),
    series_cache: Vec<(String, f64, f64)>,
    series_key: (u64, Option<String>, String, u8),
    /// Window corner radius, matched to the OS at startup.
    corner_radius: u8,
    /// Set when the plot should snap back to fitting all of its data.
    plot_reset: bool,
}

impl QHomeAccApp {
    fn new(corner_radius: u8) -> Self {
        let settings = Settings::load();
        let book_path = settings.filename.clone();
        let theme = settings.theme_id();
        let mut app = QHomeAccApp {
            settings,
            book_path,
            book: None,
            book_gen: 0,
            selected_account: String::new(),
            account_search: String::new(),
            collapsed: HashSet::new(),
            filter_enabled: false,
            filter_date: Utc::now().format("%Y-%m-%d").to_string(),
            period: Period::Minas,
            theme,
            applied_theme: None,
            pending_transaction: None,
            show_transaction: None,
            show_validations: false,
            validation_rows: Vec::new(),
            show_results: false,
            result_rows: Vec::new(),
            parse_errors: Vec::new(),
            show_errors: false,
            fatal_error: None,
            accounts_cache: Vec::new(),
            accounts_key: (u64::MAX, None),
            kart_cache: Vec::new(),
            kart_key: (u64::MAX, None, String::new()),
            kart_totals: (0.0, 0.0, 0.0),
            series_cache: Vec::new(),
            series_key: (u64::MAX, None, String::new(), 0),
            plot_reset: true,
            corner_radius,
        };
        if !app.book_path.is_empty() && PathBuf::from(&app.book_path).exists() {
            app.open_book(app.book_path.clone());
        }
        app
    }

    fn palette(&self) -> Palette {
        theme::palette(self.theme)
    }

    /// The "up to" date currently in force, if the filter is on and valid.
    fn eos(&self) -> Option<String> {
        if self.filter_enabled && self.date_valid() {
            Some(self.filter_date.clone())
        } else {
            None
        }
    }

    fn date_valid(&self) -> bool {
        NaiveDate::parse_from_str(&self.filter_date, "%Y-%m-%d").is_ok()
    }

    fn open_book(&mut self, path: String) {
        match rhomeaccount_core::parser_text::parse_folder(&path) {
            Ok((book, errors)) => {
                // Parse errors are reported but do not block opening: a single
                // unregistered account should not make the whole book
                // unreadable (this matches the Python app).
                self.parse_errors = errors;
                self.show_errors = !self.parse_errors.is_empty();
                self.settings.filename = path.clone();
                self.settings.save();
                self.book_path = path;
                self.selected_account.clear();
                self.account_search.clear();
                self.collapsed.clear();
                self.fatal_error = None;
                self.book = Some(book);
                self.book_gen += 1;
            }
            Err(e) => self.fatal_error = Some(e),
        }
    }

    fn grouping(&self) -> Grouping {
        match self.period {
            Period::Minas => Grouping::YearMonth,
            Period::Trimino => Grouping::Trimino,
            Period::Ejamino => Grouping::Ejamino,
            Period::Etos => Grouping::Year,
        }
    }

    /// «Έλεγχος υπολοίπων» — compare stored validations against balances.
    fn validate(&mut self) {
        let Some(book) = &self.book else { return };
        let mut rows = Vec::new();
        for (dat, acc, poso) in &book.validations {
            let ypoloipo = book.ypoloipo(acc, Some(dat));
            rows.push((
                dat.clone(),
                acc.clone(),
                ypoloipo,
                *poso,
                round2(ypoloipo - poso),
                ypoloipo == *poso,
            ));
        }
        self.validation_rows = rows;
        self.show_validations = true;
    }

    /// «Αποτελέσματα ανά έτος» — income minus expenses per year.
    fn results(&mut self) {
        let eos = self.eos();
        let Some(book) = &self.book else { return };
        let rows = book.results_by_year(None, eos.as_deref());
        self.result_rows = rows;
        self.show_results = true;
    }

    // ------------------------------------------------------------- caches

    fn refresh_accounts(&mut self) {
        let key = (self.book_gen, self.eos());
        if key == self.accounts_key {
            return;
        }
        self.accounts_key = key.clone();
        self.accounts_cache.clear();

        let Some(book) = &self.book else { return };
        let Ok(tree) = book.isozygio_tree(None, key.1.as_deref()) else {
            return;
        };

        // The map is sorted and '.' sorts before any letter, so an account's
        // children are always the rows immediately after it.
        let names: Vec<&String> = tree.keys().collect();
        for (i, name) in names.iter().enumerate() {
            let child_prefix = format!("{}.", name);
            let has_children = names
                .get(i + 1)
                .is_some_and(|next| next.starts_with(&child_prefix));
            self.accounts_cache.push(AccountRow {
                leaf: name.rsplit('.').next().unwrap_or(name).to_string(),
                depth: name.matches('.').count(),
                has_children,
                value: tree[*name].tvalue,
                kind: Kind::from_types(&book.chart.account_type(name)),
                name: (*name).clone(),
            });
        }
    }

    fn refresh_kartella(&mut self) {
        let key = (self.book_gen, self.eos(), self.selected_account.clone());
        if key == self.kart_key {
            return;
        }
        self.kart_key = key.clone();
        self.kart_cache.clear();
        self.kart_totals = (0.0, 0.0, 0.0);

        if key.2.is_empty() {
            return;
        }
        let Some(book) = &self.book else { return };

        let lines = book.kartella(&key.2, None, key.1.as_deref());
        if let Some(last) = lines.last() {
            self.kart_totals = (last.tdebit, last.tcredit, last.tvalue);
        }
        self.kart_cache = lines
            .iter()
            .rev() // newest first, like the Python model
            .map(|l| KartRow {
                id: l.id,
                id_str: l.id.to_string(),
                date: l.date.format("%Y-%m-%d").to_string(),
                debit: f2gr(l.debit),
                credit: f2gr(l.credit),
                balance: f2gr(l.tvalue),
                balance_value: l.tvalue,
                perigrafi: l.perigrafi.clone(),
                sxolio: l.sxolio.clone(),
                parastatiko: l.parastatiko.clone(),
            })
            .collect();
    }

    fn refresh_series(&mut self) {
        let key = (
            self.book_gen,
            self.eos(),
            self.selected_account.clone(),
            self.period as u8,
        );
        if key == self.series_key {
            return;
        }
        self.series_key = key.clone();
        self.series_cache.clear();
        // new data means the old pan/zoom no longer makes sense
        self.plot_reset = true;

        if key.2.is_empty() {
            return;
        }
        let grouping = self.grouping();
        if let Some(book) = &self.book {
            self.series_cache = book
                .time_series(&key.2, grouping, None, key.1.as_deref())
                .unwrap_or_default();
        }
    }

    fn selected_kind(&self) -> Kind {
        self.accounts_cache
            .iter()
            .find(|r| r.name == self.selected_account)
            .map(|r| r.kind)
            .unwrap_or(Kind::Other)
    }
}

// ---------------------------------------------------------------- app loop

impl eframe::App for QHomeAccApp {
    /// Transparent, so the rounded window corners show the desktop.
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        egui::Rgba::TRANSPARENT.to_array()
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if self.applied_theme != Some(self.theme) {
            theme::apply(ctx, self.theme);
            self.applied_theme = Some(self.theme);
        }

        // resolve a double-clicked kartella row into the transaction viewer
        if let Some(id) = self.pending_transaction.take() {
            if let Some(trn) = self.book.as_ref().and_then(|b| b.get_transaction(id)) {
                self.show_transaction = Some(trn.clone());
            }
        }

        self.refresh_accounts();
        self.refresh_kartella();
        self.refresh_series();

        let p = self.palette();
        let maximized = ctx.input(|i| i.viewport().maximized.unwrap_or(false));

        // Everything lives inside one panel so the window frame can round the
        // corners and clip the content to them.
        egui::CentralPanel::default()
            .frame(theme::window_frame(&p, maximized, self.corner_radius))
            .show(ctx, |ui| {
                let app_rect = ui.max_rect();

                self.draw_title_bar(ui, &p, maximized);
                self.draw_status_bar(ui, &p);
                if self.book.is_some() {
                    self.draw_accounts_panel(ui, &p);
                    self.draw_chart_panel(ui, &p);
                }
                self.draw_main(ui, &p);

                if !maximized {
                    draw_resize_handles(ui, app_rect);
                }
            });

        self.draw_popups(ctx);
    }
}

/// Invisible grab strips along the window edges, since a borderless window
/// gets none from the OS.
fn draw_resize_handles(ui: &egui::Ui, rect: egui::Rect) {
    use egui::{CursorIcon as Cur, ResizeDirection as Dir, ViewportCommand};

    const T: f32 = 6.0; // edge thickness
    const C: f32 = 16.0; // corner square

    let (l, r, t, b) = (rect.left(), rect.right(), rect.top(), rect.bottom());
    let handles: [(&str, egui::Rect, Dir, Cur); 8] = [
        (
            "n",
            egui::Rect::from_min_max(egui::pos2(l + C, t), egui::pos2(r - C, t + T)),
            Dir::North,
            Cur::ResizeNorth,
        ),
        (
            "s",
            egui::Rect::from_min_max(egui::pos2(l + C, b - T), egui::pos2(r - C, b)),
            Dir::South,
            Cur::ResizeSouth,
        ),
        (
            "w",
            egui::Rect::from_min_max(egui::pos2(l, t + C), egui::pos2(l + T, b - C)),
            Dir::West,
            Cur::ResizeWest,
        ),
        (
            "e",
            egui::Rect::from_min_max(egui::pos2(r - T, t + C), egui::pos2(r, b - C)),
            Dir::East,
            Cur::ResizeEast,
        ),
        (
            "nw",
            egui::Rect::from_min_max(egui::pos2(l, t), egui::pos2(l + C, t + C)),
            Dir::NorthWest,
            Cur::ResizeNorthWest,
        ),
        (
            "ne",
            egui::Rect::from_min_max(egui::pos2(r - C, t), egui::pos2(r, t + C)),
            Dir::NorthEast,
            Cur::ResizeNorthEast,
        ),
        (
            "sw",
            egui::Rect::from_min_max(egui::pos2(l, b - C), egui::pos2(l + C, b)),
            Dir::SouthWest,
            Cur::ResizeSouthWest,
        ),
        (
            "se",
            egui::Rect::from_min_max(egui::pos2(r - C, b - C), egui::pos2(r, b)),
            Dir::SouthEast,
            Cur::ResizeSouthEast,
        ),
    ];

    for (name, handle, dir, cursor) in handles {
        let resp = ui.interact(handle, egui::Id::new(("resize", name)), egui::Sense::drag());
        if resp.hovered() || resp.dragged() {
            ui.ctx().set_cursor_icon(cursor);
        }
        if resp.drag_started() {
            ui.ctx()
                .send_viewport_cmd(ViewportCommand::BeginResize(dir));
        }
    }
}

// ---------------------------------------------------------- window chrome

impl QHomeAccApp {
    fn draw_title_bar(&mut self, ui: &mut egui::Ui, p: &Palette, maximized: bool) {
        let book_name = self
            .book
            .as_ref()
            .map(|b| b.name.clone())
            .filter(|n| !n.is_empty());
        let n_errors = self.parse_errors.len();

        egui::TopBottomPanel::top("titlebar")
            .exact_height(TITLE_BAR_H)
            .frame(egui::Frame::new().fill(p.surface))
            .show_inside(ui, |ui| {
                let bar = ui.max_rect();

                // Claimed first so the buttons added below take priority over
                // it; anything not covered by them drags the window.
                let drag = ui.interact(
                    bar,
                    egui::Id::new("titlebar_drag"),
                    egui::Sense::click_and_drag(),
                );
                if drag.double_clicked() {
                    ui.ctx()
                        .send_viewport_cmd(egui::ViewportCommand::Maximized(!maximized));
                }
                if drag.drag_started_by(egui::PointerButton::Primary) {
                    ui.ctx().send_viewport_cmd(egui::ViewportCommand::StartDrag);
                }

                ui.horizontal_centered(|ui| {
                    ui.add_space(6.0);
                    let (mark, _) =
                        ui.allocate_exact_size(egui::vec2(20.0, 20.0), egui::Sense::hover());
                    ui.painter()
                        .rect_filled(mark, egui::CornerRadius::same(6), p.accent);
                    // Same mark as the app icon, so the badge and the taskbar
                    // entry read as the same thing.
                    ui.painter().text(
                        mark.center(),
                        egui::Align2::CENTER_CENTER,
                        "€",
                        egui::FontId::new(14.5, egui::FontFamily::Name(theme::BOLD.into())),
                        p.on_accent,
                    );
                    ui.add_space(2.0);
                    ui.label(theme::bold("rhomeaccount", 13.5).color(p.text));

                    ui.add_space(10.0);
                    if theme::icon_button(
                        ui,
                        p,
                        theme::ToolIcon::OpenFolder,
                        true,
                        "Άνοιγμα βιβλίου",
                    )
                    .clicked()
                    {
                        if let Some(dir) = rfd::FileDialog::new()
                            .set_title("Άνοιγμα βιβλίου")
                            .pick_folder()
                        {
                            self.open_book(dir.to_string_lossy().to_string());
                        }
                    }
                    if theme::icon_button(
                        ui,
                        p,
                        theme::ToolIcon::CheckCircle,
                        self.book.is_some(),
                        "Έλεγχος υπολοίπων",
                    )
                    .clicked()
                    {
                        self.validate();
                    }
                    if theme::icon_button(
                        ui,
                        p,
                        theme::ToolIcon::BarChart,
                        self.book.is_some(),
                        "Αποτελέσματα ανά έτος",
                    )
                    .clicked()
                    {
                        self.results();
                    }
                    if n_errors > 0 {
                        let warn = egui::Button::new(
                            RichText::new(format!("{} λάθη", n_errors))
                                .small()
                                .color(p.warn),
                        )
                        .fill(Color32::TRANSPARENT)
                        .stroke(egui::Stroke::new(1.0_f32, p.warn));
                        if ui.add(warn).clicked() {
                            self.show_errors = true;
                        }
                    }

                    // The book name takes what is left, minus room for the
                    // right-hand group, and truncates rather than pushing it.
                    if let Some(name) = &book_name {
                        let name_w = (ui.available_width() - RIGHT_GROUP_W).clamp(0.0, 320.0);
                        if name_w > 50.0 {
                            ui.allocate_ui_with_layout(
                                egui::vec2(name_w, 24.0),
                                egui::Layout::left_to_right(egui::Align::Center),
                                |ui| {
                                    ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Truncate);
                                    ui.label(RichText::new("·").color(p.muted));
                                    ui.label(RichText::new(name).color(p.muted));
                                },
                            );
                        }
                    }

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.spacing_mut().item_spacing.x = 2.0;
                        if theme::window_button(ui, p, WinBtn::Close).clicked() {
                            ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
                        }
                        let toggle = if maximized {
                            WinBtn::Restore
                        } else {
                            WinBtn::Maximize
                        };
                        if theme::window_button(ui, p, toggle).clicked() {
                            ui.ctx()
                                .send_viewport_cmd(egui::ViewportCommand::Maximized(!maximized));
                        }
                        if theme::window_button(ui, p, WinBtn::Minimize).clicked() {
                            ui.ctx()
                                .send_viewport_cmd(egui::ViewportCommand::Minimized(true));
                        }

                        // Added after the caption buttons so they land to the
                        // right of these in this right-to-left layout.
                        ui.add_space(10.0);
                        theme::segmented(
                            ui,
                            p,
                            &mut self.period,
                            &[
                                (Period::Minas, "Μήνας"),
                                (Period::Trimino, "Τρίμηνο"),
                                (Period::Ejamino, "Εξάμηνο"),
                                (Period::Etos, "Έτος"),
                            ],
                        );

                        ui.add_space(8.0);
                        let current = self.theme;
                        ui.menu_button(RichText::new(current.label()).small(), |ui| {
                            ui.spacing_mut().item_spacing.y = 2.0;
                            for candidate in ThemeId::ALL {
                                if theme::theme_row(ui, p, candidate, current).clicked() {
                                    self.theme = candidate;
                                    self.settings.set_theme(candidate);
                                    self.settings.save();
                                    ui.close_menu();
                                }
                            }
                        })
                        .response
                        .on_hover_text("Θέμα");
                    });
                });

                ui.painter().hline(
                    bar.x_range(),
                    bar.bottom() - 0.5,
                    egui::Stroke::new(1.0_f32, p.border),
                );
            });
    }
}

// -------------------------------------------------------------- status bar

impl QHomeAccApp {
    fn draw_status_bar(&mut self, ui: &mut egui::Ui, p: &Palette) {
        let n_accounts = self.accounts_cache.len();
        let n_rows = self.kart_cache.len();
        let n_transactions = self.book.as_ref().map_or(0, |b| b.transactions.len());
        let balanced = self.book.as_ref().map(|b| b.is_balanced());
        let path = self.book_path.clone();

        egui::TopBottomPanel::bottom("status")
            .frame(theme::bar(p, 5))
            .show_inside(ui, |ui| {
                let full = ui.max_rect();

                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 10.0;
                    match balanced {
                        None => {
                            ui.label(RichText::new("κανένα βιβλίο").small().color(p.muted));
                        }
                        Some(Ok(())) => {
                            theme::dot(ui, p.pos, 4.0);
                            ui.label(RichText::new("ισοσκελισμένο").small().color(p.pos));
                        }
                        Some(Err(e)) => {
                            theme::dot(ui, p.neg, 4.0);
                            ui.label(RichText::new(e).small().color(p.neg));
                        }
                    }
                    if self.book.is_some() {
                        ui.label(
                            RichText::new(format!("{} άρθρα", n_transactions))
                                .small()
                                .color(p.muted),
                        );
                        ui.label(
                            RichText::new(format!("{} λογαριασμοί", n_accounts))
                                .small()
                                .color(p.muted),
                        );
                        if !self.selected_account.is_empty() {
                            ui.label(
                                RichText::new(format!("{} κινήσεις", n_rows))
                                    .small()
                                    .color(p.muted),
                            );
                        }
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Truncate);
                        ui.label(RichText::new(path).small().color(p.muted));
                    });
                });

                // The date filter sits in the middle of the bar. It is placed
                // at an absolute rect rather than flowed, so that the width of
                // the status text on either side cannot shift it off-centre.
                const FILTER_W: f32 = 214.0;
                let center = egui::Rect::from_center_size(
                    full.center(),
                    egui::vec2(FILTER_W.min(full.width()), full.height()),
                );
                ui.scope_builder(
                    egui::UiBuilder::new()
                        .max_rect(center)
                        .layout(egui::Layout::left_to_right(egui::Align::Center)),
                    |ui| {
                        ui.spacing_mut().item_spacing.x = 6.0;
                        ui.checkbox(&mut self.filter_enabled, "Φίλτρο έως");
                        let valid = self.date_valid();
                        let edit = egui::TextEdit::singleline(&mut self.filter_date)
                            .desired_width(92.0)
                            .font(egui::TextStyle::Monospace)
                            .text_color(if valid { p.text } else { p.neg });
                        let resp = ui.add_enabled(self.filter_enabled, edit);
                        if !valid {
                            resp.on_hover_text("Μορφή ημερομηνίας: YYYY-MM-DD");
                        }
                    },
                );
            });
    }
}

// ------------------------------------------------------- accounts (left)

impl QHomeAccApp {
    fn draw_accounts_panel(&mut self, ui: &mut egui::Ui, p: &Palette) {
        egui::SidePanel::left("accounts")
            .resizable(true)
            .default_width(340.0)
            .min_width(250.0)
            .max_width(560.0)
            .frame(
                egui::Frame::new()
                    .fill(p.surface)
                    .inner_margin(egui::Margin::symmetric(10, 10)),
            )
            .show_inside(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(theme::bold("Ισοζύγιο", 15.0));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui
                            .add(egui::Button::new(RichText::new("−").size(15.0)).frame(false))
                            .on_hover_text("Σύμπτυξη όλων")
                            .clicked()
                        {
                            self.collapsed = self
                                .accounts_cache
                                .iter()
                                .filter(|r| r.has_children)
                                .map(|r| r.name.clone())
                                .collect();
                        }
                        if ui
                            .add(egui::Button::new(RichText::new("+").size(15.0)).frame(false))
                            .on_hover_text("Ανάπτυξη όλων")
                            .clicked()
                        {
                            self.collapsed.clear();
                        }
                    });
                });

                ui.add_space(6.0);
                ui.add(
                    egui::TextEdit::singleline(&mut self.account_search)
                        .hint_text("Αναζήτηση λογαριασμού…")
                        .desired_width(f32::INFINITY),
                );
                ui.add_space(8.0);

                self.draw_account_tree(ui, p);
            });
    }

    fn draw_account_tree(&mut self, ui: &mut egui::Ui, p: &Palette) {
        // Work out which rows are visible before painting, so the scroll area
        // can virtualise and the paint loop never touches `self`.
        let search = grup(self.account_search.trim());
        let searching = !search.is_empty();

        let mut visible: Vec<usize> = Vec::new();
        let mut hidden_under: Option<String> = None;
        for (i, row) in self.accounts_cache.iter().enumerate() {
            if searching {
                if grup(&row.name).contains(&search) {
                    visible.push(i);
                }
                continue;
            }
            if let Some(prefix) = &hidden_under {
                if row.name.starts_with(prefix.as_str()) {
                    continue;
                }
                hidden_under = None;
            }
            visible.push(i);
            if row.has_children && self.collapsed.contains(&row.name) {
                hidden_under = Some(format!("{}.", row.name));
            }
        }

        if visible.is_empty() {
            ui.add_space(20.0);
            ui.vertical_centered(|ui| {
                ui.label(RichText::new("Κανένας λογαριασμός").color(p.muted));
            });
            return;
        }

        let rows = &self.accounts_cache;
        let selected_name = self.selected_account.clone();
        let collapsed = &self.collapsed;
        let mut select: Option<String> = None;
        let mut toggle: Option<String> = None;

        const ROW_H: f32 = 25.0;
        egui::ScrollArea::vertical()
            .id_salt("accounts_scroll")
            .auto_shrink([false, false])
            .show_rows(ui, ROW_H, visible.len(), |ui, range| {
                for vi in range {
                    let row = &rows[visible[vi]];
                    // In search results the hierarchy is meaningless, so show
                    // the full path flat instead of a misleading indent.
                    let depth = if searching { 0 } else { row.depth };
                    let label = if searching { &row.name } else { &row.leaf };
                    let is_collapsed = collapsed.contains(&row.name);
                    let selected = selected_name == row.name;

                    let width = ui.available_width();
                    let (rect, resp) =
                        ui.allocate_exact_size(egui::vec2(width, ROW_H), egui::Sense::click());

                    if selected {
                        ui.painter().rect_filled(rect, 7.0, p.accent_soft);
                    } else if resp.hovered() {
                        ui.painter().rect_filled(rect, 7.0, p.hover);
                    }

                    let indent = 6.0 + depth as f32 * 14.0;
                    let chevron_x = rect.left() + indent;

                    if row.has_children && !searching {
                        theme::chevron(
                            ui.painter(),
                            egui::pos2(chevron_x + 5.0, rect.center().y),
                            is_collapsed,
                            p.muted,
                        );
                    } else {
                        ui.painter().circle_filled(
                            egui::pos2(chevron_x + 5.0, rect.center().y),
                            3.0,
                            row.kind.color(p),
                        );
                    }

                    let value_text = if row.value == 0.0 {
                        "—".to_string()
                    } else {
                        f2gr(row.value)
                    };
                    let value_color = if row.value == 0.0 {
                        p.muted
                    } else if row.value < 0.0 {
                        p.neg
                    } else {
                        p.text
                    };
                    let value_font = egui::FontId::new(12.5, egui::FontFamily::Monospace);
                    let value_w = ui
                        .painter()
                        .layout_no_wrap(value_text.clone(), value_font.clone(), value_color)
                        .rect
                        .width();

                    let text_left = chevron_x + 16.0;
                    let text_right = rect.right() - value_w - 16.0;
                    if text_right > text_left {
                        let clip = egui::Rect::from_min_max(
                            egui::pos2(text_left, rect.top()),
                            egui::pos2(text_right, rect.bottom()),
                        );
                        let (name_color, family) = if selected {
                            (p.accent, egui::FontFamily::Name(theme::BOLD.into()))
                        } else if depth == 0 {
                            (p.text, egui::FontFamily::Name(theme::BOLD.into()))
                        } else {
                            (p.text, egui::FontFamily::Proportional)
                        };
                        ui.painter().with_clip_rect(clip).text(
                            egui::pos2(text_left, rect.center().y),
                            egui::Align2::LEFT_CENTER,
                            label,
                            egui::FontId::new(13.5, family),
                            name_color,
                        );
                    }

                    ui.painter().text(
                        egui::pos2(rect.right() - 8.0, rect.center().y),
                        egui::Align2::RIGHT_CENTER,
                        value_text,
                        value_font,
                        value_color,
                    );

                    if resp.clicked() {
                        // clicking the chevron folds, clicking the row selects
                        let on_chevron = row.has_children
                            && !searching
                            && resp
                                .interact_pointer_pos()
                                .is_some_and(|pos| pos.x < chevron_x + 14.0);
                        if on_chevron {
                            toggle = Some(row.name.clone());
                        } else {
                            select = Some(row.name.clone());
                        }
                    }
                    if resp.double_clicked() && row.has_children && !searching {
                        toggle = Some(row.name.clone());
                    }
                    resp.on_hover_text(&row.name);
                }
            });

        if let Some(name) = select {
            self.selected_account = name;
        }
        if let Some(name) = toggle {
            if !self.collapsed.remove(&name) {
                self.collapsed.insert(name);
            }
        }
    }
}

// ------------------------------------------------------- kartella (centre)

impl QHomeAccApp {
    fn draw_main(&mut self, ui: &mut egui::Ui, p: &Palette) {
        egui::CentralPanel::default()
            .frame(
                egui::Frame::new()
                    .fill(p.bg)
                    .inner_margin(egui::Margin::symmetric(14, 12)),
            )
            .show_inside(ui, |ui| {
                if self.book.is_none() {
                    self.draw_welcome(ui, p);
                    return;
                }
                if self.selected_account.is_empty() {
                    self.draw_empty_selection(ui, p);
                    return;
                }
                self.draw_account_header(ui, p);
                ui.add_space(12.0);
                self.draw_kartella_table(ui, p);
            });
    }

    fn draw_welcome(&mut self, ui: &mut egui::Ui, p: &Palette) {
        ui.vertical_centered(|ui| {
            ui.add_space(ui.available_height() * 0.26);
            theme::card(p).show(ui, |ui| {
                ui.set_max_width(430.0);
                ui.vertical_centered(|ui| {
                    ui.label(theme::bold("rhomeaccount", 13.0).color(p.accent));
                    ui.add_space(6.0);
                    ui.label(theme::bold("Κανένα ανοιχτό βιβλίο", 19.0));
                    ui.add_space(4.0);
                    ui.label(
                        RichText::new(
                            "Διάλεξε τον φάκελο του βιβλίου — αυτόν που περιέχει \
                             το αρχείο 000 και τα ημερολόγια.",
                        )
                        .color(p.muted),
                    );
                    ui.add_space(14.0);
                    let open =
                        egui::Button::new(theme::bold("Άνοιγμα βιβλίου…", 14.0).color(p.on_accent))
                            .fill(p.accent)
                            .stroke(egui::Stroke::NONE)
                            .min_size(egui::vec2(190.0, 34.0));
                    if ui.add(open).clicked() {
                        if let Some(dir) = rfd::FileDialog::new()
                            .set_title("Άνοιγμα βιβλίου")
                            .pick_folder()
                        {
                            self.open_book(dir.to_string_lossy().to_string());
                        }
                    }
                });
            });
        });
    }

    fn draw_empty_selection(&self, ui: &mut egui::Ui, p: &Palette) {
        ui.vertical_centered(|ui| {
            ui.add_space(ui.available_height() * 0.3);
            ui.label(
                RichText::new("Διάλεξε λογαριασμό από το ισοζύγιο")
                    .size(15.0)
                    .color(p.muted),
            );
            ui.add_space(4.0);
            ui.label(
                RichText::new("Διπλό κλικ σε μια κίνηση δείχνει ολόκληρο το άρθρο")
                    .small()
                    .color(p.muted),
            );
        });
    }

    fn draw_account_header(&self, ui: &mut egui::Ui, p: &Palette) {
        let (debit, credit, balance) = self.kart_totals;
        let kind = self.selected_kind();
        let accent = kind.color(p);
        let account = self.selected_account.clone();
        let (parent, leaf) = match account.rfind('.') {
            Some(i) => (&account[..=i], &account[i + 1..]),
            None => ("", account.as_str()),
        };

        // Widths are handed out explicitly. Letting egui's nested left/right
        // layouts negotiate here makes the block overflow its panel.
        const CARD_H: f32 = 48.0;
        const STATS_W: f32 = 428.0;

        theme::card(p).show(ui, |ui| {
            ui.set_height(CARD_H);
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 0.0;

                let (rail, _) = ui.allocate_exact_size(egui::vec2(4.0, 40.0), egui::Sense::hover());
                ui.painter().rect_filled(rail, 2.0, accent);
                ui.add_space(12.0);

                let name_w = (ui.available_width() - STATS_W).max(140.0);
                ui.allocate_ui_with_layout(
                    egui::vec2(name_w, CARD_H),
                    egui::Layout::top_down(egui::Align::Min),
                    |ui| {
                        ui.spacing_mut().item_spacing.y = 4.0;
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = 0.0;
                            ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Truncate);
                            if !parent.is_empty() {
                                ui.label(RichText::new(parent).size(16.0).color(p.muted));
                            }
                            ui.label(theme::bold(leaf, 16.0));
                        });
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = 8.0;
                            theme::chip(ui, kind_label(kind), accent, p.surface_alt);
                            if let Some(eos) = self.eos() {
                                ui.label(
                                    RichText::new(format!("έως {}", eos)).small().color(p.muted),
                                );
                            }
                        });
                    },
                );

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.spacing_mut().item_spacing.x = 26.0;
                    theme::stat(
                        ui,
                        p,
                        "ΥΠΟΛΟΙΠΟ",
                        &fmt_or_dash(balance),
                        if balance < 0.0 { p.neg } else { p.text },
                        true,
                    );
                    theme::stat(ui, p, "ΠΙΣΤΩΣΗ", &fmt_or_dash(credit), p.neg, false);
                    theme::stat(ui, p, "ΧΡΕΩΣΗ", &fmt_or_dash(debit), p.pos, false);
                });
            });
        });
    }

    fn draw_kartella_table(&mut self, ui: &mut egui::Ui, p: &Palette) {
        let rows = self.kart_cache.clone();
        if rows.is_empty() {
            ui.vertical_centered(|ui| {
                ui.add_space(40.0);
                ui.label(RichText::new("Καμία κίνηση στην περίοδο").color(p.muted));
            });
            return;
        }

        let mut pending: Option<i32> = None;
        let p = *p;

        theme::card(&p)
            .inner_margin(egui::Margin::symmetric(6, 6))
            .show(ui, |ui| {
                TableBuilder::new(ui)
                    .striped(true)
                    // so that double-clicking anywhere on a row opens its article
                    .sense(egui::Sense::click())
                    .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
                    .column(Column::exact(48.0)) // Νο
                    .column(Column::exact(88.0)) // Ημ/νία
                    .column(Column::exact(92.0)) // Χρέωση
                    .column(Column::exact(92.0)) // Πίστωση
                    .column(Column::exact(100.0)) // Υπόλοιπο
                    .column(Column::remainder().at_least(160.0)) // Περιγραφή
                    .column(Column::initial(150.0).at_least(60.0)) // Σχόλιο
                    .column(Column::initial(110.0).at_least(50.0)) // Παρ/κό
                    .header(26.0, |mut header| {
                        for (i, h) in KARTELLA_HEADERS.iter().enumerate() {
                            header.col(|ui| {
                                let text = theme::bold(*h, 11.5).color(p.muted);
                                if RIGHT_ALIGNED[i] {
                                    ui.with_layout(
                                        egui::Layout::right_to_left(egui::Align::Center),
                                        |ui| ui.label(text),
                                    );
                                } else {
                                    ui.label(text);
                                }
                            });
                        }
                    })
                    .body(|body| {
                        body.rows(24.0, rows.len(), |mut row| {
                            let k = &rows[row.index()];

                            row.col(|ui| {
                                ui.label(theme::mono(&k.id_str, 12.5).color(p.muted));
                            });
                            row.col(|ui| {
                                ui.label(theme::mono(&k.date, 12.5).color(p.muted));
                            });
                            num_cell(&mut row, theme::mono(&k.debit, 13.0).color(p.pos));
                            num_cell(&mut row, theme::mono(&k.credit, 13.0).color(p.neg));
                            num_cell(
                                &mut row,
                                theme::mono_bold(&k.balance, 13.0)
                                    .color(if k.balance_value < 0.0 { p.neg } else { p.text }),
                            );
                            row.col(|ui| {
                                ui.add(egui::Label::new(&k.perigrafi).truncate());
                            });
                            row.col(|ui| {
                                ui.add(
                                    egui::Label::new(
                                        RichText::new(&k.sxolio).small().color(p.muted),
                                    )
                                    .truncate(),
                                );
                            });
                            row.col(|ui| {
                                ui.add(
                                    egui::Label::new(
                                        theme::mono(&k.parastatiko, 11.0).color(p.muted),
                                    )
                                    .truncate(),
                                );
                            });

                            if row.response().double_clicked() {
                                pending = Some(k.id);
                            }
                        });
                    });
            });

        if pending.is_some() {
            self.pending_transaction = pending;
        }
    }
}

fn num_cell(row: &mut egui_extras::TableRow<'_, '_>, text: RichText) {
    row.col(|ui| {
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(text)
        });
    });
}

/// Width of a numeric column inside a popup grid.
const NUM_COL_W: f32 = 96.0;

/// A right-aligned cell inside an `egui::Grid` (the table equivalent is
/// `num_cell`).
///
/// The width is handed out explicitly: a bare right-to-left layout claims all
/// of `available_width()`, which makes the last column of a grid drift to the
/// window edge and — because the window sizes itself to its content — grow
/// wider on every frame.
fn num_grid_cell(ui: &mut egui::Ui, text: RichText) {
    ui.allocate_ui_with_layout(
        egui::vec2(NUM_COL_W, ui.spacing().interact_size.y),
        egui::Layout::right_to_left(egui::Align::Center),
        |ui| {
            ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Extend);
            ui.label(text)
        },
    );
}

fn fmt_or_dash(v: f64) -> String {
    if v == 0.0 {
        "—".to_string()
    } else {
        f2gr(v)
    }
}

fn kind_label(kind: Kind) -> &'static str {
    match kind {
        Kind::Pagia => "Πάγια",
        Kind::Apothemata => "Αποθέματα",
        Kind::Apaitiseis => "Απαιτήσεις",
        Kind::Kefalaio => "Κεφάλαιο",
        Kind::Ypoxreoseis => "Υποχρεώσεις",
        Kind::Fpa => "ΦΠΑ",
        Kind::Ejoda => "Έξοδα",
        Kind::Esoda => "Έσοδα",
        Kind::Anorgana => "Ανόργανα",
        Kind::Other => "Λοιπά",
    }
}

// ---------------------------------------------------------------- chart

/// Turns a `date_groups` key into something a human reads: "2404" -> "04/24",
/// "244" -> "Τ4/24".
fn period_label(period: Period, key: &str) -> String {
    let len = key.len();
    match period {
        Period::Etos => key.to_string(),
        Period::Minas if len == 4 => format!("{}/{}", &key[2..4], &key[0..2]),
        Period::Trimino if len == 3 => format!("Τ{}/{}", &key[2..3], &key[0..2]),
        Period::Ejamino if len == 3 => format!("Ε{}/{}", &key[2..3], &key[0..2]),
        _ => key.to_string(),
    }
}

/// Axis labels: 12500 -> "12,5κ", 1200000 -> "1,2εκ".
fn compact_number(v: f64) -> String {
    let a = v.abs();
    let sign = if v < 0.0 { "-" } else { "" };
    if a >= 1_000_000.0 {
        format!("{}{:.1}εκ", sign, a / 1_000_000.0).replace('.', ",")
    } else if a >= 1_000.0 {
        format!("{}{:.1}κ", sign, a / 1_000.0).replace('.', ",")
    } else if a == 0.0 {
        "0".to_string()
    } else {
        format!("{}{:.0}", sign, a)
    }
}

impl QHomeAccApp {
    fn draw_chart_panel(&mut self, ui: &mut egui::Ui, p: &Palette) {
        let account = self.selected_account.clone();
        let series = self.series_cache.clone();
        let kind = self.selected_kind();
        let cumulative = CUMULATIVE_KINDS.contains(&kind);
        let period = self.period;

        egui::TopBottomPanel::bottom("chart")
            .exact_height(258.0)
            .frame(
                egui::Frame::new()
                    .fill(p.surface)
                    .inner_margin(egui::Margin::symmetric(16, 12)),
            )
            .show_inside(ui, |ui| {
                if account.is_empty() || series.is_empty() {
                    ui.centered_and_justified(|ui| {
                        ui.label(
                            RichText::new("Το γράφημα εμφανίζεται όταν διαλέξεις λογαριασμό")
                                .color(p.muted),
                        );
                    });
                    return;
                }

                // the value each bar / point represents
                let values: Vec<f64> = series
                    .iter()
                    .map(|(_, total, delta)| if cumulative { *total } else { *delta })
                    .collect();
                let latest = values.last().copied().unwrap_or(0.0);
                let peak =
                    values
                        .iter()
                        .copied()
                        .fold(0.0_f64, |a, b| if b.abs() > a.abs() { b } else { a });
                let active: Vec<f64> = values.iter().copied().filter(|v| *v != 0.0).collect();
                let mean = if active.is_empty() {
                    0.0
                } else {
                    active.iter().sum::<f64>() / active.len() as f64
                };
                let total: f64 = values.iter().sum();

                ui.horizontal(|ui| {
                    ui.label(theme::bold(
                        if cumulative {
                            "Εξέλιξη υπολοίπου"
                        } else {
                            "Κίνηση ανά περίοδο"
                        },
                        14.5,
                    ));
                    ui.label(RichText::new(&account).small().color(p.muted));

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui
                            .add(egui::Button::new(RichText::new("Επαναφορά").small()).frame(false))
                            .on_hover_text("Επαναφορά μεγέθυνσης")
                            .clicked()
                        {
                            self.plot_reset = true;
                        }
                        ui.add_space(8.0);
                        mini_stat(ui, p, "ΜΕΓΙΣΤΟ", &fmt_or_dash(peak));
                        mini_stat(ui, p, "Μ.Ο.", &fmt_or_dash(round2(mean)));
                        mini_stat(
                            ui,
                            p,
                            if cumulative {
                                "ΤΡΕΧΟΝ"
                            } else {
                                "ΣΥΝΟΛΟ"
                            },
                            &fmt_or_dash(if cumulative { latest } else { round2(total) }),
                        );
                    });
                });
                ui.add_space(8.0);

                let accent = kind.color(p);
                let labels: Vec<String> = series
                    .iter()
                    .map(|(k, _, _)| period_label(period, k))
                    .collect();
                let n = labels.len() as isize;
                let x_labels = labels.clone();
                let hover_labels = labels;
                let reset = std::mem::take(&mut self.plot_reset);
                let dark = p.is_dark();
                let (border, neg) = (p.border, p.neg);

                Plot::new("account_plot")
                    // panning and zooming matter here: a monthly series can be
                    // hundreds of periods wide
                    .allow_drag([true, false])
                    .allow_zoom([true, false])
                    .allow_scroll([true, false])
                    .allow_boxed_zoom(false)
                    .show_background(false)
                    .show_grid([false, true])
                    .y_axis_min_width(62.0)
                    .set_margin_fraction(egui::vec2(0.01, 0.18))
                    // The x axis is an index, so egui_plot's round-number ticks
                    // land almost nowhere. Tick on whole periods instead, at a
                    // stride that keeps roughly ten labels on screen.
                    .x_grid_spacer(|input| {
                        let (min, max) = input.bounds;
                        let span = (max - min).max(1.0);
                        let step = (span / 10.0).ceil().max(1.0);
                        let mut marks = Vec::new();
                        let mut idx = (min / step).floor() * step;
                        while idx <= max + step {
                            if idx >= 0.0 {
                                marks.push(egui_plot::GridMark {
                                    value: idx + 0.5,
                                    step_size: step,
                                });
                            }
                            idx += step;
                        }
                        marks
                    })
                    // Bar i sits at x = i + 0.5, so the tick at x falls inside
                    // bar floor(x); egui_plot already thins the ticks out.
                    .x_axis_formatter(move |mark, _| {
                        let idx = mark.value.floor() as isize;
                        if idx < 0 || idx >= n {
                            return String::new();
                        }
                        x_labels[idx as usize].clone()
                    })
                    .y_axis_formatter(|mark, _| compact_number(mark.value))
                    .label_formatter(move |_, point| {
                        let idx = point.x.floor() as isize;
                        let label = if idx >= 0 && idx < n {
                            hover_labels[idx as usize].as_str()
                        } else {
                            ""
                        };
                        format!("{}\n{}", label, f2gr(point.y))
                    })
                    .show(ui, |plot_ui| {
                        if reset {
                            plot_ui.set_auto_bounds([true, true]);
                        }

                        // a zero line, so negative movement reads at a glance
                        plot_ui.hline(HLine::new("", 0.0).color(border).width(1.0_f32));

                        if cumulative {
                            // a balance is a continuous quantity: draw it as a
                            // filled line rather than disconnected bars
                            let points: PlotPoints = values
                                .iter()
                                .enumerate()
                                .map(|(i, v)| [i as f64 + 0.5, *v])
                                .collect();
                            plot_ui.line(
                                Line::new("", points)
                                    .color(accent)
                                    .width(2.0_f32)
                                    .fill(0.0_f32)
                                    .fill_alpha(if dark { 0.20_f32 } else { 0.14_f32 }),
                            );
                        } else {
                            let bars: Vec<Bar> = values
                                .iter()
                                .enumerate()
                                .map(|(i, v)| {
                                    Bar::new(i as f64 + 0.5, *v).width(0.68).fill(if *v < 0.0 {
                                        neg
                                    } else {
                                        accent
                                    })
                                })
                                .collect();
                            plot_ui.bar_chart(BarChart::new("", bars));
                        }
                    });
            });
    }
}

/// A compact right-aligned label/value pair for the chart header.
fn mini_stat(ui: &mut egui::Ui, p: &Palette, label: &str, value: &str) {
    ui.allocate_ui_with_layout(
        egui::vec2(104.0, 34.0),
        egui::Layout::top_down(egui::Align::Max),
        |ui| {
            ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Extend);
            ui.spacing_mut().item_spacing.y = 1.0;
            ui.label(RichText::new(label).small().color(p.muted));
            ui.label(theme::mono_bold(value, 13.0).color(p.text));
        },
    );
}

// ---------------------------------------------------------------- popups

impl QHomeAccApp {
    fn draw_popups(&mut self, ctx: &egui::Context) {
        let p = self.palette();

        // transaction viewer
        if let Some(trn) = self.show_transaction.clone() {
            let mut open = true;
            let title = format!("Άρθρο {}", trn.id);
            egui::Window::new(&title)
                .title_bar(false)
                .resizable(true)
                .default_width(620.0)
                .show(ctx, |ui| {
                    if theme::popup_header(ui, &p, &title) {
                        open = false;
                    }
                    ui.horizontal(|ui| {
                        ui.label(
                            theme::mono_bold(trn.date.format("%Y-%m-%d").to_string(), 13.0)
                                .color(p.text),
                        );
                        if !trn.parastatiko.is_empty() {
                            theme::chip(ui, &trn.parastatiko, p.accent, p.accent_soft);
                        }
                    });
                    ui.label(RichText::new(&trn.perigrafi).size(15.0));
                    ui.add_space(8.0);
                    ui.separator();
                    ui.add_space(4.0);

                    egui::ScrollArea::vertical()
                        .id_salt("arthro_scroll")
                        .max_height(POPUP_MAX_H)
                        .show(ui, |ui| {
                            egui::Grid::new("arthro_grid")
                                .striped(true)
                                .num_columns(4)
                                .spacing([20.0, 5.0])
                                .min_col_width(90.0)
                                .show(ui, |ui| {
                                    for h in ["Λογ/μός", "Περιγραφή"] {
                                        ui.label(theme::bold(h, 11.5).color(p.muted));
                                    }
                                    for h in ["Χρέωση", "Πίστωση"] {
                                        num_grid_cell(ui, theme::bold(h, 11.5).color(p.muted));
                                    }
                                    ui.end_row();

                                    for lin in &trn.lines {
                                        ui.label(&lin.account_name);
                                        ui.label(RichText::new(&lin.sxolio).color(p.muted));
                                        num_grid_cell(
                                            ui,
                                            theme::mono(f2gr(lin.debit()), 13.0).color(p.pos),
                                        );
                                        num_grid_cell(
                                            ui,
                                            theme::mono(f2gr(lin.credit()), 13.0).color(p.neg),
                                        );
                                        ui.end_row();
                                    }

                                    if trn.lines.len() > 2 {
                                        let td: f64 = trn.lines.iter().map(|l| l.debit()).sum();
                                        let tc: f64 = trn.lines.iter().map(|l| l.credit()).sum();
                                        ui.label(theme::bold("Σύνολα", 14.0));
                                        ui.label("");
                                        num_grid_cell(
                                            ui,
                                            theme::mono_bold(f2gr(td), 13.0).color(p.text),
                                        );
                                        num_grid_cell(
                                            ui,
                                            theme::mono_bold(f2gr(tc), 13.0).color(p.text),
                                        );
                                        ui.end_row();
                                    }
                                });
                        });
                });
            if !open {
                self.show_transaction = None;
            }
        }

        // validation report
        if self.show_validations {
            let mut open = true;
            let rows = self.validation_rows.clone();
            let n_bad = rows.iter().filter(|r| !r.5).count();
            egui::Window::new("Έλεγχος υπολοίπων")
                .title_bar(false)
                .resizable(true)
                .default_width(700.0)
                .show(ctx, |ui| {
                    if theme::popup_header(ui, &p, "Έλεγχος υπολοίπων") {
                        open = false;
                    }
                    ui.horizontal(|ui| {
                        if n_bad == 0 {
                            theme::chip(
                                ui,
                                &format!("{} έλεγχοι OK", rows.len()),
                                p.pos,
                                p.surface_alt,
                            );
                        } else {
                            theme::chip(
                                ui,
                                &format!("{} από {} απέτυχαν", n_bad, rows.len()),
                                p.neg,
                                p.surface_alt,
                            );
                        }
                    });
                    ui.add_space(8.0);

                    egui::ScrollArea::vertical()
                        .id_salt("validations_scroll")
                        .max_height(POPUP_MAX_H)
                        .show(ui, |ui| {
                            egui::Grid::new("validations_grid")
                                .striped(true)
                                .num_columns(6)
                                .spacing([18.0, 5.0])
                                .show(ui, |ui| {
                                    for h in ["", "Ημ/νία", "Λογ/μός"] {
                                        ui.label(theme::bold(h, 11.5).color(p.muted));
                                    }
                                    for h in ["Υπόλοιπο", "Αναμενόμενο", "Διαφορά"] {
                                        num_grid_cell(ui, theme::bold(h, 11.5).color(p.muted));
                                    }
                                    ui.end_row();

                                    for (dat, acc, ypol, poso, diaf, ok) in &rows {
                                        theme::dot(ui, if *ok { p.pos } else { p.neg }, 4.5);
                                        ui.label(theme::mono(dat, 13.0).color(p.text));
                                        ui.label(acc);
                                        num_grid_cell(
                                            ui,
                                            theme::mono(f2gr(*ypol), 13.0).color(p.text),
                                        );
                                        num_grid_cell(
                                            ui,
                                            theme::mono(f2gr(*poso), 13.0).color(p.text),
                                        );
                                        num_grid_cell(
                                            ui,
                                            theme::mono(f2gr(*diaf), 13.0)
                                                .color(if *ok { p.muted } else { p.neg }),
                                        );
                                        ui.end_row();
                                    }
                                });
                        });
                });
            if !open {
                self.show_validations = false;
            }
        }

        // yearly results — έσοδα μείον έξοδα ανά χρήση
        if self.show_results {
            let mut open = true;
            let rows = self.result_rows.clone();
            let eos = self.eos();
            let t_esoda = round2(rows.iter().map(|r| r.esoda).sum());
            let t_ejoda = round2(rows.iter().map(|r| r.ejoda).sum());
            let t_apotelesma = round2(t_esoda - t_ejoda);
            egui::Window::new("Αποτελέσματα ανά έτος")
                .title_bar(false)
                .resizable(true)
                .default_width(460.0)
                .show(ctx, |ui| {
                    if theme::popup_header(ui, &p, "Αποτελέσματα ανά έτος") {
                        open = false;
                    }
                    if rows.is_empty() {
                        ui.label(RichText::new("Καμία κίνηση εσόδων ή εξόδων").color(p.muted));
                        return;
                    }
                    ui.horizontal(|ui| {
                        theme::chip(
                            ui,
                            &format!("{} χρήσεις", rows.len()),
                            p.accent,
                            p.accent_soft,
                        );
                        if let Some(eos) = &eos {
                            ui.label(RichText::new(format!("έως {}", eos)).small().color(p.muted));
                        }
                    });
                    ui.add_space(8.0);

                    // Only the year rows scroll; the totals stay pinned below
                    // them, which is the whole point of a totals line.
                    egui::ScrollArea::vertical()
                        .id_salt("results_scroll")
                        .max_height(POPUP_MAX_H)
                        .show(ui, |ui| {
                            egui::Grid::new("results_grid")
                                .striped(true)
                                .num_columns(4)
                                .spacing([18.0, 5.0])
                                .min_col_width(64.0)
                                .show(ui, |ui| {
                                    ui.label(theme::bold("Έτος", 11.5).color(p.muted));
                                    for h in ["Έσοδα", "Έξοδα", "Αποτέλεσμα"] {
                                        num_grid_cell(ui, theme::bold(h, 11.5).color(p.muted));
                                    }
                                    ui.end_row();

                                    for r in &rows {
                                        ui.label(
                                            theme::mono(r.year.to_string(), 13.0).color(p.text),
                                        );
                                        num_grid_cell(
                                            ui,
                                            theme::mono(fmt_or_dash(r.esoda), 13.0).color(p.pos),
                                        );
                                        num_grid_cell(
                                            ui,
                                            theme::mono(fmt_or_dash(r.ejoda), 13.0).color(p.neg),
                                        );
                                        num_grid_cell(
                                            ui,
                                            theme::mono_bold(fmt_or_dash(r.apotelesma), 13.0)
                                                .color(if r.apotelesma < 0.0 { p.neg } else { p.pos }),
                                        );
                                        ui.end_row();
                                    }
                                });
                        });

                    ui.add_space(2.0);
                    ui.separator();
                    egui::Grid::new("results_total_grid")
                        .num_columns(4)
                        .spacing([18.0, 5.0])
                        .min_col_width(64.0)
                        .show(ui, |ui| {
                            ui.label(theme::bold("ΣΥΝΟΛΟ", 12.0).color(p.text));
                            num_grid_cell(
                                ui,
                                theme::mono_bold(fmt_or_dash(t_esoda), 13.0).color(p.pos),
                            );
                            num_grid_cell(
                                ui,
                                theme::mono_bold(fmt_or_dash(t_ejoda), 13.0).color(p.neg),
                            );
                            num_grid_cell(
                                ui,
                                theme::mono_bold(fmt_or_dash(t_apotelesma), 13.0)
                                    .color(if t_apotelesma < 0.0 { p.neg } else { p.pos }),
                            );
                            ui.end_row();
                        });
                });
            if !open {
                self.show_results = false;
            }
        }

        // parse errors
        if self.show_errors && !self.parse_errors.is_empty() {
            let mut open = true;
            let errors = self.parse_errors.clone();
            egui::Window::new("Λάθη ανάγνωσης")
                .title_bar(false)
                .resizable(true)
                .default_width(760.0)
                .show(ctx, |ui| {
                    if theme::popup_header(ui, &p, "Λάθη ανάγνωσης") {
                        open = false;
                    }
                    ui.label(
                        RichText::new(format!(
                            "{} γραμμές δεν διαβάστηκαν σωστά. Το βιβλίο άνοιξε ούτως ή άλλως.",
                            errors.len()
                        ))
                        .color(p.muted),
                    );
                    ui.add_space(8.0);
                    egui::ScrollArea::vertical()
                        .id_salt("errors_scroll")
                        .max_height(POPUP_MAX_H)
                        .show(ui, |ui| {
                            for e in &errors {
                                ui.label(RichText::new(e).small().color(p.warn));
                            }
                        });
                });
            if !open {
                self.show_errors = false;
            }
        }

        // fatal open error
        if let Some(err) = self.fatal_error.clone() {
            let mut open = true;
            egui::Window::new("Σφάλμα")
                .title_bar(false)
                .resizable(false)
                .show(ctx, |ui| {
                    if theme::popup_header(ui, &p, "Σφάλμα") {
                        open = false;
                    }
                    ui.label(RichText::new(err).color(p.neg));
                });
            if !open {
                self.fatal_error = None;
            }
        }
    }
}
