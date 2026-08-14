//! Правка имени актёра руками.
//!
//! Базы знают русское имя не у всех: главные роли переведены, эпизодники
//! приходят латиницей — по-русски о них никто не писал, и взять имя
//! неоткуда (замер в `app/actors.rs`). Машинная транслитерация тут хуже
//! молчания: `Sean` — это Шон, а не Сеан, и подставленное наугад имя
//! уехало бы в Notion как достоверное.
//!
//! Поэтому имя вписывают руками, один раз. Оно ложится в `cast.json`
//! рядом с картиной и живёт там, пока состав не перезапрошен.

use super::PithApp;

/// Кого сейчас переименовывают.
pub struct ActorRename {
    /// Номер человека в базе фильмов.
    pub id: i64,
    /// Что набрано в поле.
    pub name: String,
    /// Имя, с которого начали, — по нему видно, менялось ли что-нибудь.
    pub was: String,
}

impl PithApp {
    /// Начинает правку имени.
    pub fn start_actor_rename(&mut self, id: i64, name: &str) {
        self.actor_rename = Some(ActorRename {
            id,
            name: name.to_string(),
            was: name.to_string(),
        });
    }

    /// Кого правят сейчас.
    pub fn actor_rename(&mut self) -> Option<&mut ActorRename> {
        self.actor_rename.as_mut()
    }

    /// Правят ли имя этого человека.
    pub fn renaming_actor(&self, id: i64) -> bool {
        self.actor_rename.as_ref().is_some_and(|r| r.id == id)
    }

    /// Бросает правку, ничего не сохраняя.
    pub fn cancel_actor_rename(&mut self) {
        self.actor_rename = None;
    }

    /// Записывает исправленное имя.
    ///
    /// Пишется и в состав на экране, и в файл: окно берёт список из своего
    /// состояния, а на диске лежит своя копия — разойтись им нельзя.
    pub fn finish_actor_rename(&mut self) {
        let Some(rename) = self.actor_rename.take() else {
            return;
        };

        let name = rename.name.trim().to_string();

        if name.is_empty() || name == rename.was {
            return;
        }

        let Some(file_name) = self.current_video_name() else {
            return;
        };

        if !self.cast_store.rename_member(&file_name, rename.id, &name) {
            return;
        }

        if let Some(cast) = self.actors.cast.as_mut()
            && let Some(member) = cast.members.iter_mut().find(|m| m.id == rename.id)
        {
            member.name = name.clone();
        }

        let notice = crate::tr!(
            format!("Имя исправлено: {name}"),
            format!("Name fixed: {name}")
        );
        self.show_notice(&notice);
    }
}
