//! Строка списка отрезков: время, название и кнопки напротив него.
//!
//! Отдельно от самой панели: у строки своя разметка — кнопки справа
//! ровным столбцом, время в колонке постоянной ширины — и держать её
//! вперемешку с устройством панели незачем.

use crate::theme;
use crate::tr;
use crate::ui::bookmarks::PanelActions;
use crate::ui::{format_time, icons};

/// Ширина колонки со временем: столько занимает «0:00:00».
const TIME_COLUMN: f32 = 52.0;

/// Одна строка списка: время, название, кнопки.
///
/// Кнопки выкладываются справа налево первыми, поэтому стоят ровным
/// столбцом независимо от длины реплики. Время — в колонке постоянной
/// ширины, иначе названия начинались бы вразнобой.
pub(super) fn show(
    ui: &mut egui::Ui,
    bookmark: &pith_store::TimeBookmark,
    others: &[String],
    can_extract: bool,
    removal_left: Option<f64>,
    ocr: Option<(f32, &str)>,
    actions: &mut PanelActions,
) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 6.0;

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            show_row_buttons(ui, bookmark, can_extract, removal_left, actions);

            // Остаток строки — под время и название.
            ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                show_row_text(ui, bookmark, others, actions);
            });
        });
    });

    // Идёт распознавание текста с кадра: полоса хода и стадия.
    if let Some((progress, stage)) = ocr {
        ui.add(
            egui::ProgressBar::new(progress)
                .desired_height(10.0)
                .text(egui::RichText::new(stage).size(10.0)),
        );
    }
}

/// Кнопки строки: убрать, переименовать, вырезать, скопировать название.
fn show_row_buttons(
    ui: &mut egui::Ui,
    bookmark: &pith_store::TimeBookmark,
    can_extract: bool,
    removal_left: Option<f64>,
    actions: &mut PanelActions,
) {
    // Пока идёт таймер, вместо корзины — оставшиеся секунды красным;
    // нажатие отменяет удаление.
    let delete = match removal_left {
        Some(left) => egui::Button::new(
            egui::RichText::new(format!("{}", left.ceil() as i64))
                .color(theme::ERROR)
                .strong(),
        ),
        None => egui::Button::new(icons::DELETE.text()),
    }
    .frame(false);

    let hint = match removal_left {
        Some(left) => tr!(
            format!(
                "Удалится через {} с — нажмите, чтобы отменить",
                left.ceil() as i64
            ),
            format!("Removed in {} s — click to cancel", left.ceil() as i64)
        )
        .to_string(),
        None => tr!("Убрать (через 5 секунд)", "Remove (in 5 seconds)").to_string(),
    };

    if ui.add(delete).on_hover_text(hint).clicked() {
        actions.remove = Some(bookmark.time_ms);
    }

    if ui
        .add(egui::Button::new(icons::EDIT.text()).frame(false))
        .on_hover_text(tr!("Переименовать", "Rename"))
        .clicked()
    {
        actions.rename = Some(bookmark.time_ms);
    }

    // Вырезать только этот отрезок: резать весь список ради одной
    // метки — это лишние файлы и минуты ожидания. Ножницы зеленеют
    // вслед за названием, когда закладке приписан актёр.
    if ui
        .add_enabled(
            can_extract,
            egui::Button::new(icons::CUT.text().color(mark_color(bookmark))).frame(false),
        )
        .on_hover_text(tr!("Вырезать этот отрезок", "Cut this fragment"))
        .on_disabled_hover_text(tr!(
            "Нужен ffmpeg.exe рядом с плеером",
            "ffmpeg.exe must sit next to the player"
        ))
        .clicked()
    {
        actions.extract_one = Some(bookmark.time_ms);
    }

    // Название закладки — обычно реплика субтитров, и её чаще всего
    // и хотят перенести в заметки. В строке название урезано, так что
    // переписать его глазами всё равно не выйдет.
    let name = bookmark.name.clone().unwrap_or_default();

    if ui
        .add_enabled(
            !name.trim().is_empty(),
            egui::Button::new(icons::COPY.text()).frame(false),
        )
        .on_hover_text(tr!("Скопировать название", "Copy the name"))
        .on_disabled_hover_text(tr!("Название пустое", "The name is empty"))
        .clicked()
    {
        actions.copy_name = Some(name);
    }
}

/// Цвет закладки в панели: зелёный, когда актёр приписан.
fn mark_color(bookmark: &pith_store::TimeBookmark) -> egui::Color32 {
    match bookmark.actor {
        Some(_) => theme::ACTOR_MARK,
        None => theme::PANEL_ACCENT,
    }
}

/// Время и название закладки: нажатие переводит воспроизведение к метке.
fn show_row_text(
    ui: &mut egui::Ui,
    bookmark: &pith_store::TimeBookmark,
    others: &[String],
    actions: &mut PanelActions,
) {
    ui.allocate_ui_with_layout(
        egui::vec2(TIME_COLUMN, ui.available_height()),
        egui::Layout::left_to_right(egui::Align::Center),
        |ui| {
            ui.label(
                egui::RichText::new(format_time(bookmark.seconds()))
                    .color(theme::PANEL_MUTED)
                    .monospace(),
            );
        },
    );

    let title = bookmark.name.clone().unwrap_or_default();
    let color = match &bookmark.actor {
        Some(_) => theme::ACTOR_MARK,
        None => theme::TEXT_PRIMARY,
    };

    let row = ui
        .add(
            egui::Label::new(egui::RichText::new(title).color(color))
                .truncate()
                .sense(egui::Sense::click()),
        )
        .on_hover_cursor(egui::CursorIcon::PointingHand);

    if let Some(actor) = &bookmark.actor {
        row.clone().on_hover_text(actor);
    }

    if row.clicked() {
        actions.jump_to = Some(bookmark.seconds());
    }

    // Двойной щелчок по тексту — окно названия и актёра, как карандаш.
    if row.double_clicked() {
        actions.rename = Some(bookmark.time_ms);
    }

    row.context_menu(|ui| {
        show_move_menu(ui, bookmark.time_ms, others, actions);
    });
}

/// Меню переноса закладки в другой список.
fn show_move_menu(ui: &mut egui::Ui, time_ms: i64, others: &[String], actions: &mut PanelActions) {
    if others.is_empty() {
        ui.label(
            egui::RichText::new(tr!("Других списков нет", "No other lists"))
                .color(theme::TEXT_SECONDARY),
        );
        return;
    }

    ui.label(
        egui::RichText::new(tr!("Перенести в список", "Move to list")).color(theme::TEXT_SECONDARY),
    );

    for name in others {
        if ui.button(crate::i18n::list_name(name)).clicked() {
            actions.move_to = Some((time_ms, name.clone()));
            ui.close();
        }
    }
}
