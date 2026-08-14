//! Окно горячих клавиш: что каким вызывается и как это переназначить.
//!
//! Сама схема живёт в настройках (`pith_store::Hotkeys`), разбор нажатий —
//! в `ui/hotkeys.rs`, а окно — в `ui/hotkeys_window.rs`. Здесь только
//! состояние окна и правка схемы.

use pith_store::{Binding, Command, Hotkeys};

use super::PithApp;

/// Состояние окна горячих клавиш.
#[derive(Default)]
pub struct HotkeysState {
    pub open: bool,
    /// Действие, которому сейчас ловим клавишу.
    ///
    /// Пока оно назначено, все клавиши принадлежат окну: иначе назначение
    /// пробела заодно ставило бы плеер на паузу.
    pub catching: Option<Command>,
    /// У кого отобрали клавишу последним назначением.
    pub taken_from: Option<Command>,
}

impl PithApp {
    pub fn open_hotkeys(&mut self) {
        self.hotkeys_state.open = true;
    }

    pub fn close_hotkeys(&mut self) {
        self.hotkeys_state.open = false;
        self.hotkeys_state.catching = None;
    }

    pub fn hotkeys_open(&self) -> bool {
        self.hotkeys_state.open
    }

    /// Нынешняя схема клавиш.
    pub fn hotkeys(&self) -> &Hotkeys {
        &self.settings.hotkeys
    }

    /// Ждём ли сейчас нажатия клавиши для назначения.
    pub fn catching_hotkey(&self) -> bool {
        self.hotkeys_state.catching.is_some()
    }

    /// Какому действию ловим клавишу.
    pub fn caught_command(&self) -> Option<Command> {
        self.hotkeys_state.catching
    }

    /// У кого отобрали клавишу последним назначением.
    pub fn hotkey_taken_from(&self) -> Option<Command> {
        self.hotkeys_state.taken_from
    }

    /// Начинает ловить клавишу для действия.
    pub fn catch_hotkey(&mut self, command: Command) {
        self.hotkeys_state.catching = Some(command);
        self.hotkeys_state.taken_from = None;
    }

    /// Прекращает ловлю, ничего не меняя.
    pub fn stop_catching_hotkey(&mut self) {
        self.hotkeys_state.catching = None;
    }

    /// Назначает пойманную клавишу.
    ///
    /// Занятая клавиша снимается с прежнего действия — об этом окно
    /// и говорит строкой под списком.
    pub fn assign_hotkey(&mut self, command: Command, binding: Binding) {
        let taken = self.settings.hotkeys.assign(command, binding.clone());

        self.hotkeys_state.catching = None;
        self.hotkeys_state.taken_from = taken;
        self.save_settings();

        tracing::info!(
            действие = ?command,
            клавиша = %binding.label(),
            отобрана_у = ?taken,
            "клавиша назначена"
        );
    }

    /// Снимает клавишу с действия.
    pub fn clear_hotkey(&mut self, command: Command) {
        self.settings.hotkeys.assign(command, Binding::default());
        self.hotkeys_state.taken_from = None;
        self.save_settings();

        tracing::info!(действие = ?command, "клавиша снята");
    }

    /// Возвращает всю схему к умолчаниям.
    pub fn reset_hotkeys(&mut self) {
        self.settings.hotkeys.reset();
        self.hotkeys_state.catching = None;
        self.hotkeys_state.taken_from = None;
        self.save_settings();

        tracing::info!("схема клавиш возвращена к умолчаниям");
    }

    /// Отличается ли схема от умолчаний.
    pub fn hotkeys_changed(&self) -> bool {
        self.settings.hotkeys.changed()
    }
}
