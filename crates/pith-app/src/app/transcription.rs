//! Транскрипция реплик перед выгрузкой.
//!
//! Работает в потоке выгрузки: словари медленные, а первый из них ещё
//! и требует паузы между словами. Порядок — тот же, что в готовой системе
//! пользователя, плюс второй словарь: **кэш → wooordhunt → cambridge**.
//!
//! Главная хитрость — считать слова, а не реплики. Слова в репликах
//! повторяются («you», «to», «the» встречаются в каждой второй), и каждое
//! спрашивается **один раз на всю выгрузку**. Найденное возвращается
//! приложению списком: файл кэша пишет оно, поток к нему не ходит.

use std::collections::BTreeMap;
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use pith_dict::{Dict, Source};
use pith_notion::{Row, Sound};
use pith_store::Sound as StoredSound;

use super::export_log::{LogKind, LogLine};

/// Сколько слов спрашивать разом.
///
/// Три: замер показал, что сайт столько терпит без единого отказа,
/// а больше упёрлось бы уже в прокси.
const WORKERS: usize = 3;

/// Что дала транскрипция.
pub(super) struct Transcribed {
    /// Слова, которых в кэше не было: их предстоит запомнить.
    pub(super) fresh: Vec<(String, StoredSound)>,
    /// Сколько слов так и не нашлось ни в одном словаре.
    pub(super) missing: usize,
}

/// Заполняет транскрипции строк, сообщая о ходе работы.
///
/// `known` — снимок хранилища приложения. `step` зовётся после каждого
/// **нового** слова: слова из памяти мгновенны, и отмерять их незачем.
/// Вторым делом `step` получает строку журнала — по ней в окне видно,
/// откуда взялось значение: из памяти, с первого сайта или со второго.
pub(super) fn transcribe(
    rows: &mut [Row],
    known: BTreeMap<String, StoredSound>,
    // `Send`: слова спрашиваются в несколько потоков, и о готовом
    // сообщает тот поток, который его дописал.
    step: impl FnMut(usize, usize, LogLine) + Send,
) -> Transcribed {
    let wanted = unique_words(rows);
    let unknown: Vec<(String, String)> = wanted
        .iter()
        .filter(|(key, _)| !known.contains_key(key))
        .cloned()
        .collect();

    let from_memory = wanted.len() - unknown.len();

    tracing::info!(
        слов = wanted.len(),
        из_памяти = from_memory,
        спросить = unknown.len(),
        "транскрипция реплик"
    );

    // Здесь человек ждёт, поэтому спрашиваем в несколько потоков и без
    // выдержки между запросами. Разогрев во время просмотра (`warmup.rs`)
    // устроен наоборот — один поток и пауза: там никто не ждёт, и вежливость
    // к чужому сайту важнее скорости.
    //
    // Замер на живых страницах, слова каждый раз свежие: 24 слова одним
    // потоком с паузой — 7,0 с, тремя потоками без паузы — **2,7 с**, и все
    // 24 нашлись в первом словаре: ни одного отказа.
    let dict = Dict::with_gap(Duration::ZERO);
    let total = unknown.len();

    let mut step = step;

    step(
        0,
        total,
        LogLine::new(
            LogKind::Memory,
            crate::tr!(
                format!(
                    "Слов в репликах {} · из памяти {from_memory} · спросить {total}",
                    wanted.len()
                ),
                format!(
                    "Words in lines {} · from memory {from_memory} · to ask {total}",
                    wanted.len()
                )
            ),
        ),
    );

    let next = AtomicUsize::new(0);
    let ready = AtomicUsize::new(0);
    let missing = AtomicUsize::new(0);
    let found = Mutex::new(known);
    let fresh = Mutex::new(Vec::new());
    let step = Mutex::new(step);

    std::thread::scope(|scope| {
        for _ in 0..WORKERS.min(total.max(1)) {
            scope.spawn(|| {
                // Каждый поток берёт следующее свободное слово, а не свою
                // долю списка: слова разной цены — известное сайту находится
                // за четверть секунды, незнакомое уходит во второй словарь.
                loop {
                    let at = next.fetch_add(1, Ordering::Relaxed);
                    let Some((key, word)) = unknown.get(at) else {
                        break;
                    };

                    let line = ask(&dict, key, word, &found, &fresh, &missing);
                    let done = ready.fetch_add(1, Ordering::Relaxed) + 1;

                    if let Ok(mut step) = step.lock() {
                        step(done, total, line);
                    }
                }
            });
        }
    });

    let found = found.into_inner().unwrap_or_default();

    for row in rows {
        row.sounds = sounds_of(&row.text, &found);
    }

    Transcribed {
        fresh: fresh.into_inner().unwrap_or_default(),
        missing: missing.load(Ordering::Relaxed),
    }
}

/// Спрашивает словари об одном слове и кладёт найденное в общий свод.
fn ask(
    dict: &Dict,
    key: &str,
    word: &str,
    found: &Mutex<BTreeMap<String, StoredSound>>,
    fresh: &Mutex<Vec<(String, StoredSound)>>,
    missing: &AtomicUsize,
) -> LogLine {
    let started = Instant::now();
    let spent = || format!("{:.1} с", started.elapsed().as_secs_f32());

    let Some(result) = dict.lookup(word) else {
        missing.fetch_add(1, Ordering::Relaxed);

        return LogLine::new(
            LogKind::Missing,
            crate::tr!(
                format!("{key} — нет ни в одном словаре   {}", spent()),
                format!("{key} — in no dictionary   {}", spent())
            ),
        );
    };

    let note = format!("{key} → {}   {}", result.transcription, spent());
    let kind = match result.source {
        Source::Wooordhunt => LogKind::First,
        Source::Cambridge => LogKind::Second,
    };

    let sound = StoredSound {
        transcription: result.transcription,
        url: result.url,
    };

    if let Ok(mut fresh) = fresh.lock() {
        fresh.push((key.to_string(), sound.clone()));
    }
    if let Ok(mut found) = found.lock() {
        found.insert(key.to_string(), sound);
    }

    LogLine::new(kind, note)
}

/// Все слова всех реплик — по одному разу, в порядке появления.
///
/// Отдаётся пара «ключ и слово как в реплике». Ключ — для памяти и первого
/// словаря, само слово — для второго: он зовёт сокращения через дефис,
/// а из ключа апостроф уже убран, и `haven't` было не восстановить.
///
/// Порядок держим, чтобы спрашивать словарь в том же виде, в каком читается
/// список: так по ходу работы понятно, где она идёт.
fn unique_words(rows: &[Row]) -> Vec<(String, String)> {
    let mut words: Vec<(String, String)> = Vec::new();

    for row in rows {
        for word in pith_dict::split(&row.text) {
            let key = pith_dict::key(&word);

            if !key.is_empty() && !words.iter().any(|(known, _)| *known == key) {
                words.push((key, word));
            }
        }
    }

    words
}

/// Транскрипция реплики: по записи на слово, в порядке слов.
///
/// Слово, которого не знает ни один словарь, пропускается — в поле Notion
/// оно просто не появится. Ставить на его место заглушку незачем: читателю
/// нужна транскрипция, а не отметка о неудаче.
fn sounds_of(text: &str, found: &BTreeMap<String, StoredSound>) -> Vec<Sound> {
    pith_dict::split(text)
        .iter()
        .filter_map(|word| found.get(&pith_dict::key(word)))
        .map(|sound| Sound {
            transcription: sound.transcription.clone(),
            url: sound.url.clone(),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{sounds_of, unique_words};
    use pith_notion::Row;
    use pith_store::Sound as StoredSound;
    use std::collections::BTreeMap;

    fn строка(text: &str) -> Row {
        Row {
            number: 1,
            text: text.into(),
            actor: None,
            sounds: Vec::new(),
        }
    }

    /// Только ключи — слова в паре нужны второму словарю, не проверке.
    fn keys(words: &[(String, String)]) -> Vec<String> {
        words.iter().map(|(key, _)| key.clone()).collect()
    }

    fn знание(pairs: &[(&str, &str)]) -> BTreeMap<String, StoredSound> {
        pairs
            .iter()
            .map(|(key, transcription)| {
                (
                    (*key).to_string(),
                    StoredSound {
                        transcription: (*transcription).to_string(),
                        url: format!("https://wooordhunt.ru/word/{key}"),
                    },
                )
            })
            .collect()
    }

    #[test]
    fn слово_спрашивается_один_раз_на_выгрузку() {
        let rows = [строка("You know you can"), строка("You can")];

        assert_eq!(keys(&unique_words(&rows)), ["you", "know", "can"]);
    }

    #[test]
    fn регистр_и_апостроф_не_плодят_слов() {
        let rows = [строка("It's it ITS")];

        assert_eq!(keys(&unique_words(&rows)), ["its", "it"]);
    }

    #[test]
    fn транскрипция_идёт_в_порядке_слов() {
        let found = знание(&[("you", "|juː|"), ("can", "|kən|")]);
        let sounds = sounds_of("Can you", &found);

        assert_eq!(sounds.len(), 2);
        assert_eq!(sounds[0].transcription, "|kən|", "порядок слов реплики");
        assert_eq!(sounds[1].transcription, "|juː|");
    }

    #[test]
    fn ненайденное_слово_пропускается() {
        let found = знание(&[("you", "|juː|")]);
        let sounds = sounds_of("Where you", &found);

        assert_eq!(sounds.len(), 1, "у «where» транскрипции нет — и записи нет");
        assert_eq!(sounds[0].transcription, "|juː|");
    }

    #[test]
    fn у_реплики_без_английских_слов_транскрипции_нет() {
        let found = знание(&[("you", "|juː|")]);

        assert!(sounds_of("Привет, 42!", &found).is_empty());
    }
}
