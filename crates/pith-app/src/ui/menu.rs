//! Контекстное меню по правому щелчку.
//!
//! Порт `ModernContextMenu` из v4: выбор дорожек, скорость, полный экран.

use pith_mpv::TrackKind;

use crate::app::PithApp;

/// Готовые значения скорости.
const SPEED_PRESETS: [f64; 6] = [0.5, 0.75, 1.0, 1.25, 1.5, 2.0];

/// Минимальная ширина меню.
const MENU_WIDTH: f32 = 220.0;

/// Пункты меню.
///
/// Вызывается из `Response::context_menu`, а не рисуется своей областью:
/// только внутри меню-контекста `ui.menu_button` превращается
/// в раскрывающийся по наведению `SubMenuButton`, который egui сам
/// размещает сбоку и переворачивает у края экрана.
pub fn show_items(app: &mut PithApp, ui: &mut egui::Ui) {
    ui.set_min_width(MENU_WIDTH);

    if ui.button("Открыть файл…").clicked() {
        app.open_file_dialog();
        ui.close();
    }

    if ui.button("Поиск по субтитрам…").clicked() {
        app.open_search();
        ui.close();
    }

    ui.separator();

    show_audio_tracks(app, ui);
    show_subtitle_tracks(app, ui);
    show_speed(app, ui);

    ui.separator();

    let subtitles_visible = app.settings().subtitles_visible;
    let label = if subtitles_visible {
        "Скрыть субтитры"
    } else {
        "Показать субтитры"
    };

    if ui.button(label).clicked() {
        app.toggle_subtitles();
        ui.close();
    }

    if ui.button("Полный экран").clicked() {
        app.toggle_fullscreen(ui.ctx());
        ui.close();
    }
}

fn show_audio_tracks(app: &mut PithApp, ui: &mut egui::Ui) {
    let tracks: Vec<_> = app
        .tracks()
        .iter()
        .filter(|t| t.kind == TrackKind::Audio)
        .map(|t| (t.id, t.label()))
        .collect();

    if tracks.is_empty() {
        return;
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
    }
}

fn show_subtitle_tracks(app: &mut PithApp, ui: &mut egui::Ui) {
    let tracks: Vec<_> = app
        .tracks()
        .iter()
        .filter(|t| t.kind == TrackKind::Subtitle)
        .map(|t| (t.id, t.label()))
        .collect();

    if tracks.is_empty() {
        return;
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
    }
    if let Some(id) = secondary_choice {
        app.choose_secondary_subtitle_track(id);
    }
}

fn show_speed(app: &mut PithApp, ui: &mut egui::Ui) {
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
    }
}
