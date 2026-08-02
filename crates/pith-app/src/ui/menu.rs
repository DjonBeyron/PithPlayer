//! Контекстное меню по правому щелчку.
//!
//! Порт `ModernContextMenu` из v4: выбор дорожек, скорость, полный экран.

use pith_mpv::TrackKind;

use crate::app::PithApp;
use crate::theme;

/// Готовые значения скорости.
const SPEED_PRESETS: [f64; 6] = [0.5, 0.75, 1.0, 1.25, 1.5, 2.0];

/// Показывает меню, если по видео щёлкнули правой кнопкой.
pub fn show(app: &mut PithApp, ctx: &egui::Context) {
    let id = egui::Id::new("context_menu");

    if ctx.input(|i| i.pointer.secondary_clicked())
        && let Some(pos) = ctx.input(|i| i.pointer.interact_pos())
    {
        ctx.memory_mut(|m| m.data.insert_temp(id, pos));
    }

    let Some(position) = ctx.memory(|m| m.data.get_temp::<egui::Pos2>(id)) else {
        return;
    };

    let mut close = false;

    let area = egui::Area::new(id.with("area"))
        .order(egui::Order::Foreground)
        .fixed_pos(position)
        .show(ctx, |ui| {
            egui::Frame::popup(ui.style())
                .fill(theme::WINDOW_BG)
                .show(ui, |ui| {
                    ui.set_min_width(230.0);
                    close = show_items(app, ui);
                });
        });

    // Щелчок мимо меню закрывает его.
    let clicked_outside = ctx.input(|i| i.pointer.any_click())
        && !area.response.rect.contains(
            ctx.input(|i| i.pointer.interact_pos())
                .unwrap_or(egui::Pos2::ZERO),
        );

    if close || clicked_outside || ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
        ctx.memory_mut(|m| m.data.remove::<egui::Pos2>(id));
    }
}

/// Пункты меню. Возвращает `true`, если меню пора закрыть.
fn show_items(app: &mut PithApp, ui: &mut egui::Ui) -> bool {
    let mut close = false;

    if ui.button("Открыть файл…").clicked() {
        app.open_file_dialog();
        close = true;
    }

    ui.separator();

    close |= show_audio_tracks(app, ui);
    close |= show_subtitle_tracks(app, ui);
    close |= show_speed(app, ui);

    ui.separator();

    let subtitles_visible = app.settings().subtitles_visible;
    let label = if subtitles_visible {
        "Скрыть субтитры"
    } else {
        "Показать субтитры"
    };

    if ui.button(label).clicked() {
        app.toggle_subtitles();
        close = true;
    }

    if ui.button("Полный экран").clicked() {
        app.toggle_fullscreen(ui.ctx());
        close = true;
    }

    close
}

fn show_audio_tracks(app: &mut PithApp, ui: &mut egui::Ui) -> bool {
    let tracks: Vec<_> = app
        .tracks()
        .iter()
        .filter(|t| t.kind == TrackKind::Audio)
        .map(|t| (t.id, t.label()))
        .collect();

    if tracks.is_empty() {
        return false;
    }

    let current = app.selected_tracks().audio;
    let mut chosen = None;

    ui.menu_button("Аудиодорожка", |ui| {
        for (id, label) in &tracks {
            if ui.radio(current == Some(*id), label).clicked() {
                chosen = Some(Some(*id));
                ui.close();
            }
        }
    });

    if let Some(id) = chosen {
        app.choose_audio_track(id);
        return true;
    }

    false
}

fn show_subtitle_tracks(app: &mut PithApp, ui: &mut egui::Ui) -> bool {
    let tracks: Vec<_> = app
        .tracks()
        .iter()
        .filter(|t| t.kind == TrackKind::Subtitle)
        .map(|t| (t.id, t.label()))
        .collect();

    if tracks.is_empty() {
        return false;
    }

    let selected = app.selected_tracks();
    let mut main_choice = None;
    let mut secondary_choice = None;

    ui.menu_button("Субтитры", |ui| {
        if ui.radio(selected.subtitle.is_none(), "Выключены").clicked() {
            main_choice = Some(None);
            ui.close();
        }

        for (id, label) in &tracks {
            if ui.radio(selected.subtitle == Some(*id), label).clicked() {
                main_choice = Some(Some(*id));
                ui.close();
            }
        }
    });

    // Вторые субтитры mpv показывает одновременно с основными.
    ui.menu_button("Вторые субтитры", |ui| {
        if ui
            .radio(selected.secondary_subtitle.is_none(), "Выключены")
            .clicked()
        {
            secondary_choice = Some(None);
            ui.close();
        }

        for (id, label) in &tracks {
            if ui
                .radio(selected.secondary_subtitle == Some(*id), label)
                .clicked()
            {
                secondary_choice = Some(Some(*id));
                ui.close();
            }
        }
    });

    if let Some(id) = main_choice {
        app.choose_subtitle_track(id);
        return true;
    }
    if let Some(id) = secondary_choice {
        app.choose_secondary_subtitle_track(id);
        return true;
    }

    false
}

fn show_speed(app: &mut PithApp, ui: &mut egui::Ui) -> bool {
    let current = app.engine().map(|e| e.state().speed).unwrap_or(1.0);
    let mut chosen = None;

    ui.menu_button(format!("Скорость: {current:.2}×"), |ui| {
        for speed in SPEED_PRESETS {
            let active = (current - speed).abs() < 0.01;
            if ui.radio(active, format!("{speed:.2}×")).clicked() {
                chosen = Some(speed);
                ui.close();
            }
        }
    });

    if let Some(speed) = chosen {
        app.set_speed(speed);
        return true;
    }

    false
}
