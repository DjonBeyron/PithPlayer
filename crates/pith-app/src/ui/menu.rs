//! Контекстное меню по правому щелчку.
//!
//! Порт `ModernContextMenu` из v4: выбор дорожек, скорость, полный экран.

use crate::app::PithApp;
use crate::theme;
use crate::tr;
use crate::ui::menu_tracks;

/// Готовые значения скорости.
const SPEED_PRESETS: [f64; 6] = [0.5, 0.75, 1.0, 1.25, 1.5, 2.0];

/// Минимальная ширина меню.
const MENU_WIDTH: f32 = 220.0;

/// Отступ между группами пунктов.
///
/// Заметный: без него подписи групп липли к пунктам соседней, и меню
/// читалось сплошным списком.
const GROUP_GAP: f32 = 12.0;

/// Пункты меню.
///
/// Вызывается из `Response::context_menu`, а не рисуется своей областью:
/// только внутри меню-контекста `ui.menu_button` превращается
/// в раскрывающийся по наведению `SubMenuButton`, который egui сам
/// размещает сбоку и переворачивает у края экрана.
pub fn show_items(app: &mut PithApp, ui: &mut egui::Ui) {
    ui.set_min_width(MENU_WIDTH);

    // Меню разбито на группы с подписями: пункты копились по мере
    // переноса возможностей из v4 и лежали вперемешку — дорожки рядом
    // с ассоциациями файлов, полный экран между замерами и нарезкой.
    show_group(ui, tr!("Файл", "File"));
    show_file_items(app, ui);

    show_group(ui, tr!("Просмотр", "Playback"));
    show_playback_items(app, ui);

    show_group(ui, tr!("Дорожки", "Tracks"));
    menu_tracks::show_audio_tracks(app, ui);
    menu_tracks::show_subtitle_tracks(app, ui);
    menu_tracks::show_audio_devices(app, ui);

    show_group(ui, tr!("Отрезки", "Fragments"));
    show_fragment_items(app, ui);

    show_group(ui, tr!("Прочее", "Other"));
    show_service_items(app, ui);
}

/// Подпись группы и отступ перед ней.
///
/// Отступ заметный: без него подписи липли к пунктам соседней группы,
/// и меню читалось сплошным списком.
fn show_group(ui: &mut egui::Ui, title: &str) {
    if ui.min_rect().height() > 0.0 {
        ui.add_space(GROUP_GAP);
    }

    ui.label(
        egui::RichText::new(title.to_uppercase())
            .color(theme::PANEL_MUTED)
            .size(10.0)
            .strong(),
    );

    ui.add_space(2.0);
}

fn show_file_items(app: &mut PithApp, ui: &mut egui::Ui) {
    if ui.button(tr!("Открыть файл…", "Open file…")).clicked() {
        app.open_file_dialog();
        ui.close();
    }

    if ui.button(tr!("История файлов…", "Recent files…")).clicked() {
        app.open_history();
        ui.close();
    }

    if ui
        .button(tr!("Поиск по субтитрам…", "Search subtitles…"))
        .clicked()
    {
        app.open_search();
        ui.close();
    }
}

fn show_playback_items(app: &mut PithApp, ui: &mut egui::Ui) {
    if ui.button(tr!("Полный экран", "Fullscreen")).clicked() {
        app.toggle_fullscreen(ui.ctx());
        ui.close();
    }

    // Повтор есть и кнопкой в панели, но в маленьком окне её там нет:
    // единственного способа включить повтор остаться не должно.
    let looping = if app.is_looping() {
        tr!("Не повторять файл", "Stop looping")
    } else {
        tr!("Повторять файл по кругу", "Loop file")
    };

    if ui.button(looping).clicked() {
        app.toggle_looping();
        ui.close();
    }

    show_speed(app, ui);

    let label = if app.settings().subtitles_visible {
        tr!("Скрыть субтитры", "Hide subtitles")
    } else {
        tr!("Показать субтитры", "Show subtitles")
    };

    if ui.button(label).clicked() {
        app.toggle_subtitles();
        ui.close();
    }

    if ui
        .button(tr!("Вид субтитров…", "Subtitle look…"))
        .on_hover_text(tr!(
            "Цвет и начертание каждого слоя",
            "Colour and weight of each layer"
        ))
        .clicked()
    {
        app.open_subtitle_style();
        ui.close();
    }
}

fn show_fragment_items(app: &mut PithApp, ui: &mut egui::Ui) {
    show_bookmark_lists(app, ui);

    // Панель вызывается язычком у правого края; через меню её можно
    // закрепить, чтобы не закрывалась нажатием мимо.
    let pin_label = if app.bookmarks_panel_pinned() {
        tr!("Открепить панель отрезков", "Unpin fragments panel")
    } else {
        tr!("Закрепить панель отрезков", "Pin fragments panel")
    };

    if ui.button(pin_label).clicked() {
        app.toggle_bookmarks_panel();
        ui.close();
    }

    if ui
        .button(tr!("Настройки нарезки…", "Fragment settings…"))
        .clicked()
    {
        app.open_fragment_settings();
        ui.close();
    }
}

fn show_service_items(app: &mut PithApp, ui: &mut egui::Ui) {
    show_languages(app, ui);

    if ui
        .button(tr!("Интеграции…", "Integrations…"))
        .on_hover_text(tr!(
            "Доступ к Notion и ключ базы фильмов",
            "Notion access and the movie database key"
        ))
        .clicked()
    {
        app.open_integrations();
        ui.close();
    }

    let metrics_label = if app.show_metrics() {
        tr!("Скрыть замеры", "Hide metrics")
    } else {
        tr!("Показать замеры", "Show metrics")
    };

    if ui.button(metrics_label).clicked() {
        app.toggle_metrics();
        ui.close();
    }

    // Ассоциации меняют настройки системы, поэтому пункт только открывает
    // подтверждение, а не выполняет действие сразу.
    let label = if app.file_types_registered() {
        tr!("Отвязать видеофайлы…", "Unlink video files…")
    } else {
        tr!(
            "Связать видеофайлы с плеером…",
            "Set as default video player…"
        )
    };

    if ui.button(label).clicked() {
        app.ask_file_types();
        ui.close();
    }
}

/// Язык интерфейса.
///
/// Названия языков не переводятся: своё слово в списке ищут глазами,
/// не читая остального меню.
fn show_languages(app: &mut PithApp, ui: &mut egui::Ui) {
    let current = app.language();
    let mut chosen = None;

    ui.menu_button(tr!("Язык", "Language"), |ui| {
        for language in pith_store::Language::ALL {
            if ui.radio(language == current, language.label()).clicked() {
                chosen = Some(language);
                ui.close();
            }
        }
    });

    if let Some(language) = chosen {
        app.set_language(language);
    }
}

/// Переключение списка отрезков, не открывая панель.
fn show_bookmark_lists(app: &mut PithApp, ui: &mut egui::Ui) {
    let names = app.list_names();
    let Some(active) = app.active_list_name() else {
        return;
    };

    let mut chosen = None;
    let mut create = false;

    let shown = crate::i18n::list_name(&active);
    let title = tr!(
        format!("Список отрезков: {shown}"),
        format!("Fragment list: {shown}")
    );

    ui.menu_button(title, |ui| {
        for name in &names {
            if ui
                .radio(*name == active, crate::i18n::list_name(name))
                .clicked()
            {
                chosen = Some(name.clone());
                ui.close();
            }
        }

        ui.separator();

        if ui.button(tr!("Новый список…", "New list…")).clicked() {
            create = true;
            ui.close();
        }
    });

    if let Some(name) = chosen {
        app.switch_list(&name);
    }
    if create {
        app.open_new_list_dialog();
    }
}

fn show_speed(app: &mut PithApp, ui: &mut egui::Ui) {
    let current = app.engine().map(|e| e.state().speed).unwrap_or(1.0);
    let mut chosen = None;

    let title = tr!(
        format!("Скорость: {current:.2}×"),
        format!("Speed: {current:.2}×")
    );

    ui.menu_button(title, |ui| {
        for speed in SPEED_PRESETS {
            let active = (current - speed).abs() < 0.01;
            if ui.radio(active, format!("{speed:.2}×")).clicked() {
                chosen = Some(speed);
                ui.close();
            }
        }
    });

    if let Some(speed) = chosen {
        app.set_speed(speed);
    }
}
