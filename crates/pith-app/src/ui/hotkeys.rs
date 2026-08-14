//! Горячие клавиши. Схема сохраняется из v4 (PLAN.md §6.8).
//!
//! Разбираются внутри кадра egui: в `logic` кадр ещё не начат
//! и `input()` не отдаёт нажатия.
//!
//! Какая клавиша что делает — больше не зашито здесь: привязки живут
//! в настройках (`pith_store::Hotkeys`) и правятся в окне
//! «Горячие клавиши…». Здесь остаётся то, что от клавиши не зависит:
//! величина шага, повторы зажатой клавиши и правила про Escape.

use pith_store::{Binding, Command};

use crate::app::PithApp;

/// Шаги перемотки, секунды.
const SEEK_STEP: f64 = 5.0;
const SEEK_STEP_SMALL: f64 = 1.0;
const SEEK_STEP_LARGE: f64 = 60.0;

/// Шаг громкости.
const VOLUME_STEP: i64 = 5;

/// Шаг изменения скорости.
const SPEED_STEP: f64 = 0.1;

/// Что нажал пользователь за кадр.
#[derive(Default)]
struct Actions {
    seek: f64,
    volume: i64,
    speed: f64,
    toggle_pause: bool,
    toggle_fullscreen: bool,
    /// Escape: полный экран, но только если его не ждёт открытое окно.
    escape: bool,
    reset_speed: bool,
    copy_subtitle: bool,
    toggle_subtitles: bool,
    open_search: bool,
    add_bookmark: bool,
    /// Показать или спрятать окно актёров.
    toggle_actors: bool,
    remove_bookmark: bool,
}

pub fn handle_hotkeys(app: &mut PithApp, ctx: &egui::Context) {
    // Пока фокус в текстовом поле, клавиши принадлежат ему.
    //
    // Проверяется именно текстовое поле, а не фокус вообще: фокус остаётся
    // и на нажатой кнопке панели, и с проверкой «занят ли фокус чем угодно»
    // после щелчка по любой кнопке замолкали все горячие клавиши.
    if ctx.text_edit_focused() {
        return;
    }

    // Пока открыто окно поверх кадра, клавиши принадлежат ему. Иначе
    // Escape закрывал окно и заодно переключал полный экран, а пробел
    // ставил паузу вместо ответа диалогу.
    if app.dialog_open() {
        return;
    }

    // Пока в окне настроек ждут нажатия клавиши, все клавиши — его.
    // Иначе назначение пробела заодно ставило бы плеер на паузу.
    if app.catching_hotkey() {
        return;
    }

    let hotkeys = app.hotkeys().clone();
    let actions = ctx.input(|i| collect_actions(i, &hotkeys));

    if actions.toggle_pause {
        app.toggle_pause();
    }
    // Escape отдаётся открытому окну: поиск, диалоги списков и настроек
    // закрываются им же, и одно нажатие не должно заодно разворачивать
    // плеер на весь экран.
    if actions.toggle_fullscreen || (actions.escape && !app.escape_belongs_to_window()) {
        app.toggle_fullscreen(ctx);
    }
    if actions.seek != 0.0 {
        app.seek_relative(actions.seek);
    }
    if actions.volume != 0 {
        app.adjust_volume(actions.volume);
    }
    if actions.speed != 0.0 {
        app.adjust_speed(actions.speed);
    }
    if actions.reset_speed {
        app.reset_speed();
    }
    if actions.copy_subtitle {
        app.copy_current_subtitle();
    }
    if actions.toggle_subtitles {
        app.toggle_subtitles();
    }
    if actions.open_search {
        app.open_search();
    }
    if actions.add_bookmark {
        app.add_bookmark_here();
    }
    if actions.remove_bookmark {
        app.remove_bookmark_here();
    }
    if actions.toggle_actors {
        app.toggle_actors_window();
    }
}

fn collect_actions(i: &egui::InputState, hotkeys: &pith_store::Hotkeys) -> Actions {
    let mut actions = Actions::default();

    // Шаг перемотки зависит от модификатора: Shift — крупный, Alt —
    // мелкий. Ctrl оставлен крупным заодно с Shift: схема из v4, и руки
    // у неё уже наработаны.
    let step = if i.modifiers.shift || i.modifiers.ctrl {
        SEEK_STEP_LARGE
    } else if i.modifiers.alt {
        SEEK_STEP_SMALL
    } else {
        SEEK_STEP
    };

    for key in pressed_keys(i) {
        let Some(command) = command_for(hotkeys, key, &i.modifiers) else {
            // Escape привязкой не назначается: им закрывают окна, и отдать
            // его другому действию значило бы остаться без выхода.
            if key == egui::Key::Escape {
                actions.escape = true;
            }
            continue;
        };

        apply(&mut actions, command, step);
    }

    // Зажатая стрелка мотает, пока её держат. Повторы считаются только
    // для перемотки: зажатая T насыпала бы закладок, а зажатая C — стопку
    // одинаковых копий в буфер.
    actions.seek += held_seek(i, hotkeys, step);

    actions
}

/// Отмечает действие, вызванное клавишей.
fn apply(actions: &mut Actions, command: Command, step: f64) {
    match command {
        Command::TogglePause => actions.toggle_pause = true,
        Command::Fullscreen => actions.toggle_fullscreen = true,
        Command::SeekForward => actions.seek += step,
        Command::SeekBack => actions.seek -= step,
        Command::VolumeUp => actions.volume += VOLUME_STEP,
        Command::VolumeDown => actions.volume -= VOLUME_STEP,
        Command::SpeedUp => actions.speed += SPEED_STEP,
        Command::SpeedDown => actions.speed -= SPEED_STEP,
        Command::SpeedReset => actions.reset_speed = true,
        Command::AddBookmark => actions.add_bookmark = true,
        Command::RemoveBookmark => actions.remove_bookmark = true,
        Command::CopySubtitle => actions.copy_subtitle = true,
        Command::ToggleSubtitles => actions.toggle_subtitles = true,
        Command::OpenSearch => actions.open_search = true,
        Command::ToggleActors => actions.toggle_actors = true,
    }
}

/// Какому действию отдана эта клавиша с этими модификаторами.
///
/// Сначала точное совпадение: `Ctrl+F` — поиск, а `F` без него —
/// полный экран, и спутать их нельзя. Если точного нет, пробуем клавишу
/// без модификаторов — но только для перемотки и громкости, где
/// модификатор задаёт величину шага, а не другое действие.
pub(super) fn command_for(
    hotkeys: &pith_store::Hotkeys,
    key: egui::Key,
    modifiers: &egui::Modifiers,
) -> Option<Command> {
    let name = key.name();

    let exact = Binding {
        key: name.to_string(),
        ctrl: modifiers.ctrl,
        shift: modifiers.shift,
        alt: modifiers.alt,
    };

    if let Some(command) = hotkeys.holder(&exact) {
        return Some(command);
    }

    hotkeys
        .holder(&Binding::key(name))
        .filter(|command| command.modifier_changes_step())
}

/// Сколько намотали повторы зажатой клавиши перемотки за этот кадр.
fn held_seek(i: &egui::InputState, hotkeys: &pith_store::Hotkeys, step: f64) -> f64 {
    i.events
        .iter()
        .filter_map(|event| match event {
            egui::Event::Key {
                key,
                physical_key,
                pressed: true,
                repeat: true,
                modifiers,
                ..
            } => {
                let key = physical_key.unwrap_or(*key);

                match command_for(hotkeys, key, modifiers) {
                    Some(Command::SeekForward) => Some(step),
                    Some(Command::SeekBack) => Some(-step),
                    _ => None,
                }
            }
            _ => None,
        })
        .sum()
}

/// Клавиши, нажатые в этом кадре, независимо от раскладки.
///
/// Берётся физическое положение клавиши: на русской раскладке `]` даёт «ъ»,
/// и привязка к логическому символу просто не срабатывала бы.
fn pressed_keys(i: &egui::InputState) -> Vec<egui::Key> {
    i.events
        .iter()
        .filter_map(|event| match event {
            egui::Event::Key {
                key,
                physical_key,
                pressed: true,
                repeat: false,
                ..
            } => {
                // Уровень trace: помогает разбирать жалобы вида
                // «клавиша не работает» — сразу видно, дошла ли она вообще.
                tracing::trace!(?key, ?physical_key, "нажата клавиша");
                Some(physical_key.unwrap_or(*key))
            }
            _ => None,
        })
        .collect()
}
