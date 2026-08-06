//! Нижняя панель управления.
//!
//! Рисуется поверх видео отдельным слоем: mpv закрашивает всё, что
//! нарисовано до него.

use crate::app::PithApp;
use crate::theme;
use crate::tr;
use crate::ui::{
    format_time, format_time_padded, icons, metrics, preview, speed, time_label, timeline, volume,
};

/// Размер кнопок панели: одинаковый, чтобы строка не прыгала.
pub(super) const BUTTON_SIZE: [f32; 2] = [30.0, 24.0];

/// Уже этой ширины полосу громкости прячем под динамик: окно подогнано
/// под вертикальное видео, и места хватает только на перемотку.
///
/// Считается по строке целиком: кнопки, обе надписи со временем, полоса
/// громкости и перемотка в её наименьшей ширине.
const NARROW_WINDOW: f32 = 760.0;

/// Уже этой ширины в строке остаются только пауза, закладка, звук и полоса
/// перемотки. Уходят и надписи со временем: в такой ширине они отнимают
/// у полосы больше, чем сообщают сами.
///
/// Остальное — открытие файла, повтор, скорость, поиск, полный экран —
/// доступно из контекстного меню по правому щелчку. Полоса перемотки нужнее
/// кнопок: без неё плеером не пользуются вовсе, а её место занимали бы они.
const TINY_WINDOW: f32 = 540.0;
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
        // Вместе с панелью убираем и указатель: на весь экран он висит
        // поверх картинки и мешает ровно так же, как и сама панель.
        // Вернётся от первого же движения мышью.
        ctx.set_cursor_icon(egui::CursorIcon::None);
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

    // На паузе кадры сами не идут, и отсчёт до скрытия замер бы на месте.
    ctx.request_repaint_after(std::time::Duration::from_millis(200));

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
    // В узком окне (подогнанном под вертикальное видео) полосе громкости
    // места не остаётся — уступаем его перемотке, а саму громкость
    // прячем под динамик. В совсем маленьком убираем и кнопки.
    let compact_volume = inner_width < NARROW_WINDOW;
    let tiny = inner_width < TINY_WINDOW;

    ui.horizontal(|ui| {
        if !tiny {
            show_open_button(app, ui);
        }

        show_play_button(app, ui);

        if !tiny {
            show_loop_button(app, ui);
            speed::show(app, ui);
            show_position(app, ui);
        }

        // Правый край выкладываем справа налево: тогда последний элемент
        // упирается ровно в отступ панели, и края выходят одинаковыми.
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            volume::show(app, ui, compact_volume);

            if !tiny {
                show_fullscreen_button(app, ui);
                show_crop_button(app, ui);
                show_search_button(app, ui);
            }

            // Закладка ставится по ходу просмотра, и кнопка стоит сразу
            // за полосой перемотки: рука и так у неё.
            show_bookmark_button(app, ui);

            // Длительность — у правого конца полосы: слева время идёт
            // вперёд, справа стоит предел, до которого оно дойдёт.
            if !tiny {
                show_duration(app, ui);
            }

            // Полоса перемотки забирает ровно то, что осталось между
            // краями, и ни точкой больше. Заданная снизу ширина в тесном
            // окне не помещалась, и полоса наползала на надпись со
            // временем — время оказывалось под ней.
            let width = (ui.available_width() - ui.spacing().item_spacing.x).max(0.0);
            show_timeline(app, ui, width);
        });
    });
}

/// Кнопка открытия файла. По правому щелчку — история открытых.
fn show_open_button(app: &mut PithApp, ui: &mut egui::Ui) {
    let response = ui
        .add_sized(
            BUTTON_SIZE,
            egui::Button::new(icons::OPEN.text().color(theme::TEXT_PRIMARY)).frame(false),
        )
        .on_hover_text(tr!(
            "Открыть файл. Правым щелчком — история открытых",
            "Open file. Right-click for recent files"
        ));

    if crate::ui::clicked(&response) {
        app.open_file_dialog();
    }

    if response.secondary_clicked() {
        app.open_history();
    }
}

/// Пауза и продолжение — одной кнопкой постоянного размера.
///
/// Размер задан явно: у значков воспроизведения и паузы разная ширина,
/// и кнопка прыгала при каждом нажатии.
fn show_play_button(app: &mut PithApp, ui: &mut egui::Ui) {
    let paused = app.engine().map(|e| e.state().paused).unwrap_or(true);
    let icon = if paused { icons::PLAY } else { icons::PAUSE };

    if icon_button(
        ui,
        icon,
        tr!("Пауза / продолжить (пробел)", "Pause / play (space)"),
    ) {
        app.toggle_pause();
    }
}

/// Повтор файла по кругу.
///
/// Стоит рядом с паузой: включают его обычно там же, где разбирают
/// кусок по многу раз.
fn show_loop_button(app: &mut PithApp, ui: &mut egui::Ui) {
    let looping = app.is_looping();

    let hint = if looping {
        tr!("Не повторять файл", "Stop looping")
    } else {
        tr!("Повторять файл по кругу", "Loop file")
    };

    let color = if looping {
        theme::PANEL_ACCENT
    } else {
        theme::TEXT_PRIMARY
    };

    let response = ui
        .add_sized(
            BUTTON_SIZE,
            egui::Button::new(icons::LOOP.text().color(color)).frame(false),
        )
        .on_hover_text(hint);

    if crate::ui::clicked(&response) {
        app.toggle_looping();
    }
}

/// Закладка на текущем месте — то же, что клавиша T.
///
/// Клавишу знает не всякий, а метку по ходу просмотра ставят часто:
/// из неё потом вырезается отрезок.
fn show_bookmark_button(app: &mut PithApp, ui: &mut egui::Ui) {
    if icon_button(
        ui,
        icons::ADD,
        tr!(
            "Поставить закладку на текущем месте (T)",
            "Add bookmark at current position (T)"
        ),
    ) {
        app.add_bookmark_here();
    }
}

/// Поиск по субтитрам — рядом с закладкой.
///
/// Найденную реплику отмечают закладкой прямо из окна поиска, поэтому
/// кнопки стоят вместе.
fn show_search_button(app: &mut PithApp, ui: &mut egui::Ui) {
    if icon_button(
        ui,
        icons::SEARCH,
        tr!("Поиск по субтитрам (Ctrl+F)", "Search subtitles (Ctrl+F)"),
    ) {
        app.open_search();
    }
}

/// Кнопка полноэкранного режима.
fn show_fullscreen_button(app: &mut PithApp, ui: &mut egui::Ui) {
    let (icon, hint) = if app.is_fullscreen() {
        (
            icons::RESTORE,
            tr!("Выйти из полного экрана (F)", "Leave fullscreen (F)"),
        )
    } else {
        (
            icons::FULLSCREEN,
            tr!("Во весь экран (F)", "Fullscreen (F)"),
        )
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
            .on_hover_text(tr!("Ищу чёрные поля…", "Looking for black bars…"));
        return;
    }

    let (icon, hint) = if app.is_cropped() {
        (
            icons::FIT_ORIGINAL,
            tr!("Вернуть чёрные поля", "Restore black bars"),
        )
    } else {
        (
            icons::FIT_SCREEN,
            tr!(
                "Растянуть на весь экран: FFmpeg найдёт чёрные поля и уберёт их",
                "Fill the screen: FFmpeg finds the black bars and crops them"
            ),
        )
    };

    if icon_button(ui, icon, hint) {
        app.toggle_crop();
    }
}

/// Кнопка со значком: одинаковый размер, без рамки поверх видео.
pub(super) fn icon_button(ui: &mut egui::Ui, icon: icons::Icon, hint: &str) -> bool {
    let response = ui
        .add_sized(
            BUTTON_SIZE,
            egui::Button::new(icon.text().color(theme::TEXT_PRIMARY)).frame(false),
        )
        .on_hover_text(hint);

    crate::ui::clicked(&response)
}

/// Текущее место — слева от полосы перемотки.
///
/// Позиция дополняется до вида длительности: иначе надпись росла бы на
/// каждом лишнем разряде, а полоса перемотки, которая занимает весь
/// остаток строки, на столько же укорачивалась бы прямо во время просмотра.
fn show_position(app: &PithApp, ui: &mut egui::Ui) {
    let duration = app.engine().map(|e| e.state().duration).unwrap_or_default();

    // То же место, что показывает ползунок: пока его тянут, mpv отдаёт
    // старую позицию, и надпись со временем застывала, хотя полоса уже
    // ушла в другой конец фильма.
    let position = app.display_position();

    time_label::show(ui, &format_time_padded(position, duration))
        .on_hover_text(tr!("Сколько уже прошло", "Time played"));
}

/// Длительность файла — справа от полосы перемотки.
fn show_duration(app: &PithApp, ui: &mut egui::Ui) {
    let duration = app.engine().map(|e| e.state().duration).unwrap_or_default();

    time_label::show(ui, &format_time(duration))
        .on_hover_text(tr!("Длительность файла", "File duration"));
}

fn show_timeline(app: &mut PithApp, ui: &mut egui::Ui, width: f32) {
    let duration = app.engine().map(|e| e.state().duration).unwrap_or_default();

    // Пока идёт перемотка, показываем желаемое место, а не то, до которого
    // mpv уже добрался: иначе ползунок прыгает назад под пальцем.
    let position = app.display_position();

    let fragments = app.fragment_ranges();
    let response = timeline::show(ui, position, duration, width, &fragments);

    match (response.dragging, response.seek_to) {
        // Ведут ползунок: быстрая перемотка по опорным кадрам.
        (true, Some(target)) => app.scrub_to(target),
        // Отпустили или щёлкнули — доводим точно и возвращаем воспроизведение.
        (false, Some(target)) => {
            app.resume_after_scrub();
            app.seek_absolute(target);
        }
        _ => {}
    }

    // Подсказка нужна и при простом наведении: посмотреть, что в этом
    // месте фильма, часто хочется, не перематывая туда.
    match response.hovered_time {
        Some(time) => {
            app.request_preview(time);
            preview::show(app, ui.ctx(), &response, duration);
        }
        None => app.clear_preview(),
    }
}
