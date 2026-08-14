//! Окно обновления: что вышло и стоит ли это ставить.
//!
//! Отдельное окно, как у интеграций. Состояние и сеть — в `app/update.rs`.
//!
//! Заметка к выпуску показывается целиком: человек ставит обновление
//! не ради номера версии, а ради того, что в нём поправлено.

use crate::app::{PithApp, UpdateStage};
use crate::theme;
use crate::tr;
use crate::ui::dialog;

/// Размер окна при первом показе.
const DEFAULT_SIZE: [f32; 2] = [520.0, 560.0];

/// Наименьший размер: уже него заметка к выпуску нечитаема.
const MIN_SIZE: [f32; 2] = [400.0, 340.0];

/// Отступ содержимого от краёв.
const PADDING: i8 = 14;

/// Сколько высоты оставлено строке кнопок под заметкой.
const BUTTONS_HEIGHT: f32 = 92.0;

/// В мегабайте столько байт — делим на него размеры установщика.
const MB: f64 = 1024.0 * 1024.0;

/// Что нажали в окне.
enum Action {
    Check,
    Download,
    Install,
    ToggleAuto,
    Close,
}

pub fn show(app: &mut PithApp, ctx: &egui::Context) {
    if !app.update_open() {
        return;
    }

    let viewport = egui::ViewportBuilder::default()
        .with_title(tr!("Обновление", "Update"))
        .with_inner_size(DEFAULT_SIZE)
        .with_min_inner_size(MIN_SIZE);

    let id = egui::ViewportId::from_hash_of("update");
    let mut action = None;

    ctx.show_viewport_immediate(id, viewport, |ctx, _class| {
        // Подложка во всё окно: буфер общий с кадром mpv, и незакрашенные
        // места показывают видео.
        let window = ctx.input(|i| i.viewport_rect());

        egui::CentralPanel::default()
            .frame(
                egui::Frame::NONE
                    .fill(theme::PANEL_CARD)
                    .inner_margin(egui::Margin::same(PADDING)),
            )
            .show(ctx, |ui| {
                ui.painter().rect_filled(window, 0.0, theme::PANEL_CARD);
                action = show_body(app, ui);
            });

        if ctx.input(|i| i.viewport().close_requested()) {
            action = Some(Action::Close);
        }
    });

    match action {
        Some(Action::Check) => app.check_update(ctx),
        Some(Action::Download) => app.download_update(ctx),
        Some(Action::Install) => app.install_update(ctx),
        Some(Action::ToggleAuto) => app.toggle_update_check(),
        Some(Action::Close) => app.close_update(),
        None => {}
    }
}

fn show_body(app: &mut PithApp, ui: &mut egui::Ui) -> Option<Action> {
    ui.label(
        egui::RichText::new(tr!("Обновление", "Update"))
            .color(theme::TEXT_PRIMARY)
            .size(21.0)
            .strong(),
    );

    ui.add_space(4.0);
    dialog::hint(
        ui,
        &tr!(
            format!("Установлена версия {}", crate::VERSION),
            format!("Version {} is installed", crate::VERSION)
        ),
    );

    ui.add_space(12.0);

    let height = (ui.available_height() - BUTTONS_HEIGHT).max(0.0);
    let mut action = None;

    egui::ScrollArea::vertical()
        .max_height(height)
        .show(ui, |ui| {
            action = show_stage(app, ui);
        });

    ui.add_space(8.0);
    action.or_else(|| show_buttons(app, ui))
}

/// Что показать в середине окна — зависит от того, что сейчас происходит.
fn show_stage(app: &PithApp, ui: &mut egui::Ui) -> Option<Action> {
    match app.update_stage() {
        UpdateStage::Idle => {
            dialog::label(ui, tr!("Проверка не запускалась", "Not checked yet"));
        }
        UpdateStage::Checking => {
            ui.horizontal(|ui| {
                ui.spinner();
                ui.add_space(6.0);
                dialog::label(ui, tr!("Спрашиваю GitHub…", "Asking GitHub…"));
            });
        }
        UpdateStage::Latest => {
            dialog::card(ui, |ui| {
                ui.label(
                    egui::RichText::new(tr!(
                        "Установлена последняя версия",
                        "You have the latest version"
                    ))
                    .color(theme::TEXT_PRIMARY)
                    .size(15.0),
                );
            });
        }
        UpdateStage::Available(release) => show_release(ui, release),
        UpdateStage::Downloading { done, total } => show_downloading(ui, *done, *total),
        UpdateStage::Ready(path) => {
            dialog::card(ui, |ui| {
                ui.label(
                    egui::RichText::new(tr!(
                        "Установщик скачан и готов к запуску",
                        "The installer is downloaded and ready"
                    ))
                    .color(theme::TEXT_PRIMARY)
                    .size(15.0),
                );
                ui.add_space(4.0);
                dialog::hint(ui, &path.display().to_string());
                ui.add_space(4.0);
                dialog::hint(
                    ui,
                    tr!(
                        "Плеер закроется, а закладки и настройки останутся на месте.",
                        "The player will close; bookmarks and settings stay where they are."
                    ),
                );
            });
        }
        UpdateStage::Failed(why) => {
            dialog::card(ui, |ui| {
                ui.label(
                    egui::RichText::new(tr!("Не вышло", "It did not work")).color(theme::ERROR),
                );
                ui.add_space(4.0);
                dialog::hint(ui, why);
            });
        }
    }

    None
}

/// Вышедший выпуск: номер, размер и заметка к нему.
fn show_release(ui: &mut egui::Ui, release: &pith_update::Release) {
    dialog::card(ui, |ui| {
        ui.label(
            egui::RichText::new(tr!(
                format!("Вышла версия {}", release.version),
                format!("Version {} is out", release.version)
            ))
            .color(theme::PANEL_ACCENT)
            .size(17.0)
            .strong(),
        );

        ui.add_space(4.0);
        dialog::hint(
            ui,
            &tr!(
                format!("Установщик {:.1} МБ", release.installer.size as f64 / MB),
                format!("Installer {:.1} MB", release.installer.size as f64 / MB)
            ),
        );

        if !release.page.is_empty() {
            ui.add_space(4.0);
            ui.hyperlink_to(
                tr!("Открыть страницу выпуска", "Open the release page"),
                &release.page,
            );
        }
    });

    if release.notes.trim().is_empty() {
        return;
    }

    ui.add_space(10.0);
    dialog::section(ui, tr!("Что изменилось", "What changed"));
    ui.add_space(4.0);

    // Заметка приходит разметкой Markdown. Своего разборщика заводить
    // не станем — показываем как есть: строки, отступы и списки в ней
    // читаются и без выделения.
    ui.add(
        egui::Label::new(
            egui::RichText::new(release.notes.trim())
                .color(theme::TEXT_SECONDARY)
                .size(13.0),
        )
        .wrap(),
    );
}

/// Ход загрузки установщика.
fn show_downloading(ui: &mut egui::Ui, done: u64, total: u64) {
    let share = if total > 0 {
        done as f32 / total as f32
    } else {
        0.0
    };

    dialog::label(
        ui,
        &tr!(
            format!(
                "Качаю установщик: {:.1} из {:.1} МБ",
                done as f64 / MB,
                total as f64 / MB
            ),
            format!(
                "Downloading the installer: {:.1} of {:.1} MB",
                done as f64 / MB,
                total as f64 / MB
            )
        ),
    );

    ui.add_space(6.0);
    ui.add(egui::ProgressBar::new(share).show_percentage());
}

/// Кнопки внизу окна — свои для каждого положения дел.
fn show_buttons(app: &mut PithApp, ui: &mut egui::Ui) -> Option<Action> {
    let mut action = None;

    // Выключатель тихой проверки живёт здесь же: спрашивают о нём ровно
    // в тот миг, когда плеер о обновлении и заговорил.
    let mut auto = app.update_check_enabled();
    ui.horizontal(|ui| {
        if dialog::toggle(ui, &mut auto).clicked() {
            action = Some(Action::ToggleAuto);
        }
        ui.add_space(6.0);
        dialog::hint(ui, tr!("Проверять при запуске", "Check on startup"));
    });

    ui.add_space(8.0);

    ui.horizontal(|ui| {
        match app.update_stage() {
            UpdateStage::Available(_) => {
                if dialog::accent_button(ui, tr!("Скачать", "Download")).clicked() {
                    action = Some(Action::Download);
                }
            }
            UpdateStage::Ready(_) => {
                if dialog::accent_button(
                    ui,
                    tr!("Установить и закрыть плеер", "Install and close the player"),
                )
                .clicked()
                {
                    action = Some(Action::Install);
                }
            }
            UpdateStage::Downloading { .. } | UpdateStage::Checking => {}
            _ => {
                if dialog::accent_button(ui, tr!("Проверить", "Check")).clicked() {
                    action = Some(Action::Check);
                }
            }
        }

        if dialog::outline_button(ui, tr!("Закрыть", "Close")).clicked() {
            action = Some(Action::Close);
        }
    });

    action
}
