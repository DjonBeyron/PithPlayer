//! Интерфейс плеера.
//!
//! Всё рисуется поверх видео отдельными слоями: mpv кладёт кадр в весь
//! буфер окна и закрашивает то, что нарисовано до него (PLAN.md §3).

mod actor_photo;
mod actors;
mod bookmark_rename;
pub(crate) mod bookmarks;
mod bookmarks_actions;
mod bookmarks_row;
mod controls;
mod dialog;
mod export;
mod export_form;
mod export_report;
mod extraction;
mod file_types;
mod fragment_settings;
mod history;
mod hotkeys;
pub mod icons;
mod integrations;
mod lists;
mod menu;
mod menu_tracks;
mod metrics;
mod migration;
mod notice;
mod panel_handle;
mod panel_head;
mod panel_output_dir;
mod panel_resize;
mod playback_badge;
mod preview;
mod resume;
mod search;
mod seek_hud;
mod speed;
mod subtitle_style;
pub mod subtitles;
mod time_label;
mod timeline;
mod volume;

pub use actors::show as show_actors_window;
pub use bookmark_rename::show as show_bookmark_rename;
pub use bookmarks::show as show_bookmarks_panel;
pub use controls::show_controls;
pub use export::show as show_export;
pub use extraction::show as show_extraction_notice;
pub use file_types::show as show_file_types_prompt;
pub use fragment_settings::show as show_fragment_settings;
pub use history::show as show_history;
pub use hotkeys::handle_hotkeys;
pub use integrations::show as show_integrations;
pub use lists::show_clear_prompt as show_clear_list_prompt;
pub use lists::show_dialog as show_list_dialog;
pub use menu::show_items as show_menu_items;
pub use migration::show as show_migration_report;
pub use notice::show as show_notice;
pub use panel_handle::show as show_panel_handle;
pub use playback_badge::show as show_playback_badge;
pub use resume::show as show_resume_offer;
pub use search::show as show_search;
pub use seek_hud::show as show_seek_hud;
pub use subtitle_style::show as show_subtitle_style;
pub use subtitles::show as show_subtitles;
pub use timeline::FragmentRange;

use crate::theme;

/// Сообщение о невозможности запуска движка.
pub fn show_fatal_error(ui: &mut egui::Ui, message: &str) {
    egui::CentralPanel::default()
        .frame(egui::Frame::NONE.fill(theme::WINDOW_BG))
        .show(ui, |ui| {
            ui.vertical_centered(|ui| {
                ui.add_space(80.0);
                ui.heading(
                    egui::RichText::new(crate::tr!(
                        "Плеер не смог запуститься",
                        "The player could not start"
                    ))
                    .color(theme::ERROR),
                );
                ui.add_space(16.0);
                ui.label(egui::RichText::new(message).color(theme::TEXT_PRIMARY));
                ui.add_space(24.0);
                ui.label(
                    egui::RichText::new(crate::tr!(
                        "Проверьте, что рядом с программой лежит libmpv-2.dll",
                        "Check that libmpv-2.dll sits next to the program"
                    ))
                    .color(theme::TEXT_SECONDARY),
                );
            });
        });
}

/// Нажата ли кнопка, которой не нужен фокус клавиатуры.
///
/// egui оставляет фокус на нажатой кнопке, а это ломает две вещи сразу:
/// пробел после щелчка «нажимает» её повторно вместо паузы, и поле поиска
/// не может вернуть себе ввод, пока фокус занят кнопкой. Кнопкам плеера
/// клавиатура не нужна — фокус отпускается тут же.
pub fn clicked(response: &egui::Response) -> bool {
    if !response.clicked() {
        return false;
    }

    response.surrender_focus();
    true
}

/// Время в формате «Ч:ММ:СС» либо «М:СС» для коротких файлов.
pub fn format_time(seconds: f64) -> String {
    format_time_padded(seconds, seconds)
}

/// Время в том же виде, что и `reference`.
///
/// Надпись со временем стоит слева от полосы перемотки, и полоса
/// занимает всё, что от надписи осталось. Стоит надписи вырасти на разряд
/// — на 9:59 → 10:00 или когда минуты переходят в часы, — как полоса на
/// столько же укорачивается. Поэтому разряды дополняются нулями до того
/// вида, который нужен длительности: длина надписи не меняется весь фильм.
pub fn format_time_padded(seconds: f64, reference: f64) -> String {
    let total = whole_seconds(seconds);
    // Позиция за длительность не уходит, но если файл ещё не измерен,
    // отталкиваемся от неё самой — иначе разрядов не хватит.
    let reference = whole_seconds(reference).max(total);

    let hours = total / 3600;
    let minutes = (total % 3600) / 60;
    let secs = total % 60;

    if reference >= 3600 {
        let width = digits(reference / 3600);
        format!("{hours:0width$}:{minutes:02}:{secs:02}")
    } else {
        let width = digits(reference / 60);
        format!("{minutes:0width$}:{secs:02}")
    }
}

/// Целые секунды. Отрицательное и нечисловое считается нулём.
fn whole_seconds(seconds: f64) -> u64 {
    if seconds.is_finite() && seconds > 0.0 {
        seconds as u64
    } else {
        0
    }
}

/// Сколько разрядов занимает число. У нуля — один.
fn digits(value: u64) -> usize {
    let mut count = 1;
    let mut rest = value / 10;

    while rest > 0 {
        count += 1;
        rest /= 10;
    }

    count
}

#[cfg(test)]
mod tests {
    use super::{format_time, format_time_padded};

    #[test]
    fn форматирует_короткое_время() {
        assert_eq!(format_time(0.0), "0:00");
        assert_eq!(format_time(65.0), "1:05");
        assert_eq!(format_time(599.0), "9:59");
    }

    #[test]
    fn форматирует_часы() {
        assert_eq!(format_time(3600.0), "1:00:00");
        assert_eq!(format_time(7325.0), "2:02:05");
    }

    #[test]
    fn защищается_от_некорректных_значений() {
        assert_eq!(format_time(-5.0), "0:00");
        assert_eq!(format_time(f64::NAN), "0:00");
        assert_eq!(format_time(f64::INFINITY), "0:00");
    }

    #[test]
    fn позиция_принимает_вид_длительности() {
        // Двухчасовой фильм: у позиции есть часы, даже когда их ноль.
        assert_eq!(format_time_padded(65.0, 7200.0), "0:01:05");
        // Двадцатипятиминутный: минуты в два разряда.
        assert_eq!(format_time_padded(305.0, 1500.0), "05:05");
        // Короткий: лишних разрядов не появляется.
        assert_eq!(format_time_padded(5.0, 300.0), "0:05");
    }

    #[test]
    fn длина_надписи_не_меняется_за_весь_фильм() {
        for duration in [59.0, 300.0, 3599.0, 7325.0, 40000.0] {
            let expected = format_time_padded(0.0, duration).chars().count();

            let mut position = 0.0;
            while position <= duration {
                let text = format_time_padded(position, duration);
                assert_eq!(
                    text.chars().count(),
                    expected,
                    "длительность {duration}, позиция {position}: «{text}»"
                );
                position += 1.0;
            }
        }
    }

    #[test]
    fn длительность_остаётся_в_прежнем_виде() {
        // Сама длительность форматируется как раньше: дополнять её не от чего.
        assert_eq!(format_time_padded(3600.0, 3600.0), "1:00:00");
        assert_eq!(format_time_padded(599.0, 599.0), "9:59");
    }
}
