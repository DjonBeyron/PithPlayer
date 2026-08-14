//! Проверка словарей на живых страницах.
//!
//! Показывает по каждому слову: что нашлось, где и за сколько. Слово,
//! которого нет в первом словаре, должно найтись во втором — за этим
//! и заведён второй.
//!
//! ```bash
//! cargo run -p pith-dict --release --example dict_probe -- "Where are you staying?"
//! cargo run -p pith-dict --release --example dict_probe -- --proxy 127.0.0.1:10809 "слова"
//! ```

use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Instant;

use pith_dict::{Dict, Source, split};

fn main() {
    let mut args: Vec<String> = std::env::args().skip(1).collect();

    if args.first().is_some_and(|first| first == "--proxy") {
        args.remove(0);

        if !args.is_empty() {
            let address = args.remove(0);
            println!("прокси: {address}");

            pith_dict::use_proxy(Some(address));
        }
    }

    // Пауза между запросами к первому словарю — главная цена транскрипции.
    let mut gap = None;

    if args.first().is_some_and(|first| first == "--gap") {
        args.remove(0);

        if !args.is_empty() {
            let ms: u64 = args.remove(0).parse().unwrap_or(1200);
            println!("пауза между словами: {ms} мс");
            gap = Some(std::time::Duration::from_millis(ms));
        }
    }

    // Сколько слов спрашивать разом. Один — как в плеере сейчас.
    let mut threads = 1;

    if args.first().is_some_and(|first| first == "--threads") {
        args.remove(0);

        if !args.is_empty() {
            threads = args.remove(0).parse().unwrap_or(1usize).max(1);
            println!("потоков: {threads}");
        }
    }

    let phrase = args.join(" ");
    let words = split(&phrase);

    if words.is_empty() {
        eprintln!("нужна фраза: dict_probe -- \"Where are you staying?\"");
        std::process::exit(2);
    }

    println!("слов: {}", words.len());

    let dict = match gap {
        Some(gap) => Dict::with_gap(gap),
        None => Dict::new(),
    };
    let started = Instant::now();
    let next = AtomicUsize::new(0);
    let results = Mutex::new(Vec::new());

    // Слова разбираются потоками из общей очереди: они разной цены,
    // и делёжка поровну оставила бы кого-то ждать.
    std::thread::scope(|scope| {
        for _ in 0..threads.min(words.len()) {
            scope.spawn(|| {
                loop {
                    let at = next.fetch_add(1, Ordering::Relaxed);
                    let Some(word) = words.get(at) else {
                        break;
                    };

                    let started = Instant::now();
                    let found = dict.lookup(word);
                    let spent = started.elapsed().as_secs_f32();

                    if let Ok(mut results) = results.lock() {
                        results.push((word.clone(), found, spent));
                    }
                }
            });
        }
    });

    let results = results.into_inner().unwrap_or_default();
    let mut found = 0;
    let mut first = 0;
    let mut second = 0;

    for (word, result, spent) in &results {
        match result {
            Some(result) => {
                found += 1;

                match result.source {
                    Source::Wooordhunt => first += 1,
                    Source::Cambridge => second += 1,
                }

                println!(
                    "  {word:16} {:14} {:10} {spent:.2} с",
                    result.transcription,
                    source_name(result.source)
                );
            }
            None => println!("  {word:16} {:14} {:10} {spent:.2} с", "—", "не нашлось"),
        }
    }

    println!(
        "нашлось {found} из {} · первый словарь {first} · второй {second} · всего {:.1} с",
        words.len(),
        started.elapsed().as_secs_f32()
    );
}

fn source_name(source: Source) -> &'static str {
    match source {
        Source::Wooordhunt => "wooordhunt",
        Source::Cambridge => "cambridge",
    }
}
