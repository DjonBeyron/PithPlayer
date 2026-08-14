//! Окно «Горячие клавиши»: список действий и переназначение.
//!
//! Всё в одном месте — раньше клавиши можно было узнать только из справки
//! `--help` или из кода. Строка на действие: название, нынешняя клавиша
//! и кнопка, которая ждёт нажатия новой.

use pith_store::{Binding, Command};

use crate::app::PithApp;
use crate::theme;
use crate::tr;
use crate::ui::dialog;

/// Размер окна при первом показе.
const DEFAULT_SIZE: [f32; 2] = [520.0, 620.0];

/// Наименьший размер: уже него подписи действий рвутся.
const MIN_SIZE: [f32; 2] = [420.0, 360.0];

/// Отступ содержимого от краёв.
const PADDING: i8 = 14;

/// Ширина плашки с клавишей.
const KEY_WIDTH: f32 = 150.0;

/// Сколько высоты оставлено строке кнопок.
const BUTTONS_HEIGHT: f32 = 84.0;

/// Что нажали в окне.
enum Action {
    Catch(Command),
    Assign(Command, Binding),
    Clear(Command),
    StopCatching,
    Reset,
    Close,
}

pub fn show(app: &mut PithApp, ctx: &egui::Context) {
    if !app.hotkeys_open() {
        return;
    }

    let viewport = egui::ViewportBuilder::default()
        .with_title(tr!("Горячие клавиши", "Keyboard shortcuts"))
        .with_inner_size(DEFAULT_SIZE)
        .with_min_inner_size(MIN_SIZE);

    let id = egui::ViewportId::from_hash_of("hotkeys");
    let mut action = None;

    ctx.show_viewport_immediate(id, viewport, |ctx, _class| {
        // Подложка во всё окно: буфер общий с кадром mpv, и незакрашенные
        // места показывают видео.
        let window = ctx.input(|i| i.viewport_rect());

        // Пока ждём клавишу — ловим её здесь, до всякой отрисовки.
        if let Some(command) = app.caught_command() {
            action = catch(ctx, command);
        }

        egui::CentralPanel::default()
            .frame(
                egui::Frame::NONE
                    .fill(theme::PANEL_CARD)
                    .inner_margin(egui::Margin::same(PADDING)),
            )
            .show(ctx, |ui| {
                ui.painter().rect_filled(window, 0.0, theme::PANEL_CARD);
                action = action.take().or_else(|| show_body(app, ui));
            });

        if ctx.input(|i| i.viewport().close_requested()) {
            action = Some(Action::Close);
        }
    });

    match action {
        Some(Action::Catch(command)) => app.catch_hotkey(command),
        Some(Action::Assign(command, binding)) => app.assign_hotkey(command, binding),
        Some(Action::Clear(command)) => app.clear_hotkey(command),
        Some(Action::StopCatching) => app.stop_catching_hotkey(),
        Some(Action::Reset) => app.reset_hotkeys(),
        Some(Action::Close) => app.close_hotkeys(),
        None => {}
    }
}

/// Ловит нажатую клавишу и делает из неё привязку.
///
/// Escape отменяет ловлю: это единственная клавиша, которую назначить
/// нельзя — ею закрывают окна, и отдать её другому действию значило бы
/// остаться без выхода.
fn catch(ctx: &egui::Context, command: Command) -> Option<Action> {
    ctx.input(|i| {
        for event in &i.events {
            let egui::Event::Key {
                key,
                physical_key,
                pressed: true,
                repeat: false,
                modifiers,
                ..
            } = event
            else {
                continue;
            };

            // Физическое положение клавиши, а не буква на ней: на русской
            // раскладке `]` даёт «ъ», и назначенное на неё не сработало бы.
            let key = physical_key.unwrap_or(*key);

            if key == egui::Key::Escape {
                return Some(Action::StopCatching);
            }

            return Some(Action::Assign(
                command,
                Binding {
                    key: key.name().to_string(),
                    ctrl: modifiers.ctrl,
                    shift: modifiers.shift,
                    alt: modifiers.alt,
                },
            ));
        }

        None
    })
}

fn show_body(app: &PithApp, ui: &mut egui::Ui) -> Option<Action> {
    ui.label(
        egui::RichText::new(tr!("Горячие клавиши", "Keyboard shortcuts"))
            .color(theme::TEXT_PRIMARY)
            .size(21.0)
            .strong(),
    );

    ui.add_space(4.0);
    dialog::hint(
        ui,
        tr!(
            "Нажмите клавишу в строке — и следующее нажатие станет новой. Escape отменяет.",
            "Click the key in a row — the next press becomes the new one. Escape cancels."
        ),
    );

    ui.add_space(12.0);

    let height = (ui.available_height() - BUTTONS_HEIGHT).max(0.0);
    let mut action = None;

    egui::ScrollArea::vertical()
        .max_height(height)
        .show(ui, |ui| {
            for command in Command::ALL {
                if let Some(chosen) = show_row(app, ui, command) {
                    action = Some(chosen);
                }
            }
        });

    ui.add_space(8.0);
    action.or_else(|| show_footer(app, ui))
}

/// Строка списка: действие, его клавиша и кнопка очистки.
fn show_row(app: &PithApp, ui: &mut egui::Ui, command: Command) -> Option<Action> {
    let binding = app.hotkeys().binding(command);
    let catching = app.caught_command() == Some(command);
    let mut action = None;

    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new(title(command))
                .color(theme::TEXT_PRIMARY)
                .size(14.0),
        );

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            // Крестик снимает клавишу: действие может остаться без неё,
            // если она нужнее другому.
            if binding.is_set()
                && !catching
                && ui
                    .add(egui::Button::new("×").frame(false))
                    .on_hover_text(tr!("Снять клавишу", "Clear the key"))
                    .clicked()
            {
                action = Some(Action::Clear(command));
            }

            let label = if catching {
                tr!("нажмите клавишу…", "press a key…").to_string()
            } else if binding.is_set() {
                binding.label()
            } else {
                tr!("не назначена", "not set").to_string()
            };

            let color = if catching {
                theme::PANEL_ACCENT
            } else if binding.is_set() {
                theme::TEXT_PRIMARY
            } else {
                theme::PANEL_MUTED
            };

            let button = egui::Button::new(egui::RichText::new(label).color(color).size(13.0))
                .min_size(egui::vec2(KEY_WIDTH, 28.0));

            if ui.add(button).clicked() {
                action = Some(if catching {
                    Action::StopCatching
                } else {
                    Action::Catch(command)
                });
            }
        });
    });

    ui.add_space(2.0);
    action
}

/// Строка внизу: что произошло с прошлым назначением и общие кнопки.
fn show_footer(app: &PithApp, ui: &mut egui::Ui) -> Option<Action> {
    let mut action = None;

    if let Some(taken) = app.hotkey_taken_from() {
        dialog::hint(
            ui,
            &tr!(
                format!("Клавиша снята с действия «{}»", title(taken)),
                format!("The key was taken from “{}”", title(taken))
            ),
        );
        ui.add_space(6.0);
    }

    ui.horizontal(|ui| {
        if app.hotkeys_changed()
            && dialog::outline_button(ui, tr!("Вернуть умолчания", "Reset to defaults")).clicked()
        {
            action = Some(Action::Reset);
        }

        if dialog::accent_button(ui, tr!("Закрыть", "Close")).clicked() {
            action = Some(Action::Close);
        }
    });

    action
}

/// Название действия для списка.
fn title(command: Command) -> &'static str {
    match command {
        Command::TogglePause => tr!("Пауза / продолжить", "Pause / resume"),
        Command::SeekForward => tr!("Перемотка вперёд", "Seek forward"),
        Command::SeekBack => tr!("Перемотка назад", "Seek back"),
        Command::VolumeUp => tr!("Громче", "Volume up"),
        Command::VolumeDown => tr!("Тише", "Volume down"),
        Command::Fullscreen => tr!("Полный экран", "Fullscreen"),
        Command::SpeedUp => tr!("Скорость больше", "Speed up"),
        Command::SpeedDown => tr!("Скорость меньше", "Slow down"),
        Command::SpeedReset => tr!("Обычная скорость", "Normal speed"),
        Command::AddBookmark => tr!("Поставить закладку", "Add a bookmark"),
        Command::RemoveBookmark => tr!("Убрать закладку", "Remove a bookmark"),
        Command::CopySubtitle => tr!("Скопировать реплику", "Copy the subtitle line"),
        Command::ToggleSubtitles => tr!("Показать субтитры", "Show subtitles"),
        Command::OpenSearch => tr!("Поиск по субтитрам", "Search subtitles"),
        Command::ToggleActors => tr!("Окно актёров", "Actors window"),
    }
}
