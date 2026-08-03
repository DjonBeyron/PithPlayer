//! Шрифты интерфейса.
//!
//! Вторые субтитры пользователь читает поверх основных, и различать их
//! проще по начертанию, а не только по положению. Для них берётся
//! Comfortaa — она установлена в системе, встраивать её в программу
//! не нужно.

use std::path::PathBuf;

/// Имя семейства, под которым Comfortaa доступна в egui.
pub const COMFORTAA: &str = "comfortaa";

/// Имя семейства со значками интерфейса.
pub const ICONS: &str = "icons";

/// Загрузился ли шрифт значков.
static ICONS_READY: std::sync::OnceLock<bool> = std::sync::OnceLock::new();

/// Где искать шрифт значков.
///
/// Segoe Fluent Icons — системный набор Windows 11, Segoe MDL2 Assets —
/// его предшественник из Windows 10. Коды знаков у них общие, поэтому
/// подходит любой. Ничего не скачиваем и не распространяем: шрифт уже
/// стоит в системе.
fn icon_candidates() -> Vec<PathBuf> {
    let windir = std::env::var("WINDIR").unwrap_or_else(|_| "C:\\Windows".into());
    let fonts = PathBuf::from(windir).join("Fonts");

    vec![fonts.join("SegoeIcons.ttf"), fonts.join("segmdl2.ttf")]
}

/// Есть ли шрифт значков. Без него кнопки показывают запасные символы.
pub fn icons_available() -> bool {
    *ICONS_READY.get().unwrap_or(&false)
}

/// Семейство значков интерфейса.
pub fn icon_family() -> egui::FontFamily {
    egui::FontFamily::Name(ICONS.into())
}

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

/// Подключает системные шрифты: Comfortaa для вторых субтитров и набор
/// значков для кнопок.
///
/// Отсутствие любого из них — не ошибка: субтитры останутся в обычном
/// шрифте, а кнопки покажут запасные символы.
pub fn install(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();

    let comfortaa = comfortaa_data();
    if let Some((source, data)) = &comfortaa {
        fonts.font_data.insert(
            COMFORTAA.to_owned(),
            std::sync::Arc::new(egui::FontData::from_owned(data.clone())),
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

        tracing::info!(source, "Comfortaa подключена для вторых субтитров");
    } else {
        tracing::info!("Comfortaa не найдена — вторые субтитры оставляю в обычном шрифте");
    }

    let icons = load_first(&icon_candidates());
    if let Some((path, data)) = &icons {
        fonts.font_data.insert(
            ICONS.to_owned(),
            std::sync::Arc::new(egui::FontData::from_owned(data.clone())),
        );
        fonts
            .families
            .insert(egui::FontFamily::Name(ICONS.into()), vec![ICONS.to_owned()]);

        tracing::info!(?path, "шрифт значков подключён");
    } else {
        tracing::warn!("шрифт значков не найден — кнопки покажут запасные символы");
    }

    let _ = ICONS_READY.set(icons.is_some());

    if comfortaa.is_some() || icons.is_some() {
        ctx.set_fonts(fonts);
    }
}

/// Первый читаемый файл из списка.
fn load_first(paths: &[PathBuf]) -> Option<(PathBuf, Vec<u8>)> {
    paths
        .iter()
        .find_map(|path| std::fs::read(path).ok().map(|data| (path.clone(), data)))
}

/// Comfortaa: сначала из системы, иначе — встроенная копия.
///
/// Копия лежит в самой программе, поэтому вторые субтитры выглядят
/// одинаково на любой машине, даже там, где шрифт не установлен.
/// Системная версия предпочтительнее: пользователь мог поставить свою.
///
/// Шрифт распространяется по SIL Open Font License — вкладывать его
/// в программу лицензия разрешает (assets/Comfortaa-OFL.txt).
fn comfortaa_data() -> Option<(&'static str, Vec<u8>)> {
    const BUILT_IN: &[u8] = include_bytes!("../assets/Comfortaa-Regular.ttf");

    if let Some((_, data)) = load_first(&candidates()) {
        return Some(("система", data));
    }

    Some(("встроенная копия", BUILT_IN.to_vec()))
}

/// Семейство для вторых субтитров.
pub fn secondary_subtitle_family() -> egui::FontFamily {
    egui::FontFamily::Name(COMFORTAA.into())
}
