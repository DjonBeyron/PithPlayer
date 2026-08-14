//! Разогрев словарной памяти во время просмотра.
//!
//! Самая долгая часть выгрузки — спросить словари о новых словах: около
//! трети секунды на слово, и при первой выгрузке фильма их набирается
//! сотня. Но тексты закладок известны задолго до выгрузки — в тот миг,
//! когда закладку ставят. Значит слова можно спрашивать тогда же, по одному,
//! пока человек смотрит кино, и к выгрузке спрашивать будет нечего.
//!
//! Это не ускорение, а перенос: та же работа делается тогда, когда её
//! никто не ждёт.
//!
//! **Плеера это не касается.** Поток занят ожиданием сети, а не счётом;
//! интерфейс он не будит — найденное подождёт ближайшего кадра, спешить
//! некуда. Запись в файл идёт не чаще раза за кадр, сколько бы слов
//! ни подоспело.
//!
//! **Конфликтов с закладками быть не может по устройству.** Отсюда только
//! читают тексты и только пишут в словарную память. Закладки не трогаются
//! вовсе: удалили закладку посреди разогрева — плеер узнает транскрипцию
//! слова, которое сейчас не нужно, и оно останется в памяти до другого
//! фильма.

use std::collections::HashSet;
use std::sync::mpsc::{Receiver, Sender, channel};

use pith_dict::Dict;
use pith_store::Sound;

use super::PithApp;

/// Фоновый разогрев словарной памяти.
#[derive(Default)]
pub(super) struct SoundWarmup {
    /// Слова, уже отданные потоку: второй раз не спрашиваем.
    ///
    /// Помнить нужно и ненайденные: их нет в памяти слов, и без этого
    /// списка они спрашивались бы на каждой новой закладке заново.
    asked: HashSet<String>,
    /// Очередь к потоку. `None` — поток ещё не заводили.
    queue: Option<Sender<Word>>,
    found: Option<Receiver<(String, Sound)>>,
}

/// Слово в очереди: ключ для памяти и вид, в каком его знает словарь.
struct Word {
    key: String,
    text: String,
}

impl PithApp {
    /// Ставит слова реплики в очередь разогрева.
    ///
    /// Ничего не делает, когда транскрипция выключена: лезть в сеть за тем,
    /// что не понадобится, незачем.
    pub(super) fn warm_words(&mut self, text: &str) {
        if !self.settings.export_transcribe || text.trim().is_empty() {
            return;
        }

        let words: Vec<Word> = pith_dict::split(text)
            .iter()
            .map(|word| Word {
                key: pith_dict::key(word),
                text: word.clone(),
            })
            .filter(|word| self.worth_asking(&word.key))
            .collect();

        if words.is_empty() {
            return;
        }

        tracing::debug!(слов = words.len(), "в очередь разогрева");

        let queue = self.warmup_queue();

        for word in words {
            self.warmup.asked.insert(word.key.clone());

            // Поток кончился — очередь мертва, разогрева в этот раз не будет.
            if queue.send(word).is_err() {
                self.warmup.queue = None;
                return;
            }
        }
    }

    /// Ставит в очередь слова всех закладок открытого видео.
    ///
    /// Нужно для списков, набранных до появления разогрева: они прогреются
    /// за просмотр, а не в миг выгрузки.
    pub(super) fn warm_current_bookmarks(&mut self) {
        let Some(video) = self.current_bookmarks() else {
            return;
        };

        let lines: Vec<String> = video
            .lists
            .iter()
            .flat_map(|list| list.bookmarks.iter())
            .filter_map(|bookmark| bookmark.name.clone())
            .collect();

        for line in lines {
            self.warm_words(&line);
        }
    }

    /// Забирает найденное и кладёт в память.
    ///
    /// Разом за кадр, сколько бы слов ни подоспело: файл пишется один раз.
    pub(super) fn poll_warmup(&mut self) {
        let Some(found) = self.warmup.found.as_ref() else {
            return;
        };

        let words: Vec<(String, Sound)> = std::iter::from_fn(|| found.try_recv().ok()).collect();

        if words.is_empty() {
            return;
        }

        self.sounds.remember(words);
    }

    /// Стоит ли спрашивать это слово.
    fn worth_asking(&self, key: &str) -> bool {
        !key.is_empty() && self.sounds.get(key).is_none() && !self.warmup.asked.contains(key)
    }

    /// Очередь к потоку разогрева, заводя его при первом обращении.
    fn warmup_queue(&mut self) -> Sender<Word> {
        if let Some(queue) = self.warmup.queue.clone() {
            return queue;
        }

        let (queue, orders) = channel::<Word>();
        let (sender, found) = channel();

        self.warmup.queue = Some(queue.clone());
        self.warmup.found = Some(found);

        std::thread::spawn(move || {
            // Словари заводятся один раз на поток: у них своя выдержка
            // между запросами, и с новым словарём она бы обнулялась.
            let dict = Dict::new();

            while let Ok(word) = orders.recv() {
                let Some(result) = dict.lookup(&word.text) else {
                    tracing::debug!(слово = %word.key, "разогрев: транскрипции нет");
                    continue;
                };

                let sound = Sound {
                    transcription: result.transcription,
                    url: result.url,
                };

                // Интерфейс не будим: слово подождёт ближайшего кадра.
                if sender.send((word.key, sound)).is_err() {
                    break;
                }
            }

            tracing::debug!("поток разогрева окончен");
        });

        queue
    }
}
