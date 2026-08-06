//! Замер источника кадров предпросмотра.
//!
//! Запуск: `cargo run -p pith-mpv --example preview_probe -- "видео.mkv"`.
//! Достаёт несколько кадров подряд и печатает, сколько занял каждый.

use std::path::Path;
use std::time::Instant;

fn main() {
    let Some(path) = std::env::args().nth(1) else {
        eprintln!("укажите файл видео");
        std::process::exit(2);
    };

    if let Err(message) = probe(Path::new(&path)) {
        eprintln!("{message}");
        std::process::exit(1);
    }
}

fn probe(path: &Path) -> Result<(), String> {
    let shot = std::env::temp_dir().join("pith_preview_probe.jpg");
    let mut engine = pith_mpv::PreviewEngine::new(shot).map_err(|e| e.to_string())?;

    let started = Instant::now();
    engine.load_file(path).map_err(|e| e.to_string())?;
    println!("открытие: {} мс", started.elapsed().as_millis());

    for time in [5.0, 300.0, 1200.0, 1205.0, 3000.0] {
        let started = Instant::now();

        match engine.grab(time) {
            Ok(data) => println!(
                "кадр {time:>7.1} с: {} байт за {} мс",
                data.len(),
                started.elapsed().as_millis()
            ),
            Err(e) => println!("кадр {time:>7.1} с: не получен — {e}"),
        }
    }

    Ok(())
}
