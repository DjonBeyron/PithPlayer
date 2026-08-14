//! Состояние приложения и главный цикл.
//!
//! Всё состояние живёт здесь и нигде больше (PLAN.md §12.4) — в v4 оно было
//! размазано по семи partial-файлам `MainForm`.

mod actor_rename;
mod actors;
mod audio;
mod bookmark_rename;
mod bookmarks;
mod bookmarks_panel;
mod child_window;
mod clipboard;
mod crop;
mod dialogs;
mod dictionary;
mod export;
mod export_log;
mod export_run;
mod export_start;
mod extraction;
mod extraction_queue;
mod file_types;
mod fragment_settings;
mod frame;
mod history;
mod hotkeys;
mod import_v4;
mod integrations;
mod language;
mod lifecycle;
mod list_dialog;
mod lists;
mod photos;
mod playback;
mod preview;
mod preview_source;
mod search;
mod seek;
mod startup;
mod subtitle_style;
mod subtitles;
mod transcription;
mod update;
mod viewport;
mod warmup;
mod watching;
mod window_state;

use std::path::PathBuf;

use pith_mpv::{Engine, HwDec};
use pith_store::{DataPaths, Settings, WatchPositions};

use crate::bench::Metrics;

pub use actor_rename::ActorRename;
pub use actors::{ActorsState, CastStatus, PhotoPreview};
pub use bookmark_rename::BookmarkRename;
use clipboard::Notice;
pub use export::{ExportDialog, ExportStage, NameLanguage};
pub use export_log::{LogKind, LogLine};
pub use file_types::FileTypesPrompt;
pub use fragment_settings::FragmentSettingsDialog;
pub use hotkeys::HotkeysState;
pub use integrations::{AccessStatus, IntegrationsState};
pub use lists::ListDialog;
pub use photos::PhotoSize;
pub use subtitles::SubtitleText;
pub use update::{UpdateStage, UpdateState};
pub use watching::ResumeOffer;

pub struct PithApp {
    engine: Option<Engine>,
    /// Сообщение об ошибке запуска движка. Показывается вместо интерфейса.
    fatal_error: Option<String>,
    pub metrics: Metrics,
    pub hwdec: HwDec,
    /// Показывать ли панель замеров. Выбор запоминается между запусками.
    show_metrics: bool,
    /// Что сейчас написано в заголовке окна — чтобы не слать команду зря.
    window_title: Option<String>,
    /// Последнее известное положение окна — запоминается при закрытии.
    window_geometry: Option<pith_store::WindowGeometry>,
    /// Окно занимает весь экран — развёрнуто кнопкой или открыто таким.
    window_maximized: bool,
    /// Системе ещё не сказано, что окно открыто развёрнутым.
    announce_maximized_pending: bool,
    /// Открытие файла ещё не доделано — ждём первого кадра.
    playback_started_pending: bool,
    /// Нужно проверить, что восстановленное окно видно на экране.
    window_position_pending: bool,
    /// Окно восстановлено из настроек, и первый файл его не переставляет.
    restored_geometry_pending: bool,
    /// Перемотка ожидает подтверждения — нужна для замера её длительности.
    seek_pending: bool,
    /// Куда перематываем. Показывается вместо позиции mpv, пока он догоняет.
    seek_target: Option<f64>,
    /// Перемотка с клавиатуры отправлена и ещё не завершилась.
    key_seek_in_flight: bool,
    /// Куда просят перемотать, пока движок занят прошлой перемоткой.
    key_seek_wanted: Option<f64>,
    /// Следующее место брать грубо, по опорному кадру: клавишу жмут подряд.
    key_seek_rough: bool,
    /// Последняя перемотка была грубой — место нужно довести точно.
    key_seek_needs_exact: bool,
    /// Куда нужно перемотать, когда движок освободится.
    scrub_wanted: Option<f64>,
    /// Перемотка отправлена и ещё не завершилась.
    scrub_in_flight: bool,
    /// Куда уже отправляли: не просим движок о том же месте дважды.
    scrub_sent: Option<f64>,
    /// Паузу поставили мы сами на время перетаскивания.
    paused_by_scrub: bool,
    /// Пользователь тянет ползунок прямо сейчас.
    scrubbing: bool,
    /// Полноэкранный режим.
    fullscreen: bool,
    /// Когда мышь двигалась последний раз — по этому прячется панель.
    last_pointer_activity: f64,
    /// Подгонять ли окно под форму видео.
    fit_window_enabled: bool,
    /// Пользователь менял размер окна вручную — больше не навязываем подгонку.
    window_resized_by_user: bool,
    /// Размер окна, который мы задали сами: помогает отличить свой ресайз
    /// от пользовательского.
    expected_window_size: Option<egui::Vec2>,
    /// Файл только что загружен — нужно подогнать окно в ближайшем кадре.
    fit_window_pending: bool,
    /// Позиции просмотра.
    watch_positions: WatchPositions,
    /// Путь к текущему файлу — ключ для позиции просмотра.
    current_path: Option<PathBuf>,
    /// Предложение продолжить просмотр, пока пользователь не ответил.
    resume_offer: Option<ResumeOffer>,
    /// Позиция, записанная в хранилище последней.
    last_position_save: f64,
    /// Приём файлов от других запусков плеера.
    instance: crate::single_instance::InstanceServer,
    /// Итог переноса данных из версии 4 — показывается один раз.
    migration: Option<pith_store::MigrationReport>,
    /// Настройки плеера.
    settings: Settings,
    /// Каталог данных — нужен для сохранения настроек.
    data_paths: DataPaths,
    /// Текущие реплики субтитров.
    subtitle_text: SubtitleText,
    /// Последняя прозвучавшая реплика — из неё берётся название закладки,
    /// поставленной в тишине между репликами.
    last_subtitle: Option<subtitles::RecentLine>,
    /// Всплывающее уведомление.
    notice: Option<Notice>,
    /// Почему не играет текущий файл.
    ///
    /// В отличие от уведомления не гаснет: иначе пользователь остаётся
    /// перед чёрным окном без единого объяснения.
    playback_error: Option<String>,
    /// Время текущего кадра — по нему гаснут уведомления.
    frame_time: f64,
    /// Дорожки текущего файла. Список меняется только при смене файла.
    tracks: Vec<pith_mpv::Track>,
    /// Какие дорожки выбраны сейчас.
    selected_tracks: subtitles::SelectedTracks,
    /// Поиск по субтитрам.
    search: search::SearchState,
    /// Открыто ли окно настройки вида субтитров.
    subtitle_style_open: bool,
    /// Цвет или начертание меняли — их нужно записать в настройки.
    subtitle_style_dirty: bool,
    /// Закладки и списки отрезков.
    bookmarks: pith_store::Bookmarks,
    /// Составы актёров по картинам.
    cast_store: pith_store::CastStore,
    /// Разогрев словарной памяти во время просмотра.
    warmup: warmup::SoundWarmup,
    /// Транскрипции слов, найденные в словарях.
    ///
    /// Общие на все картины и на все запуски: реплики повторяются,
    /// а спрашивать словарь дорого — секунда на слово.
    sounds: pith_store::SoundStore,
    /// Окно актёров.
    actors: ActorsState,
    /// Фотографии актёров: кэш текстур и загрузок.
    photos: photos::PhotoCache,
    /// Окно интеграций: доступ к Notion и ключ базы фильмов.
    integrations: IntegrationsState,

    /// Окно горячих клавиш и ловля назначаемой клавиши.
    hotkeys_state: HotkeysState,

    /// Кого сейчас переименовывают в окне актёров.
    actor_rename: Option<ActorRename>,

    /// Окно обновления: что вышло и что уже скачано.
    update: UpdateState,
    /// Спрашивали ли GitHub в этот запуск — тихая проверка идёт один раз.
    update_checked: bool,
    /// Открытое окно выгрузки отрезков в Notion.
    export: Option<ExportDialog>,
    /// Панель отрезков показана наведением на правый край.
    bookmarks_panel: bool,
    /// Панель закреплена через меню и не прячется сама.
    bookmarks_panel_pinned: bool,
    /// Окну откреплённой панели уже назначено место.
    bookmarks_window_placed: bool,
    /// Сколько кадров панель ещё рисуется невидимой.
    ///
    /// Первый кадр egui считает разметку списка закладок и полосу прокрутки,
    /// и при длинном списке видно, как панель достраивается. Показываем её,
    /// когда размеры уже посчитаны.
    bookmarks_panel_warmup: u8,
    /// Открытый диалог работы со списком отрезков.
    list_dialog: Option<ListDialog>,
    /// Открытое переименование закладки.
    bookmark_rename: Option<BookmarkRename>,
    /// Ждёт подтверждения очистка списка закладок.
    clear_list_pending: bool,
    /// Открытый диалог общих настроек нарезки.
    fragment_settings: Option<FragmentSettingsDialog>,
    /// Спрошенное подтверждение на смену файловых ассоциаций.
    file_types_prompt: Option<FileTypesPrompt>,
    /// Связаны ли файлы с плеером. `None` — ещё не спрашивали реестр.
    file_types_registered: Option<bool>,
    /// Ход нарезки.
    extraction: extraction::ExtractionState,
    /// Обрезка чёрных полей.
    crop: crop::CropState,
    /// Предпросмотр кадра при перемотке.
    preview: preview::PreviewState,
    /// Состояние, которое показывает значок в центре кадра.
    badge_paused: bool,
    /// Когда паузу переключали последний раз. `None` — ещё не трогали.
    badge_started: Option<f64>,
    /// Когда окно последний раз вернуло себе фокус.
    focus_regained_at: Option<f64>,
    /// Громкость меняли — её нужно запомнить в настройках.
    volume_changed: bool,
    /// До какого времени висит подсказка о перемотке клавишами.
    seek_hud_until: Option<f64>,
    /// История открытых файлов и папок.
    history: pith_store::History,
    /// Открыто ли окно истории.
    history_open: bool,
    /// Время кадра, в котором историю открыли.
    history_opened_at: Option<f64>,
    /// В начале кадра было открыто меню или другое всплывающее окно.
    ///
    /// Нажатие мимо меню его закрывает, и этим же нажатием не должна
    /// переключаться пауза.
    menu_was_open: bool,
}

impl PithApp {
    pub fn engine(&self) -> Option<&Engine> {
        self.engine.as_ref()
    }

    /// Видна ли панель замеров.
    pub fn show_metrics(&self) -> bool {
        self.show_metrics
    }

    /// Прячет или возвращает панель замеров. Выбор запоминается.
    pub fn toggle_metrics(&mut self) {
        self.show_metrics = !self.show_metrics;
        self.settings.show_metrics = self.show_metrics;
        self.settings.save(&self.data_paths);
    }
}
