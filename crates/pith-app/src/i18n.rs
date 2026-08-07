//! Язык интерфейса.
//!
//! Перевод лежит парами прямо там, где надпись показывается:
//! `tr!("Открыть файл…", "Open file…")`. Каталога с ключами нет намеренно —
//! при нём строка и её место расходятся по разным файлам, и в каждом окне
//! приходится держать в голове, что означает очередной `menu.open_file`.
//!
//! Текущий язык живёт в одной ячейке на всю программу, а не в состоянии
//! приложения: его спрашивает почти каждая надпись, в том числе там, где
//! `PithApp` рядом нет, — тащить его туда пришлось бы через десяток слоёв.
//! Меняется он только из меню, читается только при отрисовке.

use std::sync::atomic::{AtomicU8, Ordering};

pub use pith_store::Language;

/// Выбранный язык. Хранится номером: атомарная ячейка не умеет иначе.
static CURRENT: AtomicU8 = AtomicU8::new(0);

/// Запоминает язык интерфейса.
pub fn set(language: Language) {
    CURRENT.store(language.code(), Ordering::Relaxed);
}

/// Язык, на котором сейчас говорит интерфейс.
pub fn current() -> Language {
    Language::from_code(CURRENT.load(Ordering::Relaxed))
}

/// Выбирает строку по языку интерфейса.
///
/// Обе ветви — любые выражения одного типа, поэтому работает и с готовым
/// текстом, и с `format!`: вычисляется только выбранная.
#[macro_export]
macro_rules! tr {
    ($ru:expr, $en:expr $(,)?) => {
        match $crate::i18n::current() {
            $crate::i18n::Language::Ru => $ru,
            $crate::i18n::Language::En => $en,
        }
    };
}

/// Имя списка отрезков для показа.
///
/// Список по умолчанию заводится в хранилище под именем «Основной» — это
/// данные, и переименовывать их на ходу нельзя: по имени лежат закладки
/// и называется папка с вырезанными отрезками. Переводим только надпись.
///
/// Списки, названные пользователем, показываются как есть: это его слова.
pub fn list_name(name: &str) -> String {
    if name == pith_store::DEFAULT_LIST {
        return tr!("Основной", "Main").to_string();
    }

    name.to_string()
}

#[cfg(test)]
mod tests {
    use super::{Language, current, set};

    #[test]
    fn язык_переключается() {
        set(Language::En);
        assert_eq!(current(), Language::En);
        assert_eq!(tr!("да", "yes"), "yes");

        set(Language::Ru);
        assert_eq!(current(), Language::Ru);
        assert_eq!(tr!("да", "yes"), "да");
    }

    #[test]
    fn список_по_умолчанию_переводится_только_на_вид() {
        set(Language::En);
        assert_eq!(super::list_name(pith_store::DEFAULT_LIST), "Main");
        assert_eq!(
            super::list_name("Диалоги"),
            "Диалоги",
            "своё имя не трогаем"
        );

        set(Language::Ru);
        assert_eq!(super::list_name(pith_store::DEFAULT_LIST), "Основной");
    }

    #[test]
    fn номер_языка_переживает_обратное_превращение() {
        for language in Language::ALL {
            assert_eq!(Language::from_code(language.code()), language);
        }
    }
}
