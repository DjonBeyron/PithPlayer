//! Привязка клавиш к действиям плеера.
//!
//! Раньше клавиши были зашиты в разбор нажатий, и переназначить их было
//! нельзя. Теперь схема живёт здесь: список действий, привязка на каждое
//! и умолчания — те же, что были зашиты (схема из v4, PLAN.md §6.8).
//!
//! Клавиша хранится именем `egui`: `Space`, `ArrowLeft`, `T`. Имя, а не
//! число, потому что файл настроек читают и правят руками, а число кода
//! клавиши ничего человеку не говорит.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// Действие плеера, которому можно назначить клавишу.
///
/// Порядок перечисления — это и порядок строк в окне настроек: сперва
/// то, чем пользуются каждую минуту, потом редкое.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Command {
    TogglePause,
    SeekForward,
    SeekBack,
    VolumeUp,
    VolumeDown,
    Fullscreen,
    SpeedUp,
    SpeedDown,
    SpeedReset,
    AddBookmark,
    RemoveBookmark,
    CopySubtitle,
    ToggleSubtitles,
    OpenSearch,
    ToggleActors,
}

impl Command {
    /// Все действия по порядку.
    pub const ALL: [Self; 15] = [
        Self::TogglePause,
        Self::SeekForward,
        Self::SeekBack,
        Self::VolumeUp,
        Self::VolumeDown,
        Self::Fullscreen,
        Self::SpeedUp,
        Self::SpeedDown,
        Self::SpeedReset,
        Self::AddBookmark,
        Self::RemoveBookmark,
        Self::CopySubtitle,
        Self::ToggleSubtitles,
        Self::OpenSearch,
        Self::ToggleActors,
    ];

    /// Меняет ли модификатор не действие, а его шаг.
    ///
    /// У перемотки и громкости Shift, Ctrl и Alt задают величину шага —
    /// это схема из v4, и руки к ней привыкли. Поэтому такие действия
    /// откликаются на свою клавишу с любым модификатором, а не только
    /// с назначенным.
    pub fn modifier_changes_step(self) -> bool {
        matches!(
            self,
            Self::SeekForward | Self::SeekBack | Self::VolumeUp | Self::VolumeDown
        )
    }

    /// Привязка, с которой действие живёт, пока её не меняли.
    pub fn default_binding(self) -> Binding {
        match self {
            Self::TogglePause => Binding::key("Space"),
            Self::SeekForward => Binding::key("ArrowRight"),
            Self::SeekBack => Binding::key("ArrowLeft"),
            Self::VolumeUp => Binding::key("ArrowUp"),
            Self::VolumeDown => Binding::key("ArrowDown"),
            Self::Fullscreen => Binding::key("F"),
            Self::SpeedUp => Binding::key("CloseBracket"),
            Self::SpeedDown => Binding::key("OpenBracket"),
            Self::SpeedReset => Binding::key("Backspace"),
            Self::AddBookmark => Binding::key("T"),
            Self::RemoveBookmark => Binding::key("T").with_shift(),
            Self::CopySubtitle => Binding::key("C"),
            Self::ToggleSubtitles => Binding::key("V"),
            Self::OpenSearch => Binding::key("F").with_ctrl(),
            Self::ToggleActors => Binding::key("A"),
        }
    }
}

/// Клавиша с модификаторами.
///
/// Модификаторы разобраны по одному, а не числом: файл настроек читают
/// глазами, и `"shift": true` понятнее, чем набор битов.
/// Пустое имя клавиши означает «не назначена» — законное состояние.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Binding {
    /// Имя клавиши в терминах `egui`: `Space`, `ArrowLeft`, `T`.
    pub key: String,
    pub ctrl: bool,
    pub shift: bool,
    pub alt: bool,
}

impl Binding {
    pub fn key(name: &str) -> Self {
        Self {
            key: name.to_string(),
            ..Self::default()
        }
    }

    pub fn with_ctrl(mut self) -> Self {
        self.ctrl = true;
        self
    }

    pub fn with_shift(mut self) -> Self {
        self.shift = true;
        self
    }

    /// Назначена ли клавиша вообще.
    ///
    /// Пустая привязка — законное состояние: действие можно оставить
    /// без клавиши, если она нужнее другому.
    pub fn is_set(&self) -> bool {
        !self.key.is_empty()
    }

    /// Как привязка выглядит в окне настроек: `Shift + T`.
    pub fn label(&self) -> String {
        if !self.is_set() {
            return String::new();
        }

        let mut parts: Vec<&str> = Vec::new();

        if self.ctrl {
            parts.push("Ctrl");
        }
        if self.shift {
            parts.push("Shift");
        }
        if self.alt {
            parts.push("Alt");
        }

        parts.push(&self.key);
        parts.join(" + ")
    }
}

/// Схема горячих клавиш целиком.
///
/// Хранится картой «действие — привязка», а не списком: так в файле
/// настроек видно, какому действию что назначено, и порядок записей
/// не имеет значения.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Hotkeys {
    bindings: BTreeMap<Command, Binding>,
}

impl Default for Hotkeys {
    fn default() -> Self {
        Self {
            bindings: Command::ALL
                .iter()
                .map(|command| (*command, command.default_binding()))
                .collect(),
        }
    }
}

impl Hotkeys {
    /// Привязка действия. Незнакомое — берём умолчание.
    ///
    /// Умолчание нужно потому, что файл настроек пережил появление новых
    /// действий: в нём их просто нет, и без запасного ответа новая
    /// клавиша не работала бы до сброса схемы.
    pub fn binding(&self, command: Command) -> Binding {
        self.bindings
            .get(&command)
            .cloned()
            .unwrap_or_else(|| command.default_binding())
    }

    /// Назначает клавишу действию.
    ///
    /// Занятая клавиша снимается с прежнего действия: две команды на одной
    /// клавише — это не выбор, а неразбериха, и сработали бы обе.
    /// Возвращает действие, у которого клавишу отобрали.
    pub fn assign(&mut self, command: Command, binding: Binding) -> Option<Command> {
        let taken = binding
            .is_set()
            .then(|| self.holder(&binding))
            .flatten()
            .filter(|holder| *holder != command);

        if let Some(holder) = taken {
            self.bindings.insert(holder, Binding::default());
        }

        self.bindings.insert(command, binding);
        taken
    }

    /// Кто держит эту клавишу сейчас.
    pub fn holder(&self, binding: &Binding) -> Option<Command> {
        Command::ALL
            .iter()
            .copied()
            .find(|command| self.binding(*command) == *binding)
    }

    /// Возвращает схему к умолчаниям.
    pub fn reset(&mut self) {
        *self = Self::default();
    }

    /// Отличается ли схема от умолчаний.
    pub fn changed(&self) -> bool {
        *self != Self::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn у_каждого_действия_есть_умолчание() {
        let hotkeys = Hotkeys::default();

        for command in Command::ALL {
            assert!(
                hotkeys.binding(command).is_set(),
                "действие без клавиши: {command:?}"
            );
        }
    }

    #[test]
    fn умолчания_не_спорят_между_собой() {
        let hotkeys = Hotkeys::default();
        let mut seen: Vec<Binding> = Vec::new();

        for command in Command::ALL {
            let binding = hotkeys.binding(command);
            assert!(
                !seen.contains(&binding),
                "клавиша {} занята дважды",
                binding.label()
            );
            seen.push(binding);
        }
    }

    #[test]
    fn занятая_клавиша_снимается_с_прежнего_действия() {
        let mut hotkeys = Hotkeys::default();

        // Пробел с паузы — на закладку.
        let taken = hotkeys.assign(Command::AddBookmark, Binding::key("Space"));

        assert_eq!(taken, Some(Command::TogglePause));
        assert!(!hotkeys.binding(Command::TogglePause).is_set());
        assert_eq!(hotkeys.binding(Command::AddBookmark).key, "Space");
    }

    #[test]
    fn та_же_клавиша_тому_же_действию_ничего_не_отбирает() {
        let mut hotkeys = Hotkeys::default();

        assert_eq!(
            hotkeys.assign(Command::TogglePause, Binding::key("Space")),
            None
        );
        assert_eq!(hotkeys.binding(Command::TogglePause).key, "Space");
    }

    #[test]
    fn модификатор_делает_клавишу_другой() {
        let hotkeys = Hotkeys::default();

        // T и Shift+T — разные привязки, и обе заняты по умолчанию.
        assert_eq!(
            hotkeys.holder(&Binding::key("T")),
            Some(Command::AddBookmark)
        );
        assert_eq!(
            hotkeys.holder(&Binding::key("T").with_shift()),
            Some(Command::RemoveBookmark)
        );
    }

    #[test]
    fn сброс_возвращает_умолчания() {
        let mut hotkeys = Hotkeys::default();
        hotkeys.assign(Command::AddBookmark, Binding::key("B"));
        assert!(hotkeys.changed());

        hotkeys.reset();
        assert!(!hotkeys.changed());
        assert_eq!(hotkeys.binding(Command::AddBookmark).key, "T");
    }

    #[test]
    fn подпись_собирается_с_модификаторами() {
        assert_eq!(Binding::key("T").with_shift().label(), "Shift + T");
        assert_eq!(Binding::key("F").with_ctrl().label(), "Ctrl + F");
        assert_eq!(Binding::default().label(), "");
    }
}
