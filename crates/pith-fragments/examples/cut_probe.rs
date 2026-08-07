//! Замер отрезка: режет и показывает, где начинаются видео и звук.
//!
//! Запуск:
//!   cargo run -p pith-fragments --release --example cut_probe -- "видео.mkv" 590.6 18
//!
//! Нужен, чтобы проверять нарезку на настоящих файлах: рассинхрон начала
//! потоков глазами виден как «первые секунды без звука», а здесь — числом.

use std::path::PathBuf;

use pith_fragments::FragmentJob;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();

    let (Some(source), Some(start), Some(duration)) = (args.first(), args.get(1), args.get(2))
    else {
        eprintln!("укажите: файл, начало в секундах, длительность в секундах");
        std::process::exit(2);
    };

    let (Ok(start), Ok(duration)) = (start.parse::<f64>(), duration.parse::<f64>()) else {
        eprintln!("начало и длительность — числа");
        std::process::exit(2);
    };

    let output = std::env::temp_dir().join("pith_cut_probe.mp4");
    let _ = std::fs::remove_file(&output);

    let job = FragmentJob {
        source: PathBuf::from(source),
        output: output.clone(),
        start,
        duration,
        audio_index: Some(0),
        reencode: false,
        // Режим по умолчанию: видео копией, звук в AAC.
        audio_aac: true,
    };

    let args = job.to_args();
    println!("ffmpeg {}", args.join(" "));

    let started = std::time::Instant::now();
    let status = std::process::Command::new("ffmpeg").args(&args).status();

    match status {
        Ok(status) if status.success() => {
            println!("готово за {} мс", started.elapsed().as_millis());
            println!("файл: {}", output.display());
        }
        Ok(status) => eprintln!("ffmpeg завершился с ошибкой: {status}"),
        Err(e) => eprintln!("не удалось запустить ffmpeg: {e}"),
    }
}
