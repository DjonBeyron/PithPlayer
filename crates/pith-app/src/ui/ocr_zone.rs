//! Выбор зоны субтитров: рамка поверх кадра.
//!
//! Зона хранится долями самого видео (а не окна): окно можно растягивать,
//! а чёрные поля по краям кадра в зону не входят.

use pith_store::Zone;

use crate::app::PithApp;
use crate::theme;
use crate::tr;

/// Затемнение вокруг зоны.
const DIM: u8 = 150;

pub fn show(app: &mut PithApp, ctx: &egui::Context) {
    if !app.ocr().selecting {
        return;
    }

    let screen = ctx.input(|i| i.viewport_rect());
    let video = video_rect(app, screen);
    let draft = app.ocr().draft.unwrap_or(Zone::DEFAULT);

    let mut next = draft;
    let mut save = false;
    let mut reset = false;
    let mut cancel = ctx.input(|i| i.key_pressed(egui::Key::Escape));

    egui::Area::new(egui::Id::new("ocr_zone_layer"))
        .order(egui::Order::Foreground)
        .fixed_pos(screen.min)
        .show(ctx, |ui| {
            let (rect, response) =
                ui.allocate_exact_size(screen.size(), egui::Sense::click_and_drag());

            // Рамку рисуют перетаскиванием: от места нажатия до курсора.
            if response.dragged()
                && let (Some(start), Some(now)) = (
                    ui.input(|i| i.pointer.press_origin()),
                    response.interact_pointer_pos(),
                )
                && let Some(zone) = zone_from(video, start, now)
            {
                next = zone;
            }

            paint(ui, rect, video, rect_of(video, next));
            ui.output_mut(|o| o.cursor_icon = egui::CursorIcon::Crosshair);
        });

    egui::Area::new(egui::Id::new("ocr_zone_bar"))
        .order(egui::Order::Foreground)
        .fixed_pos(screen.center_top() + egui::vec2(0.0, 24.0))
        .pivot(egui::Align2::CENTER_TOP)
        .show(ctx, |ui| {
            egui::Frame::NONE
                .fill(theme::PANEL_CARD)
                .inner_margin(10.0)
                .corner_radius(6.0)
                .show(ui, |ui| {
                    ui.label(
                        egui::RichText::new(tr!(
                            "Обведите рамкой место субтитров и нажмите «Сохранить»",
                            "Drag a frame around the subtitles and press Save"
                        ))
                        .color(theme::TEXT_PRIMARY),
                    );
                    ui.horizontal(|ui| {
                        save = ui.button(tr!("Сохранить", "Save")).clicked();
                        reset = ui.button(tr!("Сбросить", "Reset")).clicked();
                        cancel |= ui.button(tr!("Отмена", "Cancel")).clicked();
                    });
                });
        });

    app.ocr().draft = Some(next);

    if save {
        app.save_zone(ctx);
    } else if reset {
        app.reset_zone();
    } else if cancel {
        app.ocr().selecting = false;
    }
}

/// Где на экране лежит само видео: вписано в окно по пропорциям.
fn video_rect(app: &mut PithApp, screen: egui::Rect) -> egui::Rect {
    let aspect = app
        .engine()
        .map(|e| e.state())
        .filter(|s| s.display_width > 0 && s.display_height > 0)
        .map_or(16.0 / 9.0, |s| {
            s.display_width as f32 / s.display_height as f32
        });

    let (mut w, mut h) = (screen.width(), screen.width() / aspect);
    if h > screen.height() {
        h = screen.height();
        w = h * aspect;
    }

    egui::Rect::from_center_size(screen.center(), egui::vec2(w, h))
}

/// Зона в долях видео из двух точек экрана; `None` — рамка слишком мала.
fn zone_from(video: egui::Rect, a: egui::Pos2, b: egui::Pos2) -> Option<Zone> {
    let to_frac = |p: egui::Pos2| {
        (
            ((p.x - video.min.x) / video.width()).clamp(0.0, 1.0),
            ((p.y - video.min.y) / video.height()).clamp(0.0, 1.0),
        )
    };
    let ((x1, y1), (x2, y2)) = (to_frac(a), to_frac(b));
    let zone = Zone {
        x: x1.min(x2),
        y: y1.min(y2),
        w: (x1 - x2).abs(),
        h: (y1 - y2).abs(),
    };

    zone.is_sane().then_some(zone)
}

fn rect_of(video: egui::Rect, zone: Zone) -> egui::Rect {
    egui::Rect::from_min_size(
        video.min + egui::vec2(zone.x * video.width(), zone.y * video.height()),
        egui::vec2(zone.w * video.width(), zone.h * video.height()),
    )
}

/// Затемняет всё вокруг рамки и обводит её.
fn paint(ui: &egui::Ui, screen: egui::Rect, video: egui::Rect, zone: egui::Rect) {
    let painter = ui.painter();
    let dim = egui::Color32::from_black_alpha(DIM);

    let top = egui::Rect::from_min_max(screen.min, egui::pos2(screen.max.x, zone.min.y));
    let bottom = egui::Rect::from_min_max(egui::pos2(screen.min.x, zone.max.y), screen.max);
    let left = egui::Rect::from_min_max(
        egui::pos2(screen.min.x, zone.min.y),
        egui::pos2(zone.min.x, zone.max.y),
    );
    let right = egui::Rect::from_min_max(
        egui::pos2(zone.max.x, zone.min.y),
        egui::pos2(screen.max.x, zone.max.y),
    );

    for part in [top, bottom, left, right] {
        painter.rect_filled(part, 0.0, dim);
    }

    painter.rect_stroke(
        video,
        0.0,
        egui::Stroke::new(1.0, egui::Color32::from_white_alpha(60)),
        egui::StrokeKind::Inside,
    );
    painter.rect_stroke(
        zone,
        0.0,
        egui::Stroke::new(2.0, theme::PANEL_ACCENT),
        egui::StrokeKind::Outside,
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn рамка_переводится_в_доли_видео() {
        let video = egui::Rect::from_min_size(egui::pos2(100.0, 50.0), egui::vec2(800.0, 400.0));
        let zone =
            zone_from(video, egui::pos2(500.0, 250.0), egui::pos2(100.0, 450.0)).expect("зона");

        assert!((zone.x - 0.0).abs() < 1e-3 && (zone.w - 0.5).abs() < 1e-3);
        assert!((zone.y - 0.5).abs() < 1e-3 && (zone.h - 0.5).abs() < 1e-3);
    }

    #[test]
    fn слишком_малая_рамка_отвергается() {
        let video = egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(800.0, 400.0));

        assert!(zone_from(video, egui::pos2(10.0, 10.0), egui::pos2(12.0, 12.0)).is_none());
    }

    #[test]
    fn рамка_за_краем_видео_обрезается() {
        let video = egui::Rect::from_min_size(egui::pos2(100.0, 0.0), egui::vec2(800.0, 400.0));
        let zone =
            zone_from(video, egui::pos2(0.0, 100.0), egui::pos2(2000.0, 300.0)).expect("зона");

        assert!(zone.x == 0.0 && (zone.w - 1.0).abs() < 1e-3);
    }
}
