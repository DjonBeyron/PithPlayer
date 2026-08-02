//! Связывание видеофайлов с плеером (PLAN.md §6.7).
//!
//! Действие меняет настройки системы, поэтому выполняется только по
//! явному нажатию и с подтверждением — молча ассоциации не трогаем.

use crate::associations;

use super::PithApp;

impl PithApp {
    /// Связаны ли видеофайлы с плеером сейчас.
    ///
    /// Спрашивается при отрисовке меню, поэтому ответ запоминается:
    /// лезть в реестр каждый кадр незачем.
    pub fn file_types_registered(&mut self) -> bool {
        *self.file_types_registered.get_or_insert_with(|| {
            crate::slow::probe("чтение ассоциаций из реестра", associations::is_registered)
        })
    }

    /// Ждёт ли подтверждения смена ассоциаций.
    pub fn file_types_prompt(&self) -> Option<&FileTypesPrompt> {
        self.file_types_prompt.as_ref()
    }

    /// Спрашивает подтверждение: связать файлы или отвязать.
    pub fn ask_file_types(&mut self) {
        let registered = self.file_types_registered();

        self.file_types_prompt = Some(if registered {
            FileTypesPrompt::Unregister
        } else {
            FileTypesPrompt::Register
        });
    }

    pub fn cancel_file_types(&mut self) {
        self.file_types_prompt = None;
    }

    /// Выполняет подтверждённое действие.
    pub fn confirm_file_types(&mut self) {
        let Some(prompt) = self.file_types_prompt.take() else {
            return;
        };

        match prompt {
            FileTypesPrompt::Register => self.register_file_types(),
            FileTypesPrompt::Unregister => self.unregister_file_types(),
        }

        // Состояние в реестре изменилось — перечитаем при следующем спросе.
        self.file_types_registered = None;
    }

    fn register_file_types(&mut self) {
        let result = associations::current_exe().and_then(|exe| associations::register(&exe));

        match result {
            Ok(count) => self.show_notice(&format!("Видеофайлы связаны с плеером: {count}")),
            Err(e) => {
                tracing::error!(error = %e, "не удалось связать файлы");
                self.show_notice("Не удалось изменить ассоциации");
            }
        }
    }

    fn unregister_file_types(&mut self) {
        match associations::unregister() {
            Ok(()) => self.show_notice("Связь с видеофайлами снята"),
            Err(e) => {
                tracing::error!(error = %e, "не удалось снять ассоциации");
                self.show_notice("Не удалось изменить ассоциации");
            }
        }
    }
}

/// Что подтверждает пользователь.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileTypesPrompt {
    Register,
    Unregister,
}

impl FileTypesPrompt {
    pub fn title(self) -> &'static str {
        match self {
            Self::Register => "Связать видеофайлы с плеером?",
            Self::Unregister => "Снять связь с видеофайлами?",
        }
    }

    pub fn explanation(self) -> String {
        match self {
            Self::Register => format!(
                "Плеер появится в списке «Открыть с помощью» для {} расширений: {}.\n\
                 Программа по умолчанию не меняется — её выбираете вы сами \
                 в настройках Windows.",
                associations::EXTENSIONS.len(),
                associations::EXTENSIONS.join(", ")
            ),
            Self::Unregister => "Записи плеера будут удалены из списка программ для видеофайлов. \
                 Чужие настройки не затрагиваются."
                .to_string(),
        }
    }

    pub fn confirm_label(self) -> &'static str {
        match self {
            Self::Register => "Связать",
            Self::Unregister => "Снять связь",
        }
    }
}
