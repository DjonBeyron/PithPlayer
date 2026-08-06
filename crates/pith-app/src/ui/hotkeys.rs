//! Горячие клавиши. Схема сохраняется из v4 (PLAN.md §6.8).
//!
//! Разбираются внутри кадра egui: в `logic` кадр ещё не начат
//! и `input()` не отдаёт нажатия.

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

    let actions = ctx.input(collect_actions);

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
}

fn collect_actions(i: &egui::InputState) -> Actions {
    use egui::Key;

    let mut actions = Actions {
        // Привычное сочетание для поиска.
        open_search: i.modifiers.ctrl && i.key_pressed(Key::F),
        ..Default::default()
    };

    // Шаг перемотки зависит от модификатора: как в v4.
    let step = if i.modifiers.ctrl {
        SEEK_STEP_LARGE
    } else if i.modifiers.shift {
        SEEK_STEP_SMALL
    } else {
        SEEK_STEP
    };

    // Ctrl+F занят поиском, поэтому полный экран по F — только без модификатора.
    if actions.open_search {
        return actions;
    }

    for key in pressed_keys(i) {
        match key {
            Key::Space => actions.toggle_pause = true,
            Key::Escape => actions.escape = true,
            Key::F => actions.toggle_fullscreen = true,
            Key::Backspace => actions.reset_speed = true,
            Key::ArrowRight => actions.seek += step,
            Key::ArrowLeft => actions.seek -= step,
            Key::ArrowUp => actions.volume += VOLUME_STEP,
            Key::ArrowDown => actions.volume -= VOLUME_STEP,
            Key::CloseBracket => actions.speed += SPEED_STEP,
            Key::OpenBracket => actions.speed -= SPEED_STEP,
            // Схема из v4: C копирует реплику субтитров, T ставит закладку.
            Key::C => actions.copy_subtitle = true,
            Key::V => actions.toggle_subtitles = true,
            Key::T if i.modifiers.shift => actions.remove_bookmark = true,
            Key::T => actions.add_bookmark = true,
            _ => {}
        }
    }

    actions
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
