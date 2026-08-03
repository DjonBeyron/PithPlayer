//! Нижняя панель управления.
//!
//! Рисуется поверх видео отдельным слоем: mpv закрашивает всё, что
//! нарисовано до него.

use crate::app::PithApp;
use crate::theme;
use crate::ui::{format_time, icons, metrics, timeline};

/// Размер кнопок панели: одинаковый, чтобы строка не прыгала.
const BUTTON_SIZE: [f32; 2] = [30.0, 24.0];
/// Ширина полосы громкости.
const VOLUME_WIDTH: f32 = 110.0;
/// Минимальная ширина полосы перемотки.
const MIN_TIMELINE_WIDTH: f32 = 120.0;
/// Уже этой ширины громкость прячем: окно подогнано под вертикальное видео,
/// и места хватает только на перемотку.
const NARROW_WINDOW: f32 = 620.0;
/// Отступ содержимого панели от краёв окна — одинаковый слева и справа.
const SIDE_MARGIN: f32 = 12.0;
/// Через сколько секунд прятать панель в полноэкранном режиме.
const HIDE_AFTER_SECONDS: f64 = 1.5;
/// Высота полосы у нижнего края, которой панель вызывается обратно.
///
/// Считается от низа окна и чуть выше самой панели: иначе она мигала бы
/// на своей же границе.
const BOTTOM_ZONE: f32 = 90.0;

/// Панель управления и панель замеров.
pub fn show_controls(app: &mut PithApp, ctx: &egui::Context) {
    metrics::show(app, ctx);

    if !is_visible(app, ctx) {
        return;
    }

    let screen = ctx.input(|i| i.viewport_rect());

    // Якорь к нижнему краю, а не расчёт положения по заданной высоте:
    // содержимое занимает не ровно столько, сколько мы предполагали,
    // и под панелью оставалась полоска видео.
    egui::Area::new(egui::Id::new("controls"))
        .order(egui::Order::Foreground)
        .anchor(egui::Align2::LEFT_BOTTOM, [0.0, 0.0])
        .show(ctx, |ui| {
            ui.set_width(screen.width());

            egui::Frame::NONE
                .fill(theme::PANEL_BG.gamma_multiply(0.92))
                // Отступы слева и справа задаёт только эта рамка: сужать
                // содержимое дополнительно нельзя, иначе правый край
                // отходит дальше левого.
                .inner_margin(egui::Margin::symmetric(SIDE_MARGIN as i8, 8))
                .show(ui, |ui| {
                    show_row(app, ui, screen.width() - SIDE_MARGIN * 2.0);
                });
        });
}

/// Показывать ли панель.
///
/// В оконном режиме панель видна всегда. На весь экран — включая режим
/// с обрезанными полями — прячется через полторы секунды и возвращается,
/// когда курсор опускается к нижнему краю.
fn is_visible(app: &PithApp, ctx: &egui::Context) -> bool {
    if !app.is_fullscreen() {
        return true;
    }

    let screen = ctx.input(|i| i.viewport_rect());

    // Курсор у нижнего края — панель нужна прямо сейчас.
    let at_bottom = ctx
        .input(|i| i.pointer.hover_pos())
        .is_some_and(|pointer| pointer.y >= screen.max.y - BOTTOM_ZONE);

    if at_bottom {
        return true;
    }

    // Иначе даём ей полторы секунды после последнего движения мыши:
    // сразу гасить панель, когда пользователь только что её вызвал,
    // было бы неудобно.
    let idle = ctx.input(|i| i.time - app.last_pointer_activity());
    idle < HIDE_AFTER_SECONDS
}

fn show_row(app: &mut PithApp, ui: &mut egui::Ui, inner_width: f32) {
    ui.horizontal(|ui| {
        show_open_button(app, ui);
        show_play_button(app, ui);
        show_time_label(app, ui);

        // В узком окне (подогнанном под вертикальное видео) громкости места
        // не остаётся — уступаем его полосе перемотки.
        let with_volume = inner_width >= NARROW_WINDOW;

        // Правый край выкладываем справа налево: тогда последний элемент
        // упирается ровно в отступ панели, и края выходят одинаковыми.
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if with_volume {
                show_volume(app, ui);
            }

            show_fullscreen_button(app, ui);
            show_crop_button(app, ui);
            show_speed(app, ui);

            // Полоса перемотки забирает всё, что осталось между краями.
            let width =
                (ui.available_width() - ui.spacing().item_spacing.x).max(MIN_TIMELINE_WIDTH);
            show_timeline(app, ui, width);
        });
    });
}

/// Кнопка открытия файла.
fn show_open_button(app: &mut PithApp, ui: &mut egui::Ui) {
    if icon_button(ui, icons::OPEN, "Открыть файл") {
        app.open_file_dialog();
    }
}

/// Пауза и продолжение — одной кнопкой постоянного размера.
///
/// Размер задан явно: у значков воспроизведения и паузы разная ширина,
/// и кнопка прыгала при каждом нажатии.
fn show_play_button(app: &mut PithApp, ui: &mut egui::Ui) {
    let paused = app.engine().map(|e| e.state().paused).unwrap_or(true);
    let icon = if paused { icons::PLAY } else { icons::PAUSE };

    if icon_button(ui, icon, "Пауза / продолжить (пробел)") {
        app.toggle_pause();
    }
}

/// Кнопка полноэкранного режима.
fn show_fullscreen_button(app: &mut PithApp, ui: &mut egui::Ui) {
    let (icon, hint) = if app.is_fullscreen() {
        (icons::RESTORE, "Выйти из полного экрана (F)")
    } else {
        (icons::FULLSCREEN, "Во весь экран (F)")
    };

    if icon_button(ui, icon, hint) {
        app.toggle_fullscreen(ui.ctx());
    }
}

/// Кнопка «растянуть на весь экран», убирающая чёрные поля.
///
/// Показывается только в полноэкранном режиме: в окне поля почти не
/// мешают, а вот на большом экране им достаётся заметная часть площади.
fn show_crop_button(app: &mut PithApp, ui: &mut egui::Ui) {
    if !app.is_fullscreen() {
        return;
    }

    if app.is_detecting_crop() {
        ui.add_sized(BUTTON_SIZE, egui::Spinner::new())
            .on_hover_text("Ищу чёрные поля…");
        return;
    }

    let (icon, hint) = if app.is_cropped() {
        (icons::FIT_ORIGINAL, "Вернуть чёрные поля")
    } else {
        (
            icons::FIT_SCREEN,
            "Растянуть на весь экран: FFmpeg найдёт чёрные поля и уберёт их",
        )
    };

    if icon_button(ui, icon, hint) {
        app.toggle_crop();
    }
}

/// Кнопка со значком: одинаковый размер, без рамки поверх видео.
fn icon_button(ui: &mut egui::Ui, icon: icons::Icon, hint: &str) -> bool {
    ui.add_sized(
        BUTTON_SIZE,
        egui::Button::new(icon.text().color(theme::TEXT_PRIMARY)).frame(false),
    )
    .on_hover_text(hint)
    .clicked()
}

fn show_time_label(app: &PithApp, ui: &mut egui::Ui) {
    let (position, duration) = app
        .engine()
        .map(|e| (e.state().position, e.state().duration))
        .unwrap_or((0.0, 0.0));

    ui.label(
        egui::RichText::new(format!(
            "{} / {}",
            format_time(position),
            format_time(duration)
        ))
        .color(theme::TEXT_PRIMARY)
        .monospace(),
    );
}

fn show_timeline(app: &mut PithApp, ui: &mut egui::Ui, width: f32) {
    let (position, duration) = app
        .engine()
        .map(|e| (e.state().position, e.state().duration))
        .unwrap_or((0.0, 0.0));

    // Закладки появятся на этапе 4 — до тех пор жёлтых отрезков нет.
    let fragments = app.fragment_ranges();
    let response = timeline::show(ui, position, duration, width, &fragments);

    if let Some(target) = response.seek_to {
        app.seek_absolute(target);
    }
}

/// Скорость воспроизведения. Показывается, только когда отличается от обычной.
fn show_speed(app: &mut PithApp, ui: &mut egui::Ui) {
    let speed = app.engine().map(|e| e.state().speed).unwrap_or(1.0);

    if (speed - 1.0).abs() < f64::EPSILON {
        return;
    }

    let label = ui.label(
        egui::RichText::new(format!("{speed:.2}×"))
            .color(theme::ACCENT)
            .monospace(),
    );

    if label
        .on_hover_text("Сбросить скорость")
        .interact(egui::Sense::click())
        .clicked()
    {
        app.reset_speed();
    }
}

/// Громкость — такой же полосой, как перемотка.
///
/// Выкладывается справа налево, поэтому полоса идёт первой, а значок
/// после неё оказывается слева.
fn show_volume(app: &mut PithApp, ui: &mut egui::Ui) {
    let Some(engine) = app.engine() else {
        return;
    };

    let volume = engine.state().volume;

    if let Some(chosen) = timeline::volume_bar(ui, volume, VOLUME_WIDTH, MAX_VOLUME) {
        app.set_volume(chosen);
    }

    let icon = if volume == 0 {
        icons::MUTE
    } else {
        icons::VOLUME
    };
    ui.label(icon.text().color(theme::TEXT_SECONDARY))
        .on_hover_text(format!("Громкость: {volume}%"));
}

/// Предел громкости — тот же, что у движка.
const MAX_VOLUME: i64 = 150;
