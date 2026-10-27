//! Перевод реплики субтитров через DeepL.
//!
//! Значок рядом с репликой (`ui/subtitle_translate.rs`) вызывает
//! `translate_line`; сеть идёт в отдельном потоке, готовый перевод
//! показывается под репликой, пока она не сменится.

use std::collections::HashMap;
use std::sync::mpsc::{Receiver, channel};

use pith_deepl::Deepl;
use pith_store::DeeplSettings;

use super::{AccessStatus, PithApp};

/// Перевод одной реплики: по строке на язык либо причина отказа.
pub type Translated = Result<Vec<(String, String)>, String>;

/// Что показано под репликой.
pub struct Shown {
    /// Какая реплика переведена (или переводится).
    pub text: String,
    /// `None` — ответ ещё идёт.
    pub result: Option<Translated>,
}

#[derive(Default)]
pub struct DeeplState {
    pub settings: DeeplSettings,
    /// Уже переведённое: (реплика, язык) → перевод. Повторный щелчок бесплатен.
    cache: HashMap<(String, String), String>,
    waiting: Option<Receiver<Translated>>,
    pub shown: Option<Shown>,
    /// Итог проверки ключа в окне интеграций.
    pub check: Option<AccessStatus>,
    check_rx: Option<Receiver<Result<String, String>>>,
}

impl DeeplState {
    pub fn new(settings: DeeplSettings) -> Self {
        Self {
            settings,
            ..Self::default()
        }
    }
}

impl PithApp {
    pub fn deepl(&mut self) -> &mut DeeplState {
        &mut self.deepl
    }

    /// Записывает настройки DeepL.
    pub fn save_deepl(&self) {
        self.deepl.settings.save(&self.data_paths);
    }

    /// Прячет перевод.
    pub fn hide_translation(&mut self) {
        self.deepl.shown = None;
        self.deepl.waiting = None;
    }

    /// Переводит реплику. Нет токена — подсказывает, где его указать.
    pub fn translate_line(&mut self, ctx: &egui::Context, line: &str) {
        let state = &mut self.deepl;

        if !state.settings.is_ready() {
            self.show_notice(crate::tr!(
                "Укажите токен DeepL в окне «Интеграции»",
                "Set the DeepL token in the Integrations window"
            ));
            self.open_integrations();
            return;
        }

        let text = line.trim().to_string();
        let targets = state.settings.targets.clone();

        // Всё уже переведено — берём из памяти, сеть не нужна.
        let cached: Vec<(String, String)> = targets
            .iter()
            .filter_map(|lang| {
                state
                    .cache
                    .get(&(text.clone(), lang.clone()))
                    .map(|t| (lang.clone(), t.clone()))
            })
            .collect();

        if cached.len() == targets.len() {
            state.shown = Some(Shown {
                text,
                result: Some(Ok(cached)),
            });
            return;
        }

        let Some(client) = Deepl::new(&state.settings.token) else {
            return;
        };

        state.shown = Some(Shown {
            text: text.clone(),
            result: None,
        });

        let (sender, receiver) = channel();
        state.waiting = Some(receiver);
        let ctx = ctx.clone();

        std::thread::spawn(move || {
            let mut lines = Vec::new();

            for lang in &targets {
                match client.translate(&text, lang) {
                    Ok(translated) => lines.push((lang.clone(), translated)),
                    Err(e) => {
                        let _ = sender.send(Err(e.to_string()));
                        ctx.request_repaint();
                        return;
                    }
                }
            }

            let _ = sender.send(Ok(lines));
            ctx.request_repaint();
        });
    }

    /// Проверка ключа по кнопке в окне интеграций: только чтение.
    pub fn check_deepl(&mut self, ctx: &egui::Context) {
        let token = self.deepl.settings.token.clone();
        let (sender, receiver) = channel();

        self.deepl.check = Some(AccessStatus::Working);
        self.deepl.check_rx = Some(receiver);
        let ctx = ctx.clone();

        std::thread::spawn(move || {
            let answer = match Deepl::new(&token) {
                None => Err(
                    crate::tr!("Не задан токен DeepL", "The DeepL token is missing").to_string(),
                ),
                Some(client) => client
                    .usage()
                    .map(|u| {
                        crate::tr!(
                            format!(
                                "Ключ работает. Использовано символов: {} из {}",
                                u.used, u.limit
                            ),
                            format!("The key works. Characters used: {} of {}", u.used, u.limit)
                        )
                    })
                    .map_err(|e| e.to_string()),
            };

            let _ = sender.send(answer);
            ctx.request_repaint();
        });
    }

    /// Забирает готовый перевод и итог проверки ключа.
    pub(super) fn poll_deepl(&mut self) {
        if let Some(receiver) = self.deepl.waiting.as_ref()
            && let Ok(answer) = receiver.try_recv()
        {
            self.deepl.waiting = None;

            if let (Ok(lines), Some(shown)) = (&answer, self.deepl.shown.as_ref()) {
                let text = shown.text.clone();
                for (lang, translated) in lines {
                    self.deepl
                        .cache
                        .insert((text.clone(), lang.clone()), translated.clone());
                }
            }

            if let Some(shown) = self.deepl.shown.as_mut() {
                shown.result = Some(answer);
            }
        }

        if let Some(receiver) = self.deepl.check_rx.as_ref()
            && let Ok(answer) = receiver.try_recv()
        {
            self.deepl.check_rx = None;
            self.deepl.check = Some(match answer {
                Ok(info) => AccessStatus::Verified(info),
                Err(why) => AccessStatus::Failed(why),
            });
        }
    }
}
