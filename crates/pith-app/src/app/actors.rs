//! Окно актёров: состав картины и привязка актёра к отрезку.
//!
//! Состав берётся из базы фильмов по имени открытого файла и хранится
//! в `cast.json`: после перезапуска список уже заполнен, и в сеть плеер
//! не ходит. Здесь только состояние и действия — само окно в `ui/actors.rs`.

use std::sync::mpsc::{Receiver, channel};

use pith_store::{CastMember, VideoCast};
use pith_tmdb::Tmdb;

use super::PithApp;

/// Что окно показывает вместо списка.
///
/// Записанного актёра здесь нет: об этом говорит всплывающее уведомление
/// поверх кадра — то же, что при добавлении закладки. В окне актёров такая
/// строка была не к месту: окно может стоять и на другом экране.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CastStatus {
    /// Список не запрашивали.
    Idle,
    /// Запрос к базе идёт.
    Working,
    /// Запрос не удался — причина словами.
    Failed(String),
}

/// Раскрытая крупно фотография.
///
/// Раскрывается только нажатием. Раскрытие по наведению было — и от него
/// отказались: картинка выскакивала и гасла на каждом движении мышью
/// по списку, и это мешало больше, чем помогало.
#[derive(Debug, Clone, PartialEq)]
pub struct PhotoPreview {
    /// Путь фотографии внутри базы.
    pub path: String,
    /// Подпись — имя актёра с ролью.
    pub label: String,
    /// Когда раскрыли, по времени кадра.
    ///
    /// Нажатие, которым картинку открыли, лежит во входных данных того же
    /// кадра — и закрывает её, не дав показаться. Поэтому первые мгновения
    /// нажатия мимо не считаются.
    pub opened: f64,
}

/// Состояние окна актёров.
pub struct ActorsState {
    /// Окно показано.
    pub open: bool,
    /// Состав открытой картины, если он уже известен.
    pub cast: Option<VideoCast>,
    pub status: CastStatus,
    /// Ключ, набираемый в поле окна, пока его не сохранили.
    pub key_input: String,
    /// Фотография, раскрытая крупно.
    pub preview: Option<PhotoPreview>,
    /// Место окну уже назначено.
    ///
    /// Назначается один раз при показе: называть размер каждый кадр —
    /// значит спорить с пользователем, когда он тянет окно за край.
    placed: bool,
    /// Ответ базы, которого ждём. Запрос идёт в отдельном потоке:
    /// сеть в потоке интерфейса подвесила бы окно на секунды.
    pending: Option<Receiver<Result<VideoCast, String>>>,
}

impl ActorsState {
    /// Окно открывают таким, каким его закрыли.
    pub fn new(open: bool) -> Self {
        Self {
            open,
            cast: None,
            status: CastStatus::Idle,
            key_input: String::new(),
            preview: None,
            placed: false,
            pending: None,
        }
    }
}

impl PithApp {
    pub fn actors_open(&self) -> bool {
        self.actors.open
    }

    pub fn actors_state(&self) -> &ActorsState {
        &self.actors
    }

    /// Показывает и прячет окно — клавишей и пунктом меню.
    pub fn toggle_actors_window(&mut self) {
        self.actors.open = !self.actors.open;
        self.settings.actors_window_open = self.actors.open;
        self.close_actor_photo();

        // Спрятанное окно система уничтожает, и показанное заново нужно
        // снова поставить на место — иначе оно откроется где придётся.
        if !self.actors.open {
            self.actors.placed = false;
        }

        self.save_settings();

        tracing::debug!(открыто = self.actors.open, "окно актёров");
    }

    /// Ставит окно туда, где оно стояло в прошлый раз.
    ///
    /// Только при показе: дальше размером и положением распоряжается
    /// пользователь, а не мы.
    pub fn place_actors_window(
        &mut self,
        builder: egui::ViewportBuilder,
        default_size: [f32; 2],
    ) -> egui::ViewportBuilder {
        let saved = self.settings.actors_window;

        super::child_window::place(builder, saved, default_size, &mut self.actors.placed)
    }

    /// Запоминает, куда окно перетащили.
    ///
    /// На диск не пишем: файл настроек не должен переписываться на каждое
    /// движение мышью. Запись случится при закрытии окна и выходе из плеера.
    pub fn track_actors_window(&mut self, ctx: &egui::Context) {
        if let Some(geometry) = super::child_window::geometry(ctx) {
            self.settings.actors_window = Some(geometry);
        }
    }

    /// Задан ли ключ доступа к базе.
    pub fn has_tmdb_key(&self) -> bool {
        !self.settings.tmdb_key.trim().is_empty()
    }

    /// Запоминает ключ, набранный в окне.
    pub fn save_tmdb_key(&mut self) {
        let key = self.actors.key_input.trim().to_string();

        if key.is_empty() {
            return;
        }

        self.settings.tmdb_key = key;
        self.save_settings();
        self.actors.key_input.clear();

        tracing::info!("ключ доступа к базе фильмов сохранён");
    }

    /// Поле ввода ключа — правится прямо в окне.
    pub fn actors_key_input(&mut self) -> &mut String {
        &mut self.actors.key_input
    }

    /// Раскрывает фотографию крупно — по нажатию на неё.
    pub fn open_actor_photo(&mut self, path: &str, label: &str, now: f64) {
        self.actors.preview = Some(PhotoPreview {
            path: path.to_string(),
            label: label.to_string(),
            opened: now,
        });
    }

    /// Убирает раскрытую фотографию.
    pub fn close_actor_photo(&mut self) {
        self.actors.preview = None;
    }

    /// Просит у базы состав открытой картины.
    ///
    /// Запрос уходит в отдельный поток: сеть в потоке интерфейса подвесила
    /// бы окно на всё время ожидания (CLAUDE.md — блокирующие операции вне
    /// потока интерфейса). Готовый ответ будит интерфейс сам — без этого
    /// он лежал бы в канале до ближайшего движения мышью.
    pub fn request_cast(&mut self, ctx: &egui::Context) {
        let Some(file_name) = self.current_video_name() else {
            return;
        };

        let Some(query) = pith_tmdb::parse_file_name(&file_name) else {
            self.actors.status = CastStatus::Failed(
                crate::tr!(
                    "Из имени файла не вышло названия",
                    "The file name gave no title"
                )
                .into(),
            );
            return;
        };

        let Some(tmdb) = Tmdb::new(&self.settings.tmdb_key) else {
            return;
        };

        tracing::info!(название = %query.title, год = ?query.year, "спрашиваю состав");

        let (sender, receiver) = channel();
        self.actors.pending = Some(receiver);
        self.actors.status = CastStatus::Working;

        let ctx = ctx.clone();

        std::thread::spawn(move || {
            let _ = sender.send(fetch(&tmdb, &query, &file_name));
            ctx.request_repaint();
        });
    }

    /// Забирает ответ базы, если он готов.
    pub(super) fn poll_cast(&mut self) {
        let Some(receiver) = self.actors.pending.as_ref() else {
            return;
        };

        let Ok(answer) = receiver.try_recv() else {
            return;
        };

        self.actors.pending = None;

        match answer {
            Ok(cast) => {
                tracing::info!(
                    картина = %cast.title,
                    актёров = cast.members.len(),
                    "состав получен"
                );

                self.cast_store.replace(cast.clone());
                self.actors.cast = Some(cast);
                self.actors.status = CastStatus::Idle;
            }
            Err(why) => {
                tracing::warn!(причина = %why, "состав получить не удалось");
                self.actors.status = CastStatus::Failed(why);
            }
        }
    }

    /// Подтягивает сохранённый состав при открытии файла.
    pub(super) fn load_cast_for_current(&mut self) {
        self.actors.status = CastStatus::Idle;
        self.actors.cast = self
            .current_video_name()
            .and_then(|name| self.cast_store.for_video(&name).cloned());
    }

    /// Записывает актёра ближайшей к текущему месту закладке.
    ///
    /// Ближайшей, а не только той, в чей отрезок попал курсор: промахнуться
    /// мимо отрезка легко, а намерение всегда одно — подписать соседнюю
    /// метку. Окно потом говорит, какой именно.
    pub fn assign_actor(&mut self, actor_label: &str) {
        let Some(time_ms) = self.current_time_ms() else {
            return;
        };

        let Some(video) = self.current_bookmarks_mut() else {
            return;
        };

        let Some(list) = video.active_mut() else {
            return;
        };

        let Some(target) = list.nearest(time_ms).cloned() else {
            self.actors.status =
                CastStatus::Failed(crate::tr!("Нет ни одной закладки", "No bookmarks yet").into());
            return;
        };

        list.set_actor(target.time_ms, Some(actor_label.to_string()));

        // Номер по порядку — тот же, что уйдёт заголовком строки в Notion:
        // по нему закладку и находят в списке.
        let number = list
            .bookmarks
            .iter()
            .position(|b| b.time_ms == target.time_ms)
            .map_or(0, |index| index + 1);

        self.bookmarks.save();

        // Подтверждение — тем же всплывающим уведомлением поверх кадра,
        // каким плеер отвечает на добавление закладки. Строкой в окне
        // актёров оно было не к месту: смотрят в этот миг на кадр, а окно
        // может стоять и на другом экране.
        let bookmark = target.label();
        self.show_notice(&crate::tr!(
            format!("{actor_label} — закладке {number}: {bookmark}"),
            format!("{actor_label} — to bookmark {number}: {bookmark}")
        ));

        self.actors.status = CastStatus::Idle;

        tracing::info!(
            закладка = target.time_ms,
            номер = number,
            актёр = actor_label,
            "актёр записан закладке"
        );
    }

    /// Текущее место в миллисекундах.
    fn current_time_ms(&self) -> Option<i64> {
        let engine = self.engine.as_ref()?;

        Some((engine.state().position * 1000.0) as i64)
    }
}

/// Спрашивает базу. Выполняется в отдельном потоке.
///
/// Ошибку возвращаем строкой: окно показывает её словами, а разбирать
/// причину по коду ему незачем.
fn fetch(tmdb: &Tmdb, query: &pith_tmdb::Query, file_name: &str) -> Result<VideoCast, String> {
    let title = tmdb.find(query).map_err(|e| e.to_string())?;
    let members = tmdb.cast(&title).map_err(|e| e.to_string())?;

    Ok(VideoCast {
        video_file_name: file_name.to_string(),
        title: title.name,
        year: title.year,
        members: members
            .into_iter()
            .map(|a| CastMember {
                id: a.id,
                name: a.name,
                role: a.role,
                photo: a.photo,
            })
            .collect(),
    })
}
