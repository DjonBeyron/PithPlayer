//! Замер: через сколько после команды «открыть» mpv сообщает форму кадра.
//!
//! Запуск: `cargo run -p pith-mpv --release --example size_probe -- "видео.mkv"`
//!
//! Нужен для окна: чтобы оно открывалось сразу нужного размера, форму кадра
//! надо узнать до создания окна. Замер показывает, во что это обойдётся.

use std::time::{Duration, Instant};

use pith_mpv::{Engine, EngineOptions};

fn main() {
    let Some(path) = std::env::args().nth(1) else {
        eprintln!("укажите файл видео");
        std::process::exit(2);
    };

    if let Err(message) = probe(&path) {
        eprintln!("{message}");
        std::process::exit(1);
    }
}

fn probe(path: &str) -> Result<(), String> {
    let started = Instant::now();

    let mut engine = Engine::new(&EngineOptions::default()).map_err(|e| e.to_string())?;
    println!(
        "движок создан:      {:>5} мс",
        started.elapsed().as_millis()
    );

    engine.load_file(path).map_err(|e| e.to_string())?;
    println!(
        "команда открытия:   {:>5} мс",
        started.elapsed().as_millis()
    );

    // Ждём событиями, а не опросом по кругу: очередь сама будит нас,
    // когда mpv есть что сказать.
    let deadline = Instant::now() + Duration::from_secs(5);

    loop {
        if let Some((width, height)) = engine.demux_video_size() {
            println!(
                "форма от демуксера: {:>5} мс   {width}×{height}",
                started.elapsed().as_millis()
            );
            break;
        }

        if Instant::now() >= deadline {
            return Err("форма кадра так и не пришла".into());
        }

        for event in engine.pump_events() {
            println!(
                "  событие {:>5} мс: {event:?}",
                started.elapsed().as_millis()
            );
        }

        std::thread::sleep(Duration::from_millis(5));
    }

    Ok(())
}
