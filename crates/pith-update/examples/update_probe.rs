//! Проверка обновления на живом GitHub.
//!
//! ```bash
//! cargo run -p pith-update --release --example update_probe -- 5.1.0
//! cargo run -p pith-update --release --example update_probe -- 5.1.0 --download
//! ```

use std::time::Instant;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let current = args.first().map_or("0.0.0", String::as_str);
    let download = args.iter().any(|arg| arg == "--download");

    let started = Instant::now();

    let release = match pith_update::check(current) {
        Ok(Some(release)) => release,
        Ok(None) => {
            println!("обновления нет ({:.2} с)", started.elapsed().as_secs_f32());
            return;
        }
        Err(e) => {
            println!("не вышло: {e}");
            return;
        }
    };

    println!(
        "вышло {} за {:.2} с\n  установщик: {} ({:.1} МБ)\n  страница: {}\n  заметка: {} знаков",
        release.version,
        started.elapsed().as_secs_f32(),
        release.installer.name,
        release.installer.size as f64 / 1024.0 / 1024.0,
        release.page,
        release.notes.chars().count(),
    );

    if !download {
        return;
    }

    let into = std::env::temp_dir().join("pith-update-probe");
    let started = Instant::now();
    let mut last = 0;

    match pith_update::download(&release.installer, &into, |done, total| {
        let percent = done * 100 / total.max(1);
        if percent >= last + 10 {
            last = percent;
            println!("  {percent}%");
        }
    }) {
        Ok(path) => println!(
            "загружен за {:.1} с: {}",
            started.elapsed().as_secs_f32(),
            path.display()
        ),
        Err(e) => println!("загрузка не вышла: {e}"),
    }
}
