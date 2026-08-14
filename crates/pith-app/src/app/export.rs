//! Выгрузка отрезков в Notion: что спрашиваем, как считаем и чем кончилось.
//!
//! Выгружается активный список: строка на отрезок, номер по порядку
//! заголовком. Строки уходят в одну рабочую базу — ту, что пользователь
//! однажды сделал копией образца; картины различаются полем `FILM NAME`.
//! Сама работа с Notion — в `pith-notion`, окно — в `ui/export.rs`.

use std::sync::mpsc::Receiver;

use pith_notion::{Kind, Prepared, Report};
use pith_store::Sound as StoredSound;

use super::export_log::{LogKind, LogLine};
use super::export_run::finish;
use super::export_start::summary;

use super::PithApp;

/// Чем занято окно выгрузки.
pub enum ExportStage {
    /// Спрашиваем название картины и её вид.
    Asking,
    /// Идёт транскрипция: сколько слов спрошено из скольких.
    ///
    /// Отдельно от выгрузки строк: словари медленные, и на длинном списке
    /// это самая долгая часть работы — человек должен видеть, что она идёт.
    Sounding { done: usize, total: usize },
    /// Идёт выгрузка: сколько строк создано из скольких.
    Working { done: usize, total: usize },
    /// Выгрузка кончилась — итог с числами.
    Done(Report),
    /// Не вышло вовсе: до строк дело не дошло.
    Failed(String),
}

/// На каком языке брать название картины.
///
/// Русское название приходит из базы фильмов вместе с составом и лежит
/// в `cast.json` — отдельных запросов не нужно. Английское берётся из имени
/// файла: там оно и есть, обычно в исходном виде.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NameLanguage {
    Ru,
    En,
}

/// Окно выгрузки.
pub struct ExportDialog {
    pub stage: ExportStage,
    /// Название картины без вида: «Обсессия».
    pub title: String,
    /// Брать название из имени файла, а не из поля.
    pub from_file_name: bool,
    /// На каком языке взять готовое название.
    pub language: NameLanguage,
    pub kind: Kind,
    /// Вырезать отрезки сразу после выгрузки.
    pub cut_after: bool,
    /// Заполнять транскрипцию реплик.
    ///
    /// Самая долгая часть выгрузки: новое слово стоит около секунды,
    /// зато известные берутся из хранилища мгновенно. Выключается,
    /// когда важнее скорость.
    pub transcribe: bool,
    /// Название, вычисленное из имени открытого файла.
    from_file: String,
    /// Русское название из базы фильмов, если состав уже запрашивали.
    from_cast: String,
    /// Место окну уже назначено — второй раз не навязываем.
    pub(super) placed: bool,
    /// Ответы Notion, добытые заранее: с ними выгрузка идёт сразу к строкам.
    pub(super) prepared: Option<Prepared>,
    /// Подготовка ещё идёт.
    pub(super) preparing: Option<Receiver<Result<Prepared, String>>>,
    /// Подготовка не удалась — причина словами.
    ///
    /// Видна в окне сразу, не дожидаясь нажатия: обычно это отсутствие
    /// доступа к странице, и узнать об этом лучше до, а не после.
    pub prepare_failed: Option<String>,
    /// Подготовку ещё не начинали.
    ///
    /// Начинается на первом кадре окна: там есть `Context`, без которого
    /// рабочий поток не сможет разбудить интерфейс готовым ответом.
    pub(super) prefetch_pending: bool,
    /// Журнал: что происходило и откуда бралось каждое значение.
    ///
    /// Виден в самом окне, а не только в файле журнала плеера: понять,
    /// взялось слово из памяти или из сети, нужно тому, кто нажал кнопку,
    /// и нужно сразу.
    pub(super) log: Vec<LogLine>,
    /// Окно показано.
    ///
    /// Закрытое посреди работы окно её не прерывает: выгрузка идёт дальше,
    /// а плеер возвращается в распоряжение пользователя. Держать человека
    /// у полосы двенадцать секунд незачем — работа от этого не быстрее.
    pub(super) visible: bool,
    pub(super) events: Option<Receiver<ExportEvent>>,
}

impl ExportDialog {
    /// Название, которое уйдёт в Notion, — уже с видом картины.
    pub fn film_name(&self) -> String {
        pith_notion::film_name(self.kind, self.chosen_title())
    }

    /// Что стоит в поле названия сейчас.
    pub fn chosen_title(&self) -> &str {
        if !self.from_file_name {
            return &self.title;
        }

        match self.language {
            NameLanguage::Ru if !self.from_cast.is_empty() => &self.from_cast,
            _ => &self.from_file,
        }
    }

    /// Журнал работы — показывается в самом окне.
    pub fn journal(&self) -> &[LogLine] {
        &self.log
    }

    /// Известно ли русское название.
    ///
    /// Нет — переключатель языка не нажимается: состав картины ещё
    /// не запрашивали, и взять русское название негде.
    pub fn has_russian(&self) -> bool {
        !self.from_cast.is_empty()
    }

    /// Идёт ли работа: пока идёт, окно не закрывают и кнопки не жмут.
    ///
    /// Транскрипция считается работой наравне с созданием строк. Иначе
    /// окно можно было закрыть посреди неё — а вместе с окном пропадала
    /// весть о найденных словах, и в память они не попадали. Заметно это
    /// становилось только на второй выгрузке: слова спрашивались заново.
    pub fn is_working(&self) -> bool {
        matches!(
            self.stage,
            ExportStage::Working { .. } | ExportStage::Sounding { .. }
        )
    }
}

/// Вести от рабочего потока.
pub(super) enum ExportEvent {
    /// Строка журнала: что произошло и откуда взято значение.
    Log(LogLine),
    /// Спрошено слов из скольких — идёт транскрипция.
    Sound {
        done: usize,
        total: usize,
    },
    /// Новые слова: приложение положит их в хранилище.
    Words(Vec<(String, StoredSound)>),
    Step {
        done: usize,
        total: usize,
    },
    Finished(Box<Result<Report, String>>),
}

impl PithApp {
    pub fn export_dialog(&self) -> Option<&ExportDialog> {
        self.export.as_ref()
    }

    pub fn export_dialog_mut(&mut self) -> Option<&mut ExportDialog> {
        self.export.as_mut()
    }

    /// Открывает окно выгрузки.
    ///
    /// Notion не подключён — вместо выгрузки открывается окно интеграций:
    /// спрашивать название картины, когда выгружать некуда, незачем.
    pub fn open_export(&mut self) {
        // Идущую выгрузку не сбрасываем: вместе с окном пропала бы весть
        // о найденных словах, и в память они бы не попали.
        if self.export.as_ref().is_some_and(ExportDialog::is_working) {
            return;
        }

        if !self.settings.notion.is_ready() {
            self.show_notice(crate::tr!(
                "Сначала подключите Notion",
                "Connect Notion first"
            ));
            self.open_integrations();
            return;
        }

        let from_file = self
            .current_video_name()
            .and_then(|name| pith_tmdb::parse_file_name(&name))
            .map(|query| query.title)
            .unwrap_or_default();

        // Русское название уже лежит в составе картины: база фильмов
        // отдаёт его вместе с актёрами, и лишних запросов не нужно.
        let from_cast = self
            .actors_state()
            .cast
            .as_ref()
            .map(|cast| cast.title.clone())
            .unwrap_or_default();

        self.export = Some(ExportDialog {
            stage: ExportStage::Asking,
            title: from_file.clone(),
            from_file_name: true,
            // По-русски, если база его знает: в Notion у пользователя всё
            // по-русски. Не знает — остаётся имя файла.
            language: if from_cast.is_empty() {
                NameLanguage::En
            } else {
                NameLanguage::Ru
            },
            // Ответы прошлой выгрузки: один и тот же сериал режут неделями,
            // и выбирать одно и то же каждый раз — работа на пустом месте.
            kind: if self.settings.export_series {
                Kind::Series
            } else {
                Kind::Movie
            },
            cut_after: self.settings.export_cut_after,
            transcribe: self.settings.export_transcribe,
            from_file,
            from_cast,
            placed: false,
            prepared: None,
            preparing: None,
            prepare_failed: None,
            prefetch_pending: true,
            log: Vec::new(),
            visible: true,
            events: None,
        });
    }

    /// Сколько слов уже известно словарю плеера.
    ///
    /// Показывается в окне выгрузки: по этому числу видно, насколько
    /// быстрой будет транскрипция — известные слова мгновенны.
    pub fn known_words(&self) -> usize {
        self.sounds.len()
    }

    /// Закрывает окно.
    ///
    /// Идущую выгрузку не прерывает и не ждёт: окно прячется, работа идёт
    /// дальше в своём потоке, а её ход виден в панели отрезков. По концу
    /// плеер скажет уведомлением, чем всё кончилось.
    pub fn close_export(&mut self) {
        self.remember_export_answers();
        self.save_settings();

        if let Some(dialog) = self.export.as_mut()
            && dialog.is_working()
        {
            tracing::debug!("окно выгрузки спрятано, работа продолжается");
            dialog.visible = false;
            return;
        }

        self.export = None;
    }

    /// Показано ли окно выгрузки.
    pub fn export_window_visible(&self) -> bool {
        self.export.as_ref().is_some_and(|dialog| dialog.visible)
    }

    /// Ход спрятанной выгрузки — для строки в панели отрезков.
    ///
    /// `None`, когда окно на виду или выгрузки нет: показывать одно и то же
    /// в двух местах незачем.
    pub fn hidden_export_progress(&self) -> Option<(usize, usize, bool)> {
        let dialog = self.export.as_ref().filter(|dialog| !dialog.visible)?;

        match dialog.stage {
            ExportStage::Sounding { done, total } => Some((done, total, true)),
            ExportStage::Working { done, total } => Some((done, total, false)),
            _ => None,
        }
    }

    /// Запоминает ответы окна: вид картины и переключатели.
    ///
    /// Записываются и при закрытии, и при запуске выгрузки: закрыть окно
    /// можно и крестиком, а ответ уже дан.
    pub(super) fn remember_export_answers(&mut self) {
        let Some(dialog) = self.export.as_ref() else {
            return;
        };

        self.settings.export_series = dialog.kind == Kind::Series;
        self.settings.export_transcribe = dialog.transcribe;
        self.settings.export_cut_after = dialog.cut_after;
    }

    /// Ставит окно выгрузки туда, где оно стояло в прошлый раз.
    ///
    /// Только при показе: дальше размером и положением распоряжается
    /// пользователь.
    pub fn place_export_window(
        &mut self,
        builder: egui::ViewportBuilder,
        default_size: [f32; 2],
    ) -> egui::ViewportBuilder {
        let saved = self.settings.export_window;
        let Some(dialog) = self.export.as_mut() else {
            return builder;
        };

        super::child_window::place(builder, saved, default_size, &mut dialog.placed)
    }

    /// Запоминает, куда окно перетащили. На диск ляжет при закрытии.
    pub fn track_export_window(&mut self, ctx: &egui::Context) {
        if let Some(geometry) = super::child_window::geometry(ctx) {
            self.settings.export_window = Some(geometry);
        }
    }

    /// Забирает вести от рабочего потока.
    pub(super) fn poll_export(&mut self) {
        let Some(dialog) = self.export.as_mut() else {
            return;
        };
        let Some(events) = dialog.events.as_ref() else {
            return;
        };

        // Вести сначала собираем, потом разбираем: обработка отпускает
        // канал (по концу выгрузки), а он ещё занят заимствованием.
        let news: Vec<ExportEvent> = std::iter::from_fn(|| events.try_recv().ok()).collect();

        let mut cut = false;
        let mut fresh = Vec::new();
        let mut done = None;

        for event in news {
            match event {
                ExportEvent::Log(line) => dialog.log.push(line),
                ExportEvent::Sound { done, total } => {
                    dialog.stage = ExportStage::Sounding { done, total };
                }
                ExportEvent::Words(words) => fresh = words,
                ExportEvent::Step { done, total } => {
                    dialog.stage = ExportStage::Working { done, total };
                }
                ExportEvent::Finished(answer) => {
                    dialog.events = None;

                    // Нарезка идёт только за удавшейся выгрузкой: незачем
                    // резать файлы, если в Notion ничего не легло.
                    cut = answer.is_ok() && dialog.cut_after;

                    // Окно спрятали — итог сказать всё равно нужно, и сказать
                    // там, где человек сейчас: уведомлением поверх кадра.
                    if !dialog.visible {
                        done = Some(summary(&answer).text);
                    }

                    dialog.stage = finish(*answer);
                }
            }
        }

        // Заём окна отпущен — можно трогать остальное приложение.
        if !fresh.is_empty() {
            let count = fresh.len();

            self.sounds.remember(fresh);
            let known = self.sounds.len();

            if let Some(dialog) = self.export.as_mut() {
                dialog.log.push(LogLine::new(
                    LogKind::Step,
                    crate::tr!(
                        format!("В память добавлено слов {count} · всего в памяти {known}"),
                        format!("Words added to memory {count} · {known} in memory total")
                    ),
                ));
            }
        }

        // Спрятанное окно после работы не нужно: итог сказан уведомлением,
        // а следующее нажатие откроет вопрос заново.
        if let Some(line) = done {
            self.show_notice(&line);
            self.export = None;
        }

        if cut {
            tracing::info!("выгрузка окончена, начинаю нарезку");
            self.start_extraction();
        }
    }
}
