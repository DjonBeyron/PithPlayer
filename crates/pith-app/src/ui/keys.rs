//! Имена клавиш для настроек.
//!
//! Схема горячих клавиш хранится именами (`ArrowRight`, `Space`), потому что
//! файл настроек читают и правят руками. Беда в том, что у egui на ту же
//! клавишу своё имя: `Key::ArrowRight.name()` отдаёт `"Right"`. Имена
//! разошлись — и привязка молча не срабатывала: так перестали работать
//! перемотка стрелками и громкость, а пробел и буквы продолжали, потому
//! что у них имена совпадают.
//!
//! Поэтому перевод имён живёт в одном месте и в обе стороны: и когда
//! клавишу ищут в схеме, и когда её в схему записывают.

/// Имя клавиши, каким оно ложится в настройки.
///
/// Для стрелок берём длинное имя: `ArrowRight` в файле понятнее, чем
/// `Right`, и таким оно записано в умолчаниях. Остальным хватает имени
/// от egui — оно с нашим совпадает.
pub(super) fn stored_name(key: egui::Key) -> &'static str {
    match key {
        egui::Key::ArrowRight => "ArrowRight",
        egui::Key::ArrowLeft => "ArrowLeft",
        egui::Key::ArrowUp => "ArrowUp",
        egui::Key::ArrowDown => "ArrowDown",
        other => other.name(),
    }
}

/// Все имена, под которыми клавиша может лежать в настройках.
///
/// Их два, и оба нужны: схему могли записать и нашим именем, и коротким
/// именем egui — второе попадало в файл, пока имена не были сведены.
/// Пропустить такую запись значило бы оставить человека без клавиши
/// и заставить назначать её заново.
pub(super) fn aliases(key: egui::Key) -> [&'static str; 2] {
    [stored_name(key), key.name()]
}

#[cfg(test)]
mod tests {
    use super::{aliases, stored_name};

    #[test]
    fn у_стрелок_имя_длинное() {
        assert_eq!(stored_name(egui::Key::ArrowRight), "ArrowRight");
        assert_eq!(stored_name(egui::Key::ArrowDown), "ArrowDown");
    }

    #[test]
    fn прочим_клавишам_хватает_имени_egui() {
        for key in [egui::Key::Space, egui::Key::T, egui::Key::Backspace] {
            assert_eq!(stored_name(key), key.name());
        }
    }

    #[test]
    fn среди_имён_есть_короткое_имя_egui() {
        // Именно им клавиша попадала в настройки, пока имена расходились.
        assert!(aliases(egui::Key::ArrowRight).contains(&"Right"));
    }

    /// Любое имя из настроек должно приводиться обратно к клавише: иначе
    /// окно настроек не покажет и не найдёт то, что само же записало.
    #[test]
    fn имена_читаются_обратно() {
        for key in [
            egui::Key::ArrowRight,
            egui::Key::ArrowLeft,
            egui::Key::ArrowUp,
            egui::Key::ArrowDown,
            egui::Key::Space,
            egui::Key::Backspace,
            egui::Key::OpenBracket,
            egui::Key::CloseBracket,
            egui::Key::T,
        ] {
            for name in aliases(key) {
                assert_eq!(
                    egui::Key::from_name(name),
                    Some(key),
                    "имя {name} не читается обратно"
                );
            }
        }
    }
}
