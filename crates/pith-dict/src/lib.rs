//! Транскрипция английских слов из словарей.
//!
//! Порядок такой: **кэш → wooordhunt.ru → dictionary.cambridge.org**.
//! Кэш держит приложение (`pith-store`), потому что это файл данных
//! пользователя; крейт знает только про сайты — запросить и разобрать.
//!
//! Логика первого словаря перенесена из готовой системы пользователя
//! (`C:\PITH\Development\NOTION_PITH`, документ `TRANSCRIPTION_LOGIC.md`):
//! три стратегии разбора, повторы с паузами и пауза между словами. Второй
//! словарь добавлен по той же мерке — на случай, когда первый слова не знает.

mod cambridge;
mod net;
mod wooordhunt;
mod words;

use std::sync::Mutex;
use std::time::{Duration, Instant};

pub use net::use_proxy;
pub use words::{key, split};

use words::is_apostrophe;

/// Сколько ждать ответа сайта.
const TIMEOUT: Duration = Duration::from_secs(10);

// Повторов с паузами у первого словаря больше нет — и вот почему.
//
// В готовой системе пользователя слово, которого сайт не отдал, спрашивалось
// четырежды с паузами 5, 8 и 8 секунд: так обходили ограничитель частоты.
// Ограничитель, однако, срабатывал на **новых соединениях**, а не на частоте
// (см. `GAP`), и с общим клиентом не появляется вовсе — двадцать девять слов
// подряд прошли без единого отказа.
//
// Зато цена этих повторов оказалась велика: сайт отвечает страницей без
// транскрипции не только под нагрузкой, но и на сокращения вроде `havent`.
// Каждое такое слово стоило 21 секунду ожидания на пустом месте — при двух
// закладках выгрузка вставала на минуту. Теперь по одному запросу в каждый
// словарь, и хватит; слово без транскрипции в память не попадает,
// так что следующая выгрузка попробует его заново.

/// Наименьший промежуток между запросами к первому словарю.
///
/// В готовой системе пользователя пауза была 1200 мс, и без неё сайт
/// отвечал заглушками. Причина оказалась не в частоте запросов, а в том,
/// что тот клиент открывал **новое соединение на каждое слово** — сайт
/// считал именно их. Мы держим одно (см. `net.rs`), и ограничитель
/// не срабатывает вовсе.
///
/// Замеры на живых страницах, слова каждый раз свежие:
///
/// | пауза | слов | всего | на слово |
/// |------:|-----:|------:|---------:|
/// | 1200 мс | 9 | 10,1 с | 1,12 с |
/// | 300 мс | 9 | 3,9 с | 0,43 с |
/// | без паузы | 29 | 8,6 с | 0,30 с |
///
/// Ни в одном прогоне сайт не отказал. Совсем убирать паузу всё же
/// не стали: сайт чужой и бесплатный, а 200 мс дают около двух запросов
/// в секунду — вежливо и почти бесплатно. Сработает ограничитель — плеер
/// уйдёт ко второму словарю, это уже предусмотрено.
const GAP: Duration = Duration::from_millis(200);

/// Ответ «слова не знаю».
const NOT_FOUND: u16 = 404;

/// Ответ «страница есть».
const OK: u16 = 200;

/// Что ответил словарь на один запрос.
enum Answer {
    Found(Found),
    /// Слова не знает — повторять запрос незачем.
    Unknown,
    /// Не ответил: завис под ограничителем частоты или отдал заглушку.
    Silent,
}

/// Где нашлась транскрипция.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
    Wooordhunt,
    Cambridge,
}

/// Найденная транскрипция.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Found {
    /// Транскрипция вместе с обрамляющими чертами: `|wer|`.
    pub transcription: String,
    /// Адрес страницы слова — на него ведёт ссылка в Notion.
    pub url: String,
    pub source: Source,
}

/// Доступ к словарям.
pub struct Dict {
    /// Когда последний раз спрашивали первый словарь.
    ///
    /// По нему выдерживается пауза между словами: сайт этого требует,
    /// а лишний раз ждать незачем — соседние слова часто берутся из кэша,
    /// и к сети дело не доходит вовсе.
    asked: Mutex<Option<Instant>>,
    /// Наименьший промежуток между запросами к первому словарю.
    gap: Duration,
}

impl Default for Dict {
    fn default() -> Self {
        Self {
            asked: Mutex::new(None),
            gap: GAP,
        }
    }
}

impl Dict {
    pub fn new() -> Self {
        Self::default()
    }

    /// Словари с иной паузой между запросами к первому сайту.
    ///
    /// Нужно замерам: пауза — главная цена транскрипции, и подбирать её
    /// приходится опытом, а не наугад.
    pub fn with_gap(gap: Duration) -> Self {
        Self {
            gap,
            ..Self::default()
        }
    }

    /// Транскрипция слова. `None` — не знает ни один словарь.
    ///
    /// Слово принимается в любом виде: ключ из него делается сам.
    ///
    /// Порядок обхода не просто «первый, потом второй». Первый словарь
    /// ограничивает частоту запросов, и под ограничителем он отвечает
    /// страницей-заглушкой — по ней не отличить занятый сервер от слова,
    /// которого нет. Ждать четыре попытки с паузами (это двадцать секунд)
    /// в такой миг незачем: второй словарь отвечает меньше чем за секунду
    /// и даёт ту же американскую транскрипцию. Поэтому:
    ///
    /// 1. один запрос к первому словарю;
    /// 2. не ответил или не знает — второй словарь;
    /// 3. и второй не знает — возвращаемся к первому с паузами.
    ///
    /// Замер: слово `selfie` под ограничителем стоило 24 секунды, а этим
    /// порядком — полторы.
    pub fn lookup(&self, word: &str) -> Option<Found> {
        let key = words::key(word);

        if key.is_empty() {
            return None;
        }

        // У сокращения апостроф несёт смысл, и снимать его нельзя:
        // `we're` превращается в `were`, `won't` в `wont`, `can't` в `cant` —
        // это другие слова с другим произношением, и первый словарь честно
        // отдаёт их транскрипцию. Второй словарь знает точную форму
        // (`we-re`), поэтому у сокращений начинаем с него.
        if word.chars().any(is_apostrophe) {
            if let Some(found) = self.ask_cambridge(&words::dashed(word)) {
                return Some(found);
            }

            tracing::debug!(слово = %word, "второй словарь сокращения не знает");
        }

        match self.ask_wooordhunt(&key) {
            Answer::Found(found) => return Some(found),
            Answer::Unknown => tracing::debug!(слово = %key, "первый словарь слова не знает"),
            Answer::Silent => tracing::debug!(слово = %key, "первый словарь молчит"),
        }

        // Обычное слово: второй словарь спрашиваем следом за первым.
        if !word.chars().any(is_apostrophe)
            && let Some(found) = self.ask_cambridge(&words::dashed(word))
        {
            return Some(found);
        }

        tracing::warn!(слово = %key, "транскрипции нет ни в одном словаре");
        None
    }

    /// Один запрос к первому словарю.
    fn ask_wooordhunt(&self, key: &str) -> Answer {
        let url = wooordhunt::url(key);

        self.wait_turn();

        let Some((status, html)) = fetch(&url) else {
            // Ответа нет вовсе — сервер завис под ограничителем частоты.
            return Answer::Silent;
        };

        if status == NOT_FOUND {
            return Answer::Unknown;
        }

        // Страница без транскрипций при исправном коде — заглушка
        // ограничителя частоты, а не слово без транскрипции.
        if !wooordhunt::looks_real(&html) {
            return Answer::Silent;
        }

        match wooordhunt::parse(&html) {
            Some(transcription) => Answer::Found(Found {
                transcription,
                url,
                source: Source::Wooordhunt,
            }),
            None => Answer::Unknown,
        }
    }

    /// Второй словарь. Ограничителя частоты у него не замечено, повторов нет.
    fn ask_cambridge(&self, key: &str) -> Option<Found> {
        let url = cambridge::url(key);
        let (status, html) = fetch(&url)?;

        if status != OK {
            tracing::debug!(слово = %key, status, "второй словарь слова не знает");
            return None;
        }

        cambridge::parse(&html).map(|transcription| Found {
            transcription,
            url,
            source: Source::Cambridge,
        })
    }

    /// Выдерживает паузу между запросами к первому словарю.
    fn wait_turn(&self) {
        let Ok(mut asked) = self.asked.lock() else {
            return;
        };

        if let Some(last) = *asked {
            let passed = last.elapsed();

            if passed < self.gap {
                std::thread::sleep(self.gap - passed);
            }
        }

        *asked = Some(Instant::now());
    }
}

/// Забирает страницу: код состояния и тело.
///
/// `None` — ответа не было вовсе: сервер завис или сеть не дала пройти.
/// Код отдаётся наружу, потому что `404` у первого словаря означает
/// «слова не знаю», и повторять такой запрос незачем.
fn fetch(url: &str) -> Option<(u16, String)> {
    // Обычный браузерный заголовок: без него сайты отвечают отказом.
    const AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) \
                         AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0 Safari/537.36";

    let answer = net::agent(TIMEOUT)
        .get(url)
        .header("User-Agent", AGENT)
        .header("Accept-Language", "ru-RU,ru;q=0.9,en;q=0.8")
        .call();

    match answer {
        Ok(response) => {
            let status = response.status().as_u16();
            let body = response
                .into_body()
                .read_to_string()
                .inspect_err(|e| tracing::debug!(%url, error = %e, "страницу не прочитать"))
                .ok()?;

            Some((status, body))
        }
        Err(e) => {
            tracing::debug!(%url, error = %e, "страница не отвечает");
            None
        }
    }
}
