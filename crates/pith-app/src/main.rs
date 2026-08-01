//! Pith Player v5 — точка входа.

// Не открывать консоль в релизной сборке на Windows.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod bench;
mod cli;
mod logging;
mod single_instance;
mod theme;
mod ui;
mod video;
mod window;

use app::PithApp;
use pith_store::DataPaths;

fn main() -> eframe::Result<()> {
    logging::init();
    logging::install_panic_hook();

    let raw_args: Vec<String> = std::env::args().skip(1).collect();

    if raw_args.iter().any(|a| a == "--help" || a == "-h") {
        println!("{}", cli::HELP);
        return Ok(());
    }

    let args = cli::Args::parse(raw_args);
    let data_paths = DataPaths::discover();

    // Второй запуск отдаёт файл работающему плееру и выходит.
    let Some(instance) =
        single_instance::become_primary_or_forward(&data_paths, args.file.as_deref())
    else {
        return Ok(());
    };

    tracing::info!(version = env!("CARGO_PKG_VERSION"), "запуск Pith Player");

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Pith Player")
            .with_inner_size([1280.0, 720.0])
            // Минимум согласован с подгонкой окна под форму видео:
            // уже этого панель управления не помещается (window.rs).
            .with_min_inner_size([380.0, 260.0])
            .with_transparent(false),
        renderer: eframe::Renderer::Glow,
        // Завершаться сразу из цикла событий, а не возвращаться в main.
        //
        // При значении `true` (умолчание eframe) окно уничтожается первым,
        // а приложение освобождается уже после — и mpv с загруженным файлом
        // намертво подвешивает выход. При `false` eframe вызывает `on_exit`,
        // где движок освобождается штатно, и только потом закрывает процесс.
        run_and_return: false,
        ..Default::default()
    };

    let result = eframe::run_native(
        "Pith Player",
        options,
        Box::new(move |cc| {
            theme::apply(&cc.egui_ctx);
            Ok(Box::new(PithApp::new(cc, args, instance)))
        }),
    );

    single_instance::release(&data_paths);
    result
}
