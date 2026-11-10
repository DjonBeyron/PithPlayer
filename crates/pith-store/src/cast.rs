//! Составы актёров, разобранные по фильмам.
//!
//! Состав приходит из базы фильмов и хранится, чтобы после перезапуска
//! список был заполнен и в сеть ходить не пришлось. Ключ — тот же, что
//! у закладок: имя файла без расширения.

use serde::{Deserialize, Serialize};

use crate::file::{read_json, write_json};
use crate::paths::DataPaths;

/// Версия формата файла.
const FORMAT_VERSION: u32 = 1;

/// Один актёр в сохранённом составе.
///
/// Свой тип, а не заимствованный у крейта базы фильмов: хранилище
/// не должно зависеть от чужого API, а он однажды сменится.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CastMember {
    /// Номер в базе — по нему запрашивается русское имя.
    pub id: i64,
    pub name: String,
    /// Кого играет. Пусто — база роли не знает.
    pub role: Option<String>,
    /// Путь к фотографии внутри базы, без адреса сервера.
    pub photo: Option<String>,
}

impl CastMember {
    /// Подпись для списка: «Имя (Роль)».
    pub fn label(&self) -> String {
        match &self.role {
            Some(role) if !role.is_empty() => format!("{} ({role})", self.name),
            _ => self.name.clone(),
        }
    }
}

/// Состав одной картины.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VideoCast {
    /// Имя файла без расширения — ключ, как у закладок.
    pub video_file_name: String,
    /// Название картины, как его знает база: по нему видно, ту ли нашли.
    pub title: String,
    /// Год выпуска, если база его знает.
    pub year: Option<u32>,
    pub members: Vec<CastMember>,
    /// Состав создан или правится вручную, без базы фильмов.
    #[serde(default)]
    pub manual: bool,
}

#[derive(Debug, Serialize, Deserialize)]
struct CastFile {
    version: u32,
    videos: Vec<VideoCast>,
    /// Окно актёров открывается в ручном режиме (без базы фильмов).
    #[serde(default)]
    manual_mode: bool,
}

impl Default for CastFile {
    fn default() -> Self {
        Self {
            version: FORMAT_VERSION,
            videos: Vec::new(),
            manual_mode: false,
        }
    }
}

/// Приставка пути фотографии, лежащей на диске, а не в базе фильмов.
pub const LOCAL_PHOTO: &str = "file:";

/// Хранилище составов.
pub struct CastStore {
    paths: DataPaths,
    data: CastFile,
}

impl CastStore {
    /// Читает составы. Отсутствие файла — не ошибка.
    pub fn load(paths: DataPaths) -> Self {
        let data: CastFile = read_json(&paths.cast())
            .unwrap_or_else(|e| {
                tracing::warn!(error = %e, "не удалось прочитать составы актёров");
                None
            })
            .unwrap_or_default();

        tracing::debug!(картин = data.videos.len(), "составы загружены");
        Self { paths, data }
    }

    /// Состав картины, если он уже сохранён.
    pub fn for_video(&self, video_file_name: &str) -> Option<&VideoCast> {
        self.data
            .videos
            .iter()
            .find(|v| v.video_file_name == video_file_name)
    }

    /// Записывает состав, заменяя прежний.
    ///
    /// Вписанные руками люди (номер меньше нуля) переносятся в новый
    /// состав: обновление списка из базы их не стирает.
    pub fn replace(&mut self, mut cast: VideoCast) {
        if let Some(old) = self.for_video(&cast.video_file_name) {
            let kept: Vec<CastMember> = old.members.iter().filter(|m| m.id < 0).cloned().collect();
            cast.members.extend(kept);
        }

        self.data
            .videos
            .retain(|v| v.video_file_name != cast.video_file_name);
        self.data.videos.push(cast);
        self.save();
    }

    /// Открывать ли окно актёров в ручном режиме.
    pub fn manual_mode(&self) -> bool {
        self.data.manual_mode
    }

    pub fn set_manual_mode(&mut self, on: bool) {
        if self.data.manual_mode != on {
            self.data.manual_mode = on;
            self.save();
        }
    }

    /// Состав картины для правки руками; создаётся пустым, если его не было.
    fn manual_cast(&mut self, video: &str) -> &mut VideoCast {
        if self.for_video(video).is_none() {
            self.data.videos.push(VideoCast {
                video_file_name: video.to_string(),
                title: String::new(),
                year: None,
                members: Vec::new(),
                manual: true,
            });
        }

        self.data
            .videos
            .iter_mut()
            .find(|v| v.video_file_name == video)
            .expect("состав только что создан")
    }

    /// Вписывает название картины руками.
    pub fn set_title(&mut self, video: &str, title: &str) {
        let title = title.trim();
        let cast = self.manual_cast(video);

        if cast.title != title {
            cast.title = title.to_string();
            self.save();
        }
    }

    /// Добавляет человека руками. Номер — отрицательный, чтобы не пересечься
    /// с номерами базы фильмов. Возвращает номер.
    pub fn add_manual_member(
        &mut self,
        video: &str,
        name: &str,
        role: Option<String>,
        photo: Option<String>,
    ) -> i64 {
        let cast = self.manual_cast(video);
        let id = cast.members.iter().map(|m| m.id).min().unwrap_or(0).min(0) - 1;

        cast.members.push(CastMember {
            id,
            name: name.trim().to_string(),
            role: role.filter(|r| !r.trim().is_empty()),
            photo,
        });
        self.save();

        id
    }

    /// Меняет фотографию человека.
    pub fn set_member_photo(&mut self, video: &str, id: i64, photo: Option<String>) -> bool {
        let Some(member) = self
            .data
            .videos
            .iter_mut()
            .find(|v| v.video_file_name == video)
            .and_then(|v| v.members.iter_mut().find(|m| m.id == id))
        else {
            return false;
        };

        member.photo = photo;
        self.save();
        true
    }

    /// Убирает человека из состава.
    pub fn remove_member(&mut self, video: &str, id: i64) -> bool {
        let Some(cast) = self
            .data
            .videos
            .iter_mut()
            .find(|v| v.video_file_name == video)
        else {
            return false;
        };

        let before = cast.members.len();
        cast.members.retain(|m| m.id != id);

        let removed = cast.members.len() != before;
        if removed {
            self.save();
        }
        removed
    }

    /// Переименовывает актёра в составе картины.
    ///
    /// Нужно там, где базы молчат: русское имя есть не у всех — эпизодникам
    /// его не досталось ни в TMDB, ни в Wikidata, потому что по-русски о них
    /// никто не писал. Вписанное руками имя живёт здесь же, рядом с картиной,
    /// и в Notion уезжает уже оно.
    ///
    /// Возвращает `true`, если такой человек нашёлся и имя правда изменилось.
    pub fn rename_member(&mut self, video_file_name: &str, actor_id: i64, name: &str) -> bool {
        let name = name.trim();

        if name.is_empty() {
            return false;
        }

        let Some(video) = self
            .data
            .videos
            .iter_mut()
            .find(|v| v.video_file_name == video_file_name)
        else {
            return false;
        };

        let Some(member) = video.members.iter_mut().find(|m| m.id == actor_id) else {
            return false;
        };

        if member.name == name {
            return false;
        }

        tracing::info!(было = %member.name, стало = %name, "имя актёра исправлено");
        member.name = name.to_string();
        self.save();

        true
    }

    /// Убирает состав картины. Нужно, когда состав нашли неверный.
    pub fn forget(&mut self, video_file_name: &str) {
        let before = self.data.videos.len();
        self.data
            .videos
            .retain(|v| v.video_file_name != video_file_name);

        if self.data.videos.len() != before {
            self.save();
        }
    }

    fn save(&self) {
        if let Err(e) = write_json(&self.paths.cast(), &self.data) {
            tracing::error!(error = %e, "не удалось сохранить составы актёров");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{CastMember, CastStore, VideoCast};
    use crate::paths::DataPaths;

    fn состав(file: &str, name: &str) -> VideoCast {
        VideoCast {
            video_file_name: file.into(),
            title: "Титаник".into(),
            year: Some(1997),
            manual: false,
            members: vec![CastMember {
                id: 6193,
                name: name.into(),
                role: Some("Jack Dawson".into()),
                photo: Some("/abc.jpg".into()),
            }],
        }
    }

    #[test]
    fn имя_актёра_правится_и_переживает_перезапуск() {
        let dir = tempfile::tempdir().expect("временный каталог");
        let paths = DataPaths::with_root(dir.path());

        let mut store = CastStore::load(paths.clone());
        store.replace(состав("кино.mkv", "Mark St. Cyr"));

        assert!(store.rename_member("кино.mkv", 6193, "Марк Сент-Сир"));

        let reopened = CastStore::load(paths);
        let member = &reopened.for_video("кино.mkv").expect("состав").members[0];

        assert_eq!(member.name, "Марк Сент-Сир");
    }

    #[test]
    fn пустое_имя_и_чужой_номер_ничего_не_меняют() {
        let dir = tempfile::tempdir().expect("временный каталог");
        let mut store = CastStore::load(DataPaths::with_root(dir.path()));
        store.replace(состав("кино.mkv", "Mark St. Cyr"));

        assert!(!store.rename_member("кино.mkv", 6193, "   "), "пустое имя");
        assert!(
            !store.rename_member("кино.mkv", 999, "Кто-то"),
            "чужой номер"
        );
        assert!(
            !store.rename_member("другое.mkv", 6193, "Кто-то"),
            "чужой файл"
        );
        assert!(
            !store.rename_member("кино.mkv", 6193, "Mark St. Cyr"),
            "то же имя"
        );

        let member = &store.for_video("кино.mkv").expect("состав").members[0];
        assert_eq!(member.name, "Mark St. Cyr");
    }

    #[test]
    fn подпись_содержит_роль() {
        let member = состав("f", "Леонардо ДиКаприо").members.remove(0);
        assert_eq!(member.label(), "Леонардо ДиКаприо (Jack Dawson)");
    }

    #[test]
    fn состав_переживает_перезапуск() {
        let dir = tempfile::tempdir().expect("временный каталог");
        let paths = DataPaths::with_root(dir.path());

        let mut store = CastStore::load(paths.clone());
        store.replace(состав("Titanic", "Леонардо ДиКаприо"));

        let reopened = CastStore::load(paths);
        let cast = reopened.for_video("Titanic").expect("состав на месте");

        assert_eq!(cast.title, "Титаник");
        assert_eq!(cast.members.len(), 1);
        assert_eq!(cast.members[0].name, "Леонардо ДиКаприо");
    }

    #[test]
    fn повторная_запись_заменяет_прежний_состав() {
        let dir = tempfile::tempdir().expect("временный каталог");
        let mut store = CastStore::load(DataPaths::with_root(dir.path()));

        store.replace(состав("Titanic", "Старое имя"));
        store.replace(состав("Titanic", "Новое имя"));

        let cast = store.for_video("Titanic").expect("состав на месте");
        assert_eq!(cast.members[0].name, "Новое имя");
    }

    #[test]
    fn вписанное_руками_переживает_перезапуск_и_обновление_из_базы() {
        let dir = tempfile::tempdir().expect("временный каталог");
        let paths = DataPaths::with_root(dir.path());

        let mut store = CastStore::load(paths.clone());
        store.set_title("Фильм", "Мой фильм");
        let id = store.add_manual_member(
            "Фильм",
            "Леон",
            Some("герой".into()),
            Some("file:C:/a.png".into()),
        );
        assert!(id < 0);
        store.set_manual_mode(true);

        let mut reopened = CastStore::load(paths);
        assert!(reopened.manual_mode());
        let cast = reopened.for_video("Фильм").expect("состав");
        assert_eq!(cast.title, "Мой фильм");
        assert_eq!(cast.members[0].label(), "Леон (герой)");

        // Обновление из базы не стирает вписанных людей.
        reopened.replace(состав("Фильм", "Из базы"));
        let cast = reopened.for_video("Фильм").expect("состав");
        assert_eq!(cast.members.len(), 2);
        assert!(cast.members.iter().any(|m| m.name == "Леон"));
    }

    #[test]
    fn человека_можно_убрать_и_сменить_фото() {
        let dir = tempfile::tempdir().expect("временный каталог");
        let mut store = CastStore::load(DataPaths::with_root(dir.path()));
        let id = store.add_manual_member("Ф", "А", None, None);

        assert!(store.set_member_photo("Ф", id, Some("file:x".into())));
        assert!(store.remove_member("Ф", id));
        assert!(!store.remove_member("Ф", id));
    }

    #[test]
    fn забытый_состав_исчезает() {
        let dir = tempfile::tempdir().expect("временный каталог");
        let mut store = CastStore::load(DataPaths::with_root(dir.path()));

        store.replace(состав("Titanic", "Леонардо ДиКаприо"));
        store.forget("Titanic");

        assert!(store.for_video("Titanic").is_none());
    }

    #[test]
    fn чужой_фильм_не_находится() {
        let dir = tempfile::tempdir().expect("временный каталог");
        let store = CastStore::load(DataPaths::with_root(dir.path()));

        assert!(store.for_video("Titanic").is_none());
    }
}
