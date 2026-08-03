//! Шрифты интерфейса.
//!
//! Вторые субтитры пользователь читает поверх основных, и различать их
//! проще по начертанию, а не только по положению. Для них берётся
//! Comfortaa — она установлена в системе, встраивать её в программу
//! не нужно.

use std::path::PathBuf;

/// Имя семейства, под которым Comfortaa доступна в egui.
pub const COMFORTAA: &str = "comfortaa";

/// Где искать файл шрифта: сначала профиль пользователя, потом система.
///
/// Шрифты, поставленные «для себя», лежат в профиле — именно так их
/// обычно и устанавливают.
fn candidates() -> Vec<PathBuf> {
    let mut paths = Vec::new();

    if let Ok(local) = std::env::var("LOCALAPPDATA") {
        paths.push(PathBuf::from(&local).join("Microsoft\\Windows\\Fonts\\Comfortaa-Regular.ttf"));
        paths.push(PathBuf::from(&local).join("Microsoft\\Windows\\Fonts\\Comfortaa-Medium.ttf"));
    }

    if let Ok(windir) = std::env::var("WINDIR") {
        paths.push(PathBuf::from(&windir).join("Fonts\\Comfortaa-Regular.ttf"));
        paths.push(PathBuf::from(&windir).join("Fonts\\Comfortaa-Medium.ttf"));
    }

    paths
}

/// Подключает Comfortaa, если она есть в системе.
///
/// Отсутствие шрифта — не ошибка: вторые субтитры просто останутся
/// в шрифте по умолчанию.
pub fn install(ctx: &egui::Context) {
    let Some((path, data)) = candidates()
        .into_iter()
        .find_map(|path| std::fs::read(&path).ok().map(|data| (path, data)))
    else {
        tracing::info!("Comfortaa не найдена — вторые субтитры оставляю в обычном шрифте");
        return;
    };

    let mut fonts = egui::FontDefinitions::default();
    fonts.font_data.insert(
        COMFORTAA.to_owned(),
        std::sync::Arc::new(egui::FontData::from_owned(data)),
    );

    // Отдельное семейство, а не подмена основного: остальной интерфейс
    // остаётся на прежнем шрифте.
    fonts.families.insert(
        egui::FontFamily::Name(COMFORTAA.into()),
        vec![
            COMFORTAA.to_owned(),
            // Запасной: у Comfortaa нет кириллицы в старых версиях,
            // недостающие знаки возьмутся из обычного шрифта.
            "Ubuntu-Light".to_owned(),
        ],
    );

    ctx.set_fonts(fonts);
    tracing::info!(?path, "Comfortaa подключена для вторых субтитров");
}

/// Семейство для вторых субтитров.
pub fn secondary_subtitle_family() -> egui::FontFamily {
    egui::FontFamily::Name(COMFORTAA.into())
}
