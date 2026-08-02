//! Интерфейс плеера.
//!
//! Всё рисуется поверх видео отдельными слоями: mpv кладёт кадр в весь
//! буфер окна и закрашивает то, что нарисовано до него (PLAN.md §3).

mod bookmarks;
mod controls;
mod fragment_settings;
mod hotkeys;
mod lists;
mod menu;
mod metrics;
mod migration;
mod notice;
mod resume;
mod search;
pub mod subtitles;
mod timeline;

pub use bookmarks::show as show_bookmarks_panel;
pub use controls::show_controls;
pub use fragment_settings::show as show_fragment_settings;
pub use hotkeys::handle_hotkeys;
pub use lists::show_dialog as show_list_dialog;
pub use menu::show_items as show_menu_items;
pub use migration::show as show_migration_report;
pub use notice::show as show_notice;
pub use resume::show as show_resume_offer;
pub use search::show as show_search;
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
                ui.heading(egui::RichText::new("Плеер не смог запуститься").color(theme::ERROR));
                ui.add_space(16.0);
                ui.label(egui::RichText::new(message).color(theme::TEXT_PRIMARY));
                ui.add_space(24.0);
                ui.label(
                    egui::RichText::new("Проверьте, что рядом с программой лежит libmpv-2.dll")
                        .color(theme::TEXT_SECONDARY),
                );
            });
        });
}

/// Время в формате «Ч:ММ:СС» либо «М:СС» для коротких файлов.
pub fn format_time(seconds: f64) -> String {
    if !seconds.is_finite() || seconds < 0.0 {
        return "0:00".into();
    }

    let total = seconds as u64;
    let hours = total / 3600;
    let minutes = (total % 3600) / 60;
    let secs = total % 60;

    if hours > 0 {
        format!("{hours}:{minutes:02}:{secs:02}")
    } else {
        format!("{minutes}:{secs:02}")
    }
}

#[cfg(test)]
mod tests {
    use super::format_time;

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
}
