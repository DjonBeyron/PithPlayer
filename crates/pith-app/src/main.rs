//! Pith Player v5 — точка входа.

// Не открывать консоль в релизной сборке на Windows.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod associations;
mod bench;
mod cli;
mod logging;
mod single_instance;
mod slow;
mod theme;
mod ui;
mod video;
mod window;

use app::PithApp;
use pith_store::DataPaths;

/// Версия программы — её видно в заголовке окна и в правом верхнем углу.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

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
        viewport: restore_geometry(
            egui::ViewportBuilder::default()
                .with_title(format!("Pith Player {}", env!("CARGO_PKG_VERSION")))
                // Минимум согласован с подгонкой окна под форму видео:
                // уже этого панель управления не помещается (window.rs).
                .with_min_inner_size([380.0, 260.0])
                .with_icon(window_icon())
                .with_transparent(false),
            &data_paths,
        ),
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

/// Возвращает окно туда, где его закрыли в прошлый раз.
///
/// Настройки читаются до создания приложения: положение окна задаётся
/// при его создании, менять его потом — значит показать пользователю
/// скачок. Битые или бессмысленные значения пропускаем, тогда окно
/// откроется как обычно.
fn restore_geometry(viewport: egui::ViewportBuilder, paths: &DataPaths) -> egui::ViewportBuilder {
    /// Размер окна при первом запуске.
    const DEFAULT_SIZE: [f32; 2] = [1280.0, 720.0];

    let saved = pith_store::Settings::load(paths)
        .window
        .filter(pith_store::WindowGeometry::is_sane);

    match saved {
        Some(geometry) => {
            tracing::info!(?geometry, "окно открывается там же, где было закрыто");
            viewport
                .with_position([geometry.x, geometry.y])
                .with_inner_size([geometry.width, geometry.height])
        }
        None => viewport.with_inner_size(DEFAULT_SIZE),
    }
}

/// Иконка окна и панели задач.
///
/// Встроена в бинарник: плеер должен выглядеть одинаково и при запуске
/// из папки сборки, и после установки. Битая иконка — не повод падать,
/// вернём пустую и оставим окну значок по умолчанию.
fn window_icon() -> egui::IconData {
    const ICON: &[u8] = include_bytes!("../assets/icon.ico");

    match image::load_from_memory(ICON) {
        Ok(image) => {
            let rgba = image.to_rgba8();
            let (width, height) = rgba.dimensions();

            egui::IconData {
                rgba: rgba.into_raw(),
                width,
                height,
            }
        }
        Err(e) => {
            tracing::warn!(error = %e, "не удалось разобрать иконку окна");
            egui::IconData::default()
        }
    }
}
