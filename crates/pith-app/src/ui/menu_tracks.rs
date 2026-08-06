//! Подменю дорожек: звук, субтитры и устройство вывода.
//!
//! Отдельно от самого меню: там пункты плеера, а здесь — три списка,
//! которые собираются по одному правилу и занимают больше всего места.

use pith_mpv::TrackKind;

use crate::app::PithApp;
use crate::tr;

pub fn show_audio_tracks(app: &mut PithApp, ui: &mut egui::Ui) {
    let tracks: Vec<_> = app
        .tracks()
        .iter()
        .filter(|t| t.kind == TrackKind::Audio)
        .map(|t| (t.id, track_label(t)))
        .collect();

    if tracks.is_empty() {
        return;
    }

    let current = app.selected_tracks().audio;
    let mut chosen = None;

    ui.menu_button(tr!("Аудиодорожка", "Audio track"), |ui| {
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

pub fn show_subtitle_tracks(app: &mut PithApp, ui: &mut egui::Ui) {
    let tracks: Vec<_> = app
        .tracks()
        .iter()
        .filter(|t| t.kind == TrackKind::Subtitle)
        .map(|t| (t.id, track_label(t)))
        .collect();

    if tracks.is_empty() {
        return;
    }

    let selected = app.selected_tracks();
    let mut main_choice = None;
    let mut secondary_choice = None;
    let off = tr!("Выключены", "Off");

    ui.menu_button(tr!("Субтитры", "Subtitles"), |ui| {
        if ui.radio(selected.subtitle.is_none(), off).clicked() {
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

    // Дополнительные субтитры mpv показывает одновременно с основными.
    ui.menu_button(tr!("Доп. субтитры", "Second subtitles"), |ui| {
        if ui
            .radio(selected.secondary_subtitle.is_none(), off)
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

/// Куда выводить звук. Переключается без перезапуска плеера.
pub fn show_audio_devices(app: &mut PithApp, ui: &mut egui::Ui) {
    let devices = app.audio_devices();
    if devices.is_empty() {
        return;
    }

    let current = app.current_audio_device();
    let mut chosen = None;

    ui.menu_button(tr!("Вывод звука", "Audio output"), |ui| {
        for device in &devices {
            // Автоматический выбор mpv отдаёт первым — оставляем его наверху
            // и подписываем понятнее, чем «Autoselect device».
            let label = if device.is_auto() {
                tr!("Как в системе", "System default").to_string()
            } else {
                device.label()
            };

            if ui.radio(device.name == current, label).clicked() {
                chosen = Some(device.name.clone());
                ui.close();
            }
        }
    });

    if let Some(name) = chosen {
        app.choose_audio_device(&name);
    }
}

/// Подпись дорожки в меню.
///
/// Название и язык берутся из файла как есть, а пометка о форсированной
/// дорожке — наше слово, и переводится вместе с остальным интерфейсом.
fn track_label(track: &pith_mpv::Track) -> String {
    let label = track.label();

    if track.forced {
        tr!(format!("{label} форсир."), format!("{label} forced"))
    } else {
        label
    }
}
