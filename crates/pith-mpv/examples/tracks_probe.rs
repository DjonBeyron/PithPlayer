//! Замер: во что обходится список дорожек сразу после загрузки файла.
//!
//! Запуск: `cargo run -p pith-mpv --release --example tracks_probe -- "видео.mkv"`
//!
//! Занятый mpv отвечает на запрос свойства не сразу, а ждать приходится
//! в потоке интерфейса. Замер показывает, во сколько обходится список
//! дорожек в самый занятый миг — сразу после загрузки (PLAN.md §6.14).

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
    let mut engine = Engine::new(&EngineOptions::default()).map_err(|e| e.to_string())?;
    engine.load_file(path).map_err(|e| e.to_string())?;

    // Ждём загрузки событиями: опроса по кругу здесь нет.
    let deadline = Instant::now() + Duration::from_secs(20);
    while !engine.state().file_loaded {
        if Instant::now() >= deadline {
            return Err("файл так и не загрузился".into());
        }
        engine.pump_events();
        std::thread::sleep(Duration::from_millis(2));
    }

    let started = Instant::now();
    let tracks = engine.tracks();
    println!(
        "список дорожек: {:>4} мс   дорожек {}",
        started.elapsed().as_millis(),
        tracks.len()
    );

    for track in &tracks {
        println!("  {:?} {}", track.kind, track.label());
    }

    Ok(())
}
