//! Дрібні намальовані елементи: піктограми та кнопки з ними.
//!
//! Гліфи (▶, ■, ⟳) не використовуються навмисно: у зібраному наборі шрифтів
//! egui їх покриття не гарантоване, і замість піктограми з'являється рамка.
//! Тут усе малюється painter'ом, тож вигляд однаковий на будь-якій системі.

use crate::theme;
use eframe::egui::{self, Color32, CornerRadius, Pos2, Rect, Stroke, StrokeKind, Vec2};

#[derive(Clone, Copy, PartialEq)]
pub enum Icon {
    /// Трикутник — «запустити».
    Play,
    /// Коло зі стрілкою — «підняти проксі».
    Restart,
    /// Квадрат — «зупинити».
    Stop,
    /// Дві рамки — «скопіювати».
    Copy,
    /// Тека — «огляд…».
    Folder,
    /// Трикутник із окликом — попередження.
    Warn,
    /// Кутовий знак у шапці.
    Mark,
}

/// Малює піктограму в коробці `size` × `size` з центром `c`.
pub fn paint_icon(painter: &egui::Painter, c: Pos2, size: f32, icon: Icon, color: Color32) {
    let s = size;
    let stroke = Stroke::new((s / 11.0).max(1.2), color);
    match icon {
        Icon::Play => {
            painter.add(egui::Shape::convex_polygon(
                vec![
                    egui::pos2(c.x - s * 0.26, c.y - s * 0.34),
                    egui::pos2(c.x + s * 0.32, c.y),
                    egui::pos2(c.x - s * 0.26, c.y + s * 0.34),
                ],
                color,
                Stroke::NONE,
            ));
        }
        Icon::Stop => {
            let half = s * 0.28;
            painter.rect_filled(
                Rect::from_center_size(c, Vec2::splat(half * 2.0)),
                CornerRadius::same(1),
                color,
            );
        }
        Icon::Restart => {
            let r = s * 0.34;
            painter.circle_stroke(c, r, stroke);
            // Вістря праворуч угорі — кільце читається як «знову».
            painter.add(egui::Shape::convex_polygon(
                vec![
                    egui::pos2(c.x + r * 0.25, c.y - r * 1.05),
                    egui::pos2(c.x + r * 1.15, c.y - r * 1.05),
                    egui::pos2(c.x + r * 1.15, c.y - r * 0.15),
                ],
                color,
                Stroke::NONE,
            ));
        }
        Icon::Copy => {
            painter.rect_stroke(
                Rect::from_min_size(
                    egui::pos2(c.x - s * 0.08, c.y - s * 0.08),
                    Vec2::splat(s * 0.54),
                ),
                CornerRadius::same(2),
                stroke,
                StrokeKind::Inside,
            );
            painter.rect_stroke(
                Rect::from_min_size(
                    egui::pos2(c.x - s * 0.46, c.y - s * 0.46),
                    Vec2::splat(s * 0.54),
                ),
                CornerRadius::same(2),
                stroke,
                StrokeKind::Inside,
            );
        }
        Icon::Folder => {
            painter.rect_stroke(
                Rect::from_min_size(
                    egui::pos2(c.x - s * 0.44, c.y - s * 0.26),
                    egui::vec2(s * 0.88, s * 0.58),
                ),
                CornerRadius::same(2),
                stroke,
                StrokeKind::Inside,
            );
            painter.line_segment(
                [
                    egui::pos2(c.x - s * 0.44, c.y - s * 0.26),
                    egui::pos2(c.x - s * 0.10, c.y - s * 0.26),
                ],
                stroke,
            );
            painter.line_segment(
                [
                    egui::pos2(c.x - s * 0.30, c.y - s * 0.42),
                    egui::pos2(c.x - s * 0.10, c.y - s * 0.26),
                ],
                stroke,
            );
        }
        Icon::Warn => {
            painter.add(egui::Shape::convex_polygon(
                vec![
                    egui::pos2(c.x, c.y - s * 0.42),
                    egui::pos2(c.x + s * 0.44, c.y + s * 0.38),
                    egui::pos2(c.x - s * 0.44, c.y + s * 0.38),
                ],
                Color32::TRANSPARENT,
                stroke,
            ));
            painter.line_segment(
                [
                    egui::pos2(c.x, c.y - s * 0.10),
                    egui::pos2(c.x, c.y + s * 0.08),
                ],
                stroke,
            );
            painter.circle_filled(egui::pos2(c.x, c.y + s * 0.24), stroke.width * 0.7, color);
        }
        Icon::Mark => {
            painter.rect_stroke(
                Rect::from_center_size(c, Vec2::splat(s * 0.78)),
                CornerRadius::same((s * 0.2) as u8),
                stroke,
                StrokeKind::Inside,
            );
            painter.line_segment(
                [
                    egui::pos2(c.x - s * 0.14, c.y - s * 0.16),
                    egui::pos2(c.x + s * 0.04, c.y),
                ],
                stroke,
            );
            painter.line_segment(
                [
                    egui::pos2(c.x + s * 0.04, c.y),
                    egui::pos2(c.x - s * 0.14, c.y + s * 0.16),
                ],
                stroke,
            );
        }
    }
}

/// Кнопка з піктограмою і підписом. `primary` — суцільна акцентна заливка.
/// Повертає `true`, якщо на неї щойно клацнули.
pub fn action_button(
    ui: &mut egui::Ui,
    enabled: bool,
    primary: bool,
    icon: Icon,
    label: &str,
    width: f32,
) -> bool {
    let height = if primary {
        theme::PRIMARY_H
    } else {
        theme::CONTROL_H
    };
    let sense = if enabled {
        egui::Sense::click()
    } else {
        egui::Sense::hover()
    };
    let (rect, response) = ui.allocate_exact_size(egui::vec2(width, height), sense);

    let hovered = enabled && response.hovered();
    let pressed = enabled && response.is_pointer_button_down_on();

    let (fill, line, fg) = match (primary, enabled, pressed, hovered) {
        (true, false, _, _) => (
            Color32::from_rgb(0x2A, 0x37, 0x30),
            Color32::TRANSPARENT,
            Color32::from_rgb(0x6A, 0x71, 0x67),
        ),
        (true, true, true, _) => (
            Color32::from_rgb(0x53, 0x93, 0x6A),
            Color32::TRANSPARENT,
            theme::ON_ACCENT,
        ),
        (true, true, _, true) => (
            Color32::from_rgb(0x6F, 0xB8, 0x88),
            Color32::TRANSPARENT,
            theme::ON_ACCENT,
        ),
        (true, true, _, _) => (theme::ACCENT, Color32::TRANSPARENT, theme::ON_ACCENT),
        (false, false, _, _) => (
            theme::DISABLED_BG,
            theme::DISABLED_LINE,
            theme::DISABLED_TEXT,
        ),
        (false, true, true, _) => (theme::CARD_ACTIVE, theme::LINE_HOVER, theme::TEXT),
        (false, true, _, true) => (theme::CARD_HOVER, theme::LINE_HOVER, theme::TEXT),
        (false, true, _, _) => (theme::CARD, theme::LINE, theme::TEXT_2),
    };

    let painter = ui.painter();
    let radius = CornerRadius::same(theme::R_CONTROL);
    painter.rect_filled(rect, radius, fill);
    if line != Color32::TRANSPARENT {
        painter.rect_stroke(rect, radius, Stroke::new(1.0, line), StrokeKind::Inside);
    }

    let font = if primary {
        egui::FontId::proportional(15.0)
    } else {
        egui::FontId::proportional(13.0)
    };
    let icon_size = if primary { 14.0 } else { 13.0 };
    let gap = 9.0;
    let galley = painter.layout_no_wrap(label.to_string(), font, fg);
    let total = icon_size + gap + galley.size().x;
    let start_x = rect.center().x - total / 2.0;

    paint_icon(
        painter,
        egui::pos2(start_x + icon_size / 2.0, rect.center().y),
        icon_size,
        icon,
        fg,
    );
    let text_pos = egui::pos2(
        start_x + icon_size + gap,
        rect.center().y - galley.size().y / 2.0,
    );
    painter.galley(text_pos, galley, fg);

    if enabled {
        response.clicked()
    } else {
        false
    }
}

/// Квадратна кнопка лише з піктограмою. `hint` читає екранний диктор.
pub fn icon_button(
    ui: &mut egui::Ui,
    size: Vec2,
    icon: Icon,
    tint: Color32,
    frame: Option<(Color32, Color32)>,
    hint: &str,
) -> bool {
    let (rect, response) = ui.allocate_exact_size(size, egui::Sense::click());
    let painter = ui.painter();
    let radius = CornerRadius::same(theme::R_CONTROL);

    if let Some((fill, line)) = frame {
        let fill = if response.is_pointer_button_down_on() {
            theme::CARD_ACTIVE
        } else if response.hovered() {
            theme::CARD_HOVER
        } else {
            fill
        };
        painter.rect_filled(rect, radius, fill);
        painter.rect_stroke(rect, radius, Stroke::new(1.0, line), StrokeKind::Inside);
    } else if response.hovered() {
        painter.rect_filled(rect, radius, theme::CARD);
    }

    let tint = if response.hovered() { theme::TEXT } else { tint };
    paint_icon(painter, rect.center(), size.y.min(size.x) * 0.60, icon, tint);

    response.on_hover_text(hint).clicked()
}
