//! Выгрузка отрезков в Notion.
//!
//! База в Notion одна на все картины, и делает её пользователь сам —
//! кнопкой «Duplicate» с образца `DIFF`. Так копия сохраняет виды,
//! фильтры и оформление: API Notion копировать не умеет (ни страницу,
//! ни базу, ни блок), а собранная по свойствам база всего этого лишена.
//!
//! Плеер в готовую базу только складывает строки. Картины различаются
//! полем `FILM NAME`, поэтому карточку на картину заводить не нужно.
//!
//! Крейт знает только про Notion. Ни окон, ни закладок — этим занимается
//! приложение (PLAN.md §12.4).

mod client;
mod defaults;
mod error;
mod id;
mod net;
mod row;
mod writer;

use serde_json::{Map, Value};

pub use client::Notion;
pub use error::{NotionError, Result};
pub use id::{dashed, parse as parse_id};
pub use net::use_proxy;
pub use row::{Row, Sound};

/// Всё, что нужно знать до первой строки.
///
/// Узнаётся тремя запросами и **до** нажатия «Выгрузить»: пока человек
/// выбирает название и вид картины, плеер уже спрашивает Notion. Три
/// запроса — это три секунды ожидания, и незачем показывать их тому,
/// кто и так занят вопросом.
#[derive(Debug, Clone)]
pub struct Prepared {
    /// Номер рабочей базы.
    pub database: String,
    /// Заготовка строки — значения строки образца.
    pub sample: Map<String, Value>,
    /// Последний занятый номер: счёт продолжится с него.
    pub base: usize,
    /// В базе есть числовое поле номера, и в него можно писать.
    ///
    /// От него зависит и сортировка вида, и право писать строки в несколько
    /// потоков: без поля порядок держится только на порядке создания.
    pub numbered: bool,
}

impl Prepared {
    /// Взяли ли заготовку у образца.
    pub fn has_sample(&self) -> bool {
        !self.sample.is_empty()
    }
}

/// То, что у Notion от выгрузки к выгрузке не меняется.
///
/// Узнать это стоит четырёх запросов — около трёх секунд, — а меняется
/// оно только вместе со ссылками на страницы. Поэтому приложение держит
/// это у себя и спрашивает заново лишь когда ссылки другие.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Stable {
    /// Номер рабочей базы.
    pub database: String,
    /// Заготовка строки — значения строки образца.
    pub sample: Map<String, Value>,
    /// В базе есть числовое поле номера.
    pub numbered: bool,
}

/// Узнаёт постоянную часть: базу, заготовку строки и поле номера.
///
/// Четыре запроса: база в странице, поле номера, база образца и его строка.
/// Заготовку достаём как получится — не вышло, строки создадутся без
/// наследованных значений, о чём скажет отчёт. А вот без номера базы
/// выгружать некуда, и это уже отказ.
pub fn discover(notion: &Notion, work_page: &str, template_page: &str) -> Result<Stable> {
    let database = notion.database_in_page(work_page)?;

    tracing::info!(база = %database, "рабочая база найдена");

    Ok(Stable {
        sample: row_sample(notion, template_page),
        numbered: numbering_field(notion, &database),
        database,
    })
}

/// Достраивает подготовку свежим последним номером.
///
/// Единственное, что приходится спрашивать каждый раз: строки в базе
/// прибавляются, и счёт должен продолжаться с последней.
pub fn prepare_from(notion: &Notion, stable: Stable) -> Prepared {
    Prepared {
        base: numbering_base(notion, &stable.database),
        database: stable.database,
        sample: stable.sample,
        numbered: stable.numbered,
    }
}

/// Узнаёт всё нужное для выгрузки — пятью запросами, без памяти.
pub fn prepare(notion: &Notion, work_page: &str, template_page: &str) -> Result<Prepared> {
    let stable = discover(notion, work_page, template_page)?;

    Ok(prepare_from(notion, stable))
}

/// Заводит в базе числовое поле номера, если его там нет.
///
/// Без него строки уйдут — только без номера-числа, и вид сортировать
/// будет не по чему. Поэтому неудача не отказ, а запись в отчёте.
fn numbering_field(notion: &Notion, database: &str) -> bool {
    match notion.ensure_number_property(database, row::NUMBER) {
        Ok(ready) => ready,
        Err(e) => {
            tracing::warn!(error = %e, "поле номера не завести — строки пойдут без него");
            false
        }
    }
}

/// Что получилось.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Report {
    /// Номер базы — по нему её открывают в браузере.
    pub database_id: String,
    /// Сколько строк создано.
    pub created: usize,
    /// Сколько строк ушло без актёра.
    pub without_actor: usize,
    /// Строки, которые Notion не принял: номер и причина.
    pub failed: Vec<(usize, String)>,
    /// Заготовку строки удалось взять у образца.
    ///
    /// Ложь означает, что строки ушли без наследованных значений —
    /// без `STATUS` в том числе, а на него смотрит синхронизатор.
    pub sample_taken: bool,
    /// Номер первой созданной строки.
    ///
    /// Нумерация в базе сквозная, и по этому номеру видно, куда именно
    /// легли отрезки: база одна на все картины.
    pub first_number: usize,
}

/// Выгружает отрезки, сообщая о ходе работы.
///
/// `progress` вызывается после каждой строки: сколько сделано из скольких.
/// Отдельная строка, которую Notion не принял, работу не прерывает —
/// её причина попадает в отчёт. Отказа целиком здесь быть не может:
/// всё, что могло не получиться, случилось в `prepare`.
pub fn export(
    notion: &Notion,
    prepared: &Prepared,
    film_name: &str,
    rows: &[Row],
    progress: impl FnMut(usize, usize) + Send,
) -> Report {
    tracing::info!(
        база = %prepared.database,
        картина = %film_name,
        строк = rows.len(),
        по_номеру = prepared.numbered,
        "выгружаю"
    );

    let without_actor = rows
        .iter()
        .filter(|row| row.actor.as_deref().unwrap_or("").trim().is_empty())
        .count();

    // Порядок строк в базе задаётся порядком создания, но держится на нём
    // только пока номер живёт в текстовом заголовке. С числовым полем `NUM`
    // вид сортируется по номеру, и порядок создания перестаёт значить
    // что-либо — потому строки и пишутся в несколько потоков (`writer.rs`).
    let (created, failed) = writer::write_rows(notion, prepared, film_name, rows, progress);

    Report {
        database_id: prepared.database.clone(),
        created,
        without_actor,
        failed,
        sample_taken: prepared.has_sample(),
        first_number: prepared.base + 1,
    }
}

/// С какого номера продолжать счёт.
///
/// База одна на все картины, и заголовки строк в ней сквозные: новая
/// выгрузка продолжает нумерацию, а не начинает её заново. Иначе в базе
/// оказывается по нескольку строк с заголовком «1». Одинаковые реплики
/// при этом в порядке вещей — совпадать не должны только номера.
///
/// Не вышло спросить — начинаем с нуля: строки важнее их номеров,
/// а совпавший номер ничего не ломает.
fn numbering_base(notion: &Notion, database: &str) -> usize {
    match notion.max_number(database) {
        Ok(largest) => {
            tracing::info!(последний = largest, "нумерация продолжится с этого");
            largest
        }
        Err(e) => {
            tracing::warn!(error = %e, "номера в базе не прочитать, счёт с единицы");
            0
        }
    }
}

/// Заготовка строки — значения единственной строки образца.
///
/// Копии строки в API нет, поэтому «дублировать первую строку» выходит
/// так: её значения читаются у образца и ложатся в основание каждой новой.
/// Без этого у строк пустой `STATUS`, а обратная синхронизация смотрит
/// именно на него.
///
/// Образец берётся отдельной страницей, а не первой строкой рабочей базы:
/// та наполняется отрезками, и «первая строка» в ней — что придётся.
///
/// Не вышло — не беда: строки создадутся без заготовки, о чём скажет отчёт.
fn row_sample(notion: &Notion, template_page: &str) -> Map<String, Value> {
    let read = notion
        .database_in_page(template_page)
        .and_then(|database| notion.first_row(&database));

    match read {
        Ok(Some(row)) => {
            let sample = defaults::from_row(&row);
            tracing::info!(полей = sample.len(), "заготовка строки взята у образца");
            sample
        }
        Ok(None) => {
            tracing::warn!("в образце нет строк — заготовки не будет");
            Map::new()
        }
        Err(e) => {
            tracing::warn!(error = %e, "строку образца не прочитать");
            Map::new()
        }
    }
}

/// Название картины с её видом: «Фильм: Титаник».
///
/// Пустое название заменяется на «Без названия»: строка идёт в поле
/// каждой строки базы, и пустой она быть не должна.
pub fn film_name(kind: Kind, title: &str) -> String {
    let title = title.trim();

    let title = if title.is_empty() {
        "Без названия"
    } else {
        title
    };

    format!("{}: {title}", kind.as_str())
}

/// Фильм или сериал — выбор пользователя перед выгрузкой.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Series,
    Movie,
}

impl Kind {
    fn as_str(self) -> &'static str {
        match self {
            Self::Series => "Сериал",
            Self::Movie => "Фильм",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Kind, film_name};

    #[test]
    fn вид_и_название_склеиваются() {
        assert_eq!(film_name(Kind::Movie, "Титаник"), "Фильм: Титаник");
        assert_eq!(
            film_name(Kind::Series, "Во все тяжкие"),
            "Сериал: Во все тяжкие"
        );
    }

    #[test]
    fn пустое_название_заменяется() {
        assert_eq!(film_name(Kind::Series, ""), "Сериал: Без названия");
        assert_eq!(film_name(Kind::Movie, "   "), "Фильм: Без названия");
    }
}
