//! Обновление плеера: проверка выпуска и загрузка установщика.
//!
//! Сеть — в отдельном потоке, как везде (CLAUDE.md). Само окно живёт
//! в `ui/update.rs`, а разговор с GitHub — в крейте `pith-update`.
//!
//! Решение всегда за человеком. Плеер сам ничего не ставит и не
//! перезапускает: он лишь узнаёт, что вышло, и по нажатию скачивает
//! установщик — запустить его тоже просят кнопкой.

use std::path::PathBuf;
use std::sync::mpsc::{Receiver, channel};

use pith_update::Release;

use super::PithApp;

/// Что происходит с обновлением сейчас.
#[derive(Debug, Clone, Default, PartialEq)]
pub enum UpdateStage {
    /// Ничего не спрашивали.
    #[default]
    Idle,
    /// Идёт вопрос к GitHub.
    Checking,
    /// Установлена последняя версия.
    Latest,
    /// Вышло обновление.
    Available(Box<Release>),
    /// Идёт загрузка установщика: сколько байт пришло и сколько всего.
    Downloading { done: u64, total: u64 },
    /// Установщик скачан и ждёт запуска.
    Ready(PathBuf),
    /// Причина неудачи словами.
    Failed(String),
}

/// Состояние окна обновления.
#[derive(Default)]
pub struct UpdateState {
    pub open: bool,
    pub stage: UpdateStage,
    /// Выпуск, чей установщик скачан или качается.
    pub release: Option<Box<Release>>,
    /// Ответ проверки, которого ждём.
    checking: Option<Receiver<Result<Option<Box<Release>>, String>>>,
    /// Ход загрузки: числа идут потоком, итог — последним сообщением.
    loading: Option<Receiver<Progress>>,
}

/// Весточка от потока загрузки.
enum Progress {
    Step { done: u64, total: u64 },
    Done(PathBuf),
    Failed(String),
}

impl PithApp {
    /// Открывает окно обновления и сразу спрашивает GitHub.
    ///
    /// Спрашивает сразу потому, что за этим окно и открывают: заставлять
    /// человека нажимать «проверить» в окне, которое он открыл ради
    /// проверки, — лишний шаг.
    pub fn open_update(&mut self, ctx: &egui::Context) {
        self.update.open = true;

        if !matches!(self.update.stage, UpdateStage::Downloading { .. }) {
            self.check_update(ctx);
        }
    }

    pub fn close_update(&mut self) {
        self.update.open = false;
    }

    pub fn update_open(&self) -> bool {
        self.update.open
    }

    /// Что показывать в окне обновления.
    pub fn update_stage(&self) -> &UpdateStage {
        &self.update.stage
    }

    /// Спрашивает GitHub о последнем выпуске.
    pub fn check_update(&mut self, ctx: &egui::Context) {
        if matches!(self.update.stage, UpdateStage::Checking) {
            return;
        }

        let (sender, receiver) = channel();
        self.update.checking = Some(receiver);
        self.update.stage = UpdateStage::Checking;

        let ctx = ctx.clone();

        std::thread::spawn(move || {
            let answer = pith_update::check(crate::VERSION)
                .map(|release| release.map(Box::new))
                .map_err(|e| e.to_string());

            let _ = sender.send(answer);
            ctx.request_repaint();
        });
    }

    /// Проверка при запуске — тихая, без окна.
    ///
    /// Идёт один раз за запуск и только если её не выключили: плеер
    /// не должен ходить в сеть за спиной у того, кто этого не просил.
    /// Ответ виден строкой в углу, а не окном поверх кадра: человек сел
    /// смотреть кино, а не ставить обновления.
    pub(super) fn check_update_quietly(&mut self, ctx: &egui::Context) {
        if !self.settings.update_check || self.update_checked {
            return;
        }

        self.update_checked = true;
        self.check_update(ctx);
    }

    /// Забирает ответ проверки, если он готов.
    pub(super) fn poll_update(&mut self, ctx: &egui::Context) {
        self.poll_update_check();
        self.poll_update_download(ctx);
    }

    fn poll_update_check(&mut self) {
        let Some(receiver) = self.update.checking.as_ref() else {
            return;
        };
        let Ok(answer) = receiver.try_recv() else {
            return;
        };

        self.update.checking = None;

        match answer {
            Ok(Some(release)) => {
                // Окно закрыто — значит проверка была тихой, при запуске.
                // Скажем строкой в углу и не полезем поверх кадра.
                if !self.update.open {
                    let notice = crate::tr!(
                        format!("Вышла версия {} — меню, «Обновление…»", release.version),
                        format!("Version {} is out — menu, “Update…”", release.version)
                    );
                    self.show_notice(&notice);
                }

                self.update.stage = UpdateStage::Available(release);
            }
            Ok(None) => self.update.stage = UpdateStage::Latest,
            Err(why) => {
                tracing::warn!(причина = %why, "проверить обновление не вышло");
                self.update.stage = UpdateStage::Failed(why);
            }
        }
    }

    /// Скачивает установщик вышедшего выпуска.
    pub fn download_update(&mut self, ctx: &egui::Context) {
        let UpdateStage::Available(release) = &self.update.stage else {
            return;
        };

        let release = release.clone();
        let installer = release.installer.clone();
        let into = self.data_paths.updates();

        self.update.release = Some(release);
        self.update.stage = UpdateStage::Downloading {
            done: 0,
            total: installer.size,
        };

        let (sender, receiver) = channel();
        self.update.loading = Some(receiver);

        let ctx = ctx.clone();

        std::thread::spawn(move || {
            let steps = sender.clone();

            let answer = pith_update::download(&installer, &into, |done, total| {
                let _ = steps.send(Progress::Step { done, total });
            });

            let _ = sender.send(match answer {
                Ok(path) => Progress::Done(path),
                Err(e) => Progress::Failed(e.to_string()),
            });

            ctx.request_repaint();
        });
    }

    fn poll_update_download(&mut self, ctx: &egui::Context) {
        let Some(receiver) = self.update.loading.as_ref() else {
            return;
        };

        // Разом за кадр: чисел приходит много, а перерисовка одна.
        let steps: Vec<Progress> = std::iter::from_fn(|| receiver.try_recv().ok()).collect();

        if steps.is_empty() {
            return;
        }

        for step in steps {
            match step {
                Progress::Step { done, total } => {
                    self.update.stage = UpdateStage::Downloading { done, total };
                }
                Progress::Done(path) => {
                    self.update.loading = None;
                    self.update.stage = UpdateStage::Ready(path);
                }
                Progress::Failed(why) => {
                    tracing::warn!(причина = %why, "установщик не скачался");
                    self.update.loading = None;
                    self.update.stage = UpdateStage::Failed(why);
                }
            }
        }

        ctx.request_repaint();
    }

    /// Запускает скачанный установщик и закрывает плеер.
    ///
    /// Закрывать приходится самим: установщик заменяет запущенный файл,
    /// и Windows его не отдаст. Мастер умеет закрыть плеер и сам, но
    /// тогда человек увидит окно «программа занята» — а он уже нажал
    /// «установить», и спрашивать второй раз незачем.
    pub fn install_update(&mut self, ctx: &egui::Context) {
        let UpdateStage::Ready(path) = &self.update.stage else {
            return;
        };

        match std::process::Command::new(path).spawn() {
            Ok(_) => {
                tracing::info!(установщик = %path.display(), "запущен, плеер закрывается");
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
            Err(e) => {
                tracing::warn!(error = %e, "установщик не запустился");
                self.update.stage = UpdateStage::Failed(e.to_string());
            }
        }
    }

    /// Проверять ли обновления при запуске.
    pub fn update_check_enabled(&self) -> bool {
        self.settings.update_check
    }

    pub fn toggle_update_check(&mut self) {
        self.settings.update_check = !self.settings.update_check;
        self.save_settings();

        tracing::debug!(включено = self.settings.update_check, "проверка обновлений");
    }
}
