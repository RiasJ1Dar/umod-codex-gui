//! Візуальні токени вікна і базовий стиль egui.
//!
//! Джерело правди для кольорів, метрики і дрібних помічників малювання.
//! Мокапи редизайну: артборд «Токени» на канві проєкту.

use eframe::egui::{self, Color32, CornerRadius, FontId, Margin, Stroke};

// --- поверхні --------------------------------------------------------------

/// Тло вікна.
pub const BG: Color32 = Color32::from_rgb(0x15, 0x17, 0x1C);
/// Смуга шапки під заголовком ОС.
pub const HEADER: Color32 = Color32::from_rgb(0x1A, 0x1D, 0x23);
/// Картка статусу, друга кнопка.
pub const CARD: Color32 = Color32::from_rgb(0x1E, 0x21, 0x27);
/// Наведення на другу кнопку.
pub const CARD_HOVER: Color32 = Color32::from_rgb(0x26, 0x2A, 0x31);
/// Натиснута друга кнопка.
pub const CARD_ACTIVE: Color32 = Color32::from_rgb(0x2E, 0x33, 0x3B);
/// Поле вводу, згорнутий список.
pub const FIELD: Color32 = Color32::from_rgb(0x14, 0x17, 0x1B);
/// Тло журналу — найглибша поверхня.
pub const LOG_BG: Color32 = Color32::from_rgb(0x10, 0x12, 0x15);

// --- лінії -----------------------------------------------------------------

pub const LINE: Color32 = Color32::from_rgb(0x2C, 0x30, 0x37);
pub const LINE_SOFT: Color32 = Color32::from_rgb(0x26, 0x2A, 0x31);
pub const LINE_HOVER: Color32 = Color32::from_rgb(0x3A, 0x40, 0x4A);

// --- текст -----------------------------------------------------------------

pub const TEXT: Color32 = Color32::from_rgb(0xE4, 0xE6, 0xEA);
pub const TEXT_2: Color32 = Color32::from_rgb(0xC6, 0xCB, 0xD3);
pub const TEXT_DIM: Color32 = Color32::from_rgb(0x8C, 0x94, 0xA0);

// --- сигнали ---------------------------------------------------------------

/// Акцент: головна кнопка і стан «працює».
pub const ACCENT: Color32 = Color32::from_rgb(0x62, 0xA9, 0x7B);
/// Текст на темному тлі в акцентному стані.
pub const ACCENT_TEXT: Color32 = Color32::from_rgb(0x8F, 0xCB, 0xA4);
/// Тло картки статусу, коли проксі працює.
pub const ACCENT_BG: Color32 = Color32::from_rgb(0x1B, 0x26, 0x21);
/// Рамка тієї ж картки.
pub const ACCENT_LINE: Color32 = Color32::from_rgb(0x2F, 0x47, 0x39);
/// Другорядний текст на акцентному тлі.
pub const ACCENT_DIM: Color32 = Color32::from_rgb(0x93, 0xA7, 0x9A);
/// Текст на заливці акценту.
pub const ON_ACCENT: Color32 = Color32::from_rgb(0x0E, 0x13, 0x11);

pub const WARN: Color32 = Color32::from_rgb(0xD8, 0xA6, 0x48);
pub const WARN_LINE: Color32 = Color32::from_rgb(0x8A, 0x6A, 0x28);
pub const WARN_BG: Color32 = Color32::from_rgb(0x2A, 0x24, 0x18);
pub const WARN_ICON: Color32 = Color32::from_rgb(0xE0, 0xB9, 0x6A);

pub const ERR: Color32 = Color32::from_rgb(0xDE, 0x6F, 0x65);
pub const ERR_TEXT: Color32 = Color32::from_rgb(0xE8, 0x86, 0x7C);

// --- неактивний контрол ----------------------------------------------------

pub const DISABLED_BG: Color32 = Color32::from_rgb(0x19, 0x1C, 0x21);
pub const DISABLED_LINE: Color32 = Color32::from_rgb(0x23, 0x27, 0x2D);
pub const DISABLED_TEXT: Color32 = Color32::from_rgb(0x5E, 0x64, 0x6E);

// --- метрика ---------------------------------------------------------------

/// Висота смуги шапки.
pub const HEADER_H: f32 = 40.0;
/// Поля навколо вмісту вікна.
pub const PAD: i8 = 20;
/// Висота звичайного контрола.
pub const CONTROL_H: f32 = 38.0;
/// Висота головної кнопки.
pub const PRIMARY_H: f32 = 46.0;
/// Заокруглення картки.
pub const R_CARD: u8 = 10;
/// Заокруглення поля і кнопки.
pub const R_CONTROL: u8 = 8;
/// Крок між блоками форми.
pub const GAP: f32 = 16.0;

/// Прописує токени у стиль контексту. Викликається один раз при старті.
///
/// Вікно завжди темне: вигляд не залежить від теми системи, бо палітра
/// світлого варіанта не проєктувалась.
pub fn install(ctx: &egui::Context) {
    ctx.set_theme(egui::ThemePreference::Dark);
    ctx.all_styles_mut(apply);
}

fn apply(style: &mut egui::Style) {
    let v = &mut style.visuals;
    v.dark_mode = true;
    v.panel_fill = BG;
    v.window_fill = CARD;
    v.window_stroke = Stroke::new(1.0, LINE);
    v.window_corner_radius = CornerRadius::same(R_CONTROL);
    v.menu_corner_radius = CornerRadius::same(R_CONTROL);
    v.extreme_bg_color = FIELD;
    v.text_edit_bg_color = Some(FIELD);
    v.faint_bg_color = CARD;
    v.code_bg_color = LOG_BG;
    v.override_text_color = Some(TEXT);
    v.warn_fg_color = WARN;
    v.error_fg_color = ERR;
    v.hyperlink_color = ACCENT_TEXT;
    v.selection.bg_fill = ACCENT_LINE;
    v.selection.stroke = Stroke::new(1.0, TEXT);

    // Одна рамка і одне заокруглення на всі стани: контрол не має
    // «дихати» під курсором, міняється лише заливка.
    for (w, fill, line, fg) in [
        (&mut v.widgets.noninteractive, CARD, LINE, TEXT),
        (&mut v.widgets.inactive, CARD, LINE, TEXT_2),
        (&mut v.widgets.hovered, CARD_HOVER, LINE_HOVER, TEXT),
        (&mut v.widgets.active, CARD_ACTIVE, LINE_HOVER, TEXT),
        (&mut v.widgets.open, CARD_HOVER, LINE_HOVER, TEXT),
    ] {
        w.bg_fill = fill;
        w.weak_bg_fill = fill;
        w.bg_stroke = Stroke::new(1.0, line);
        w.fg_stroke = Stroke::new(1.0, fg);
        w.corner_radius = CornerRadius::same(R_CONTROL);
        w.expansion = 0.0;
    }
    // Поле вводу і згорнутий список сидять глибше за картку.
    v.widgets.inactive.bg_fill = FIELD;

    let s = &mut style.spacing;
    s.item_spacing = egui::vec2(8.0, 8.0);
    s.button_padding = egui::vec2(12.0, 8.0);
    s.interact_size = egui::vec2(40.0, CONTROL_H);
    s.combo_height = 280.0;
    s.menu_margin = Margin::same(6);
    s.scroll.bar_width = 6.0;
    s.scroll.bar_inner_margin = 2.0;
    s.scroll.bar_outer_margin = 0.0;

    use egui::{FontFamily, TextStyle};
    style.text_styles = [
        (TextStyle::Heading, FontId::new(15.0, FontFamily::Proportional)),
        (TextStyle::Body, FontId::new(14.0, FontFamily::Proportional)),
        (TextStyle::Button, FontId::new(13.0, FontFamily::Proportional)),
        (TextStyle::Small, FontId::new(11.0, FontFamily::Proportional)),
        (TextStyle::Monospace, FontId::new(12.0, FontFamily::Monospace)),
    ]
    .into();
}

// --- помічники малювання ---------------------------------------------------

/// Рамка картки (статус, журнал).
pub fn card(fill: Color32, line: Color32, radius: u8, pad_x: i8, pad_y: i8) -> egui::Frame {
    egui::Frame::new()
        .fill(fill)
        .stroke(Stroke::new(1.0, line))
        .corner_radius(CornerRadius::same(radius))
        .inner_margin(Margin::symmetric(pad_x, pad_y))
}

/// Текст із розрядкою — у egui немає letter-spacing, тож літери
/// викладаються поштучно. Потрібно для заголовка і підписів секцій.
pub fn tracked_text(
    painter: &egui::Painter,
    left_center: egui::Pos2,
    text: &str,
    font: FontId,
    color: Color32,
    tracking: f32,
) -> f32 {
    let mut x = left_center.x;
    for ch in text.chars() {
        let galley = painter.layout_no_wrap(ch.to_string(), font.clone(), color);
        let w = galley.size().x;
        let y = left_center.y - galley.size().y / 2.0;
        painter.galley(egui::pos2(x, y), galley, color);
        x += w + tracking;
    }
    x - left_center.x
}

/// Підпис секції: 10 px, розрядка, приглушений.
/// Повертає смугу, яку зайняв, — у неї можна домалювати дію праворуч.
pub fn section_label(ui: &mut egui::Ui, text: &str) -> egui::Rect {
    let (rect, _) = ui.allocate_exact_size(
        egui::vec2(ui.available_width(), 16.0),
        egui::Sense::hover(),
    );
    tracked_text(
        ui.painter(),
        rect.left_center(),
        text,
        FontId::proportional(10.0),
        TEXT_DIM,
        1.4,
    );
    rect
}
