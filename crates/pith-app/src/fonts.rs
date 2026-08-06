//! Шрифты интерфейса.
//!
//! У субтитров свои шрифты: основные — Montserrat, дополнительные —
//! Comfortaa. Их читают одновременно, и разные начертания различают слои
//! надёжнее, чем одно лишь положение на экране.
//!
//! Оба вложены в программу: вид субтитров не должен зависеть от того,
//! что установлено на машине. Системная копия, если она есть, берётся
//! первой — пользователь мог поставить свою версию.
//!
//! Оба распространяются по SIL Open Font License, вкладывать их
//! в программу лицензия разрешает (assets/Comfortaa-OFL.txt).

use std::path::PathBuf;

/// Имя семейства, под которым Comfortaa доступна в egui.
pub const COMFORTAA: &str = "comfortaa";

/// Имя семейства, под которым Montserrat доступна в egui.
pub const MONTSERRAT: &str = "montserrat";

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
fn candidates(names: &[&str]) -> Vec<PathBuf> {
    let mut roots = Vec::new();

    if let Ok(local) = std::env::var("LOCALAPPDATA") {
        roots.push(PathBuf::from(local).join("Microsoft\\Windows\\Fonts"));
    }

    if let Ok(windir) = std::env::var("WINDIR") {
        roots.push(PathBuf::from(windir).join("Fonts"));
    }

    roots
        .iter()
        .flat_map(|root| names.iter().map(|name| root.join(name)))
        .collect()
}

/// Подключает шрифты субтитров и набор значков для кнопок.
///
/// Шрифты субтитров есть всегда — встроенные копии лежат в программе.
/// Набор значков системный, и его отсутствие не ошибка: кнопки покажут
/// запасные символы.
pub fn install(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();

    install_subtitle_font(
        &mut fonts,
        COMFORTAA,
        comfortaa_data(),
        "дополнительных субтитров",
    );
    install_subtitle_font(
        &mut fonts,
        MONTSERRAT,
        montserrat_data(),
        "основных субтитров",
    );

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

    ctx.set_fonts(fonts);
}

/// Добавляет шрифт субтитров отдельным семейством.
///
/// Именно отдельным, а не подменой основного: остальной интерфейс
/// остаётся на прежнем шрифте.
fn install_subtitle_font(
    fonts: &mut egui::FontDefinitions,
    name: &str,
    font: (&'static str, Vec<u8>),
    purpose: &str,
) {
    let (source, data) = font;

    fonts.font_data.insert(
        name.to_owned(),
        std::sync::Arc::new(egui::FontData::from_owned(data)),
    );

    fonts.families.insert(
        egui::FontFamily::Name(name.into()),
        vec![
            name.to_owned(),
            // Запасной: если в шрифте не окажется нужных знаков,
            // они возьмутся из обычного.
            "Ubuntu-Light".to_owned(),
        ],
    );

    tracing::info!(шрифт = name, источник = source, "шрифт {purpose} подключён");
}

/// Первый читаемый файл из списка.
fn load_first(paths: &[PathBuf]) -> Option<(PathBuf, Vec<u8>)> {
    paths
        .iter()
        .find_map(|path| std::fs::read(path).ok().map(|data| (path.clone(), data)))
}

/// Comfortaa: сначала из системы, иначе — встроенная копия.
fn comfortaa_data() -> (&'static str, Vec<u8>) {
    const BUILT_IN: &[u8] = include_bytes!("../assets/Comfortaa-Regular.ttf");

    load_from_system(&["Comfortaa-Regular.ttf", "Comfortaa-Medium.ttf"], BUILT_IN)
}

/// Montserrat: сначала из системы, иначе — встроенная копия.
fn montserrat_data() -> (&'static str, Vec<u8>) {
    const BUILT_IN: &[u8] = include_bytes!("../assets/Montserrat-Regular.ttf");

    load_from_system(
        &["Montserrat-Regular.ttf", "Montserrat-Medium.ttf"],
        BUILT_IN,
    )
}

/// Системная копия шрифта, а если её нет — встроенная в программу.
///
/// Системная предпочтительнее: пользователь мог поставить свою версию,
/// например с большим набором знаков.
fn load_from_system(names: &[&str], built_in: &'static [u8]) -> (&'static str, Vec<u8>) {
    if let Some((_, data)) = load_first(&candidates(names)) {
        return ("система", data);
    }

    ("встроенная копия", built_in.to_vec())
}

/// Семейство основных субтитров.
pub fn main_subtitle_family() -> egui::FontFamily {
    egui::FontFamily::Name(MONTSERRAT.into())
}

/// Семейство дополнительных субтитров.
pub fn secondary_subtitle_family() -> egui::FontFamily {
    egui::FontFamily::Name(COMFORTAA.into())
}
