//! Pith Player v5 — точка входа.
//!
//! Этап 0: движок, окно, воспроизведение, замеры (PLAN.md §7).

// Не открывать консоль в релизной сборке на Windows.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod bench;
mod cli;
mod logging;
mod theme;
mod ui;
mod video;

use app::PithApp;

fn main() -> eframe::Result<()> {
    logging::init();
    logging::install_panic_hook();

    let raw_args: Vec<String> = std::env::args().skip(1).collect();

    if raw_args.iter().any(|a| a == "--help" || a == "-h") {
        println!("{}", cli::HELP);
        return Ok(());
    }

    let args = cli::Args::parse(raw_args);

    tracing::info!(version = env!("CARGO_PKG_VERSION"), "запуск Pith Player");

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Pith Player")
            .with_inner_size([1280.0, 720.0])
            .with_min_inner_size([480.0, 320.0]),
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

    eframe::run_native(
        "Pith Player",
        options,
        Box::new(move |cc| {
            theme::apply(&cc.egui_ctx);
            Ok(Box::new(PithApp::new(cc, args)))
        }),
    )
}
