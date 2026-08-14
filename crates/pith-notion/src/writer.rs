//! Запись строк в базу — в несколько потоков.
//!
//! Строка — это один запрос и один круг до Notion, около трёх четвертей
//! секунды. Тридцать закладок подряд — полминуты ожидания, и почти всё
//! это время плеер просто ждёт сети. Потоки её и разбирают.
//!
//! **Почему это стало можно.** Порядок строк в базе задаётся порядком
//! создания, и потоки его перемешивают. Пока номер жил только в текстовом
//! заголовке, порядок был единственным, на чём он держался. С числовым
//! полем `NUM` вид сортируется по нему, и кто первым доехал — неважно.
//!
//! **Сколько потоков.** Три: у Notion предел около трёх запросов в секунду,
//! дальше отказ `429`. Больше потоков только упёрлись бы в него и начали
//! ждать повторов.

use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use crate::Prepared;
use crate::client::Notion;
use crate::error::NotionError;
use crate::row::{self, Row};

/// Сколько строк писать одновременно.
const WORKERS: usize = 3;

/// Сколько раз повторять строку, которую отвергли по частоте запросов.
const RETRIES: usize = 3;

/// Сколько ждать перед повтором: дальше вдвое дольше.
const BACKOFF: Duration = Duration::from_secs(1);

/// Код отказа «слишком часто».
const TOO_MANY: u16 = 429;

/// Пишет строки в базу, сообщая о ходе работы.
///
/// `progress` зовётся из рабочих потоков по мере готовности строк — не
/// по порядку номеров. Отсюда и требование `Send`: считать готовое ровно
/// в том потоке, который дописал строку, дешевле, чем гонять итоги
/// через ещё один канал.
pub(crate) fn write_rows(
    notion: &Notion,
    prepared: &Prepared,
    film_name: &str,
    rows: &[Row],
    progress: impl FnMut(usize, usize) + Send,
) -> (usize, Vec<(usize, String)>) {
    let total = rows.len();

    if total == 0 {
        return (0, Vec::new());
    }

    let next = AtomicUsize::new(0);
    let created = AtomicUsize::new(0);
    let failed = Mutex::new(Vec::new());
    let done = AtomicUsize::new(0);
    let progress = Mutex::new(progress);

    std::thread::scope(|scope| {
        for _ in 0..WORKERS.min(total) {
            scope.spawn(|| {
                // Каждый поток берёт следующую свободную строку, а не свою
                // долю списка: строки одинаковы по цене, но сеть отвечает
                // вразнобой, и делёжка поровну оставила бы кого-то ждать.
                loop {
                    let index = next.fetch_add(1, Ordering::Relaxed);
                    let Some(row) = rows.get(index) else {
                        break;
                    };

                    let number = prepared.base + row.number;
                    let mut properties = prepared.sample.clone();
                    properties.extend(row::properties(row, film_name, number, prepared.numbered));

                    match create(notion, &prepared.database, properties) {
                        Ok(()) => {
                            created.fetch_add(1, Ordering::Relaxed);
                        }
                        Err(e) => {
                            tracing::warn!(строка = row.number, error = %e, "строка не создана");

                            if let Ok(mut failed) = failed.lock() {
                                failed.push((row.number, e.to_string()));
                            }
                        }
                    }

                    let ready = done.fetch_add(1, Ordering::Relaxed) + 1;

                    if let Ok(mut progress) = progress.lock() {
                        progress(ready, total);
                    }
                }
            });
        }
    });

    let mut failed = failed.into_inner().unwrap_or_default();

    // Отвечали вразнобой — в отчёте номера должны идти по порядку.
    failed.sort_by_key(|(number, _)| *number);

    (created.load(Ordering::Relaxed), failed)
}

/// Создаёт строку, переживая отказ «слишком часто».
///
/// Три потока держатся у самого предела Notion, и одиночный `429` —
/// дело обычное. Ждём и повторяем, всякий раз вдвое дольше. Прочие отказы
/// повторять незачем: битую строку Notion не примет и на второй раз.
fn create(
    notion: &Notion,
    database: &str,
    properties: serde_json::Map<String, serde_json::Value>,
) -> crate::Result<()> {
    let mut wait = BACKOFF;

    for attempt in 0..=RETRIES {
        match notion.create_row(database, properties.clone()) {
            Err(NotionError::Refused { status, message }) if status == TOO_MANY => {
                if attempt == RETRIES {
                    return Err(NotionError::Refused { status, message });
                }

                tracing::debug!(?wait, "Notion просит реже — жду и повторяю");
                std::thread::sleep(wait);
                wait *= 2;
            }
            other => return other,
        }
    }

    Ok(())
}
