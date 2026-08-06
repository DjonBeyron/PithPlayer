//! Диалог общих настроек нарезки (порт `RecordingSettingsForm` из v4).
//!
//! Вид собран по макету: шапка с названием, разделы карточками, тумблеры
//! вместо галочек и подвал с двумя кнопками. Прежнее окно было сплошной
//! лентой подписей и пояснений — в ней приходилось вчитываться, чтобы
//! понять, где кончается одна настройка и начинается другая.

use crate::app::{FragmentSettingsDialog, PithApp};
use crate::theme;
use crate::tr;
use crate::ui::dialog;

/// Ширина окна.
const WIDTH: f32 = 560.0;

/// Скругление окна.
const WINDOW_RADIUS: u8 = 12;

/// Отступ содержимого от краёв окна.
const SIDE_PADDING: i8 = 20;

/// Промежуток между разделами.
const SECTION_GAP: f32 = 20.0;

/// Пределы длительности и отступа, секунды.
const MAX_DURATION: u32 = 600;
const MAX_BUFFER: u32 = 120;

/// Что пользователь сделал с окном.
#[derive(Default)]
struct Actions {
    apply: bool,
    cancel: bool,
    pick_dir: bool,
    clear_dir: bool,
}

pub fn show(app: &mut PithApp, ctx: &egui::Context) {
    if app.fragment_settings_dialog().is_none() {
        return;
    }

    let mut actions = Actions::default();

    // Слой выше панели управления: иначе её кнопки — пауза, полный экран —
    // рисуются поверх окна и перехватывают нажатия по нему.
    egui::Window::new(tr!("Настройки нарезки", "Fragment settings"))
        .order(egui::Order::Foreground)
        // Заголовок рисуем сами: полоса egui с кнопкой сворачивания
        // выбивается из макета, а сворачивать тут нечего.
        .title_bar(false)
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
        .frame(window_frame())
        .show(ctx, |ui| {
            ui.set_width(WIDTH);

            show_header(ui);
            dialog::divider(ui);
            show_body(app, ui, &mut actions);
            dialog::divider(ui);
            show_footer(ui, &mut actions);
        });

    actions.apply |= ctx.input(|i| i.key_pressed(egui::Key::Enter));
    actions.cancel |= ctx.input(|i| i.key_pressed(egui::Key::Escape));

    apply_actions(app, actions);
}

/// Рамка окна: подложка, тонкая обводка и мягкая тень под ней.
fn window_frame() -> egui::Frame {
    egui::Frame::NONE
        .fill(theme::DIALOG_BG)
        .stroke(egui::Stroke::new(1.0, theme::DIALOG_BORDER))
        .corner_radius(WINDOW_RADIUS)
        .shadow(egui::epaint::Shadow {
            offset: [0, 10],
            blur: 30,
            spread: 0,
            color: egui::Color32::from_black_alpha(140),
        })
        // Отступы задают шапка, тело и подвал по отдельности: черта между
        // ними должна идти от края до края.
        .inner_margin(0)
}

fn show_header(ui: &mut egui::Ui) {
    egui::Frame::NONE
        .inner_margin(egui::Margin {
            left: SIDE_PADDING,
            right: SIDE_PADDING,
            top: 14,
            bottom: 14,
        })
        .show(ui, |ui| {
            ui.label(
                egui::RichText::new(tr!("Настройки нарезки", "Fragment settings"))
                    .color(theme::DIALOG_TEXT)
                    .size(19.0)
                    .strong(),
            );
        });
}

fn show_body(app: &mut PithApp, ui: &mut egui::Ui, actions: &mut Actions) {
    let hint = app.fragment_settings_hint();

    egui::Frame::NONE
        .inner_margin(egui::Margin::symmetric(SIDE_PADDING, 18))
        .show(ui, |ui| {
            if let Some(hint) = hint {
                ui.label(egui::RichText::new(hint).color(theme::ERROR));
                ui.add_space(12.0);
            }

            let Some(fields) = app.fragment_settings_dialog_mut() else {
                return;
            };

            show_defaults(fields, ui);
            ui.add_space(SECTION_GAP);

            show_output_dir(fields, ui, actions);
            ui.add_space(SECTION_GAP);

            show_advanced(fields, ui);
        });
}

/// Длительность отрезка и отступ назад от метки.
fn show_defaults(fields: &mut FragmentSettingsDialog, ui: &mut egui::Ui) {
    dialog::section(
        ui,
        tr!(
            "Значения по умолчанию для новых списков отрезков",
            "Defaults for new fragment lists"
        ),
    );

    dialog::card(ui, |ui| {
        ui.horizontal(|ui| {
            let column = ui.available_width() / 2.0;

            ui.vertical(|ui| {
                ui.set_width(column);
                dialog::label(ui, tr!("Длительность, с", "Duration, s"));
                ui.add_space(6.0);
                dialog::number(ui, &mut fields.duration_sec, 1..=MAX_DURATION);
            });

            ui.vertical(|ui| {
                dialog::label(ui, tr!("Отступ назад, с", "Lead-in, s"));
                ui.add_space(6.0);
                dialog::number(ui, &mut fields.buffer_sec, 0..=MAX_BUFFER);
            });
        });

        ui.add_space(12.0);
        dialog::divider(ui);
        ui.add_space(10.0);

        dialog::hint(
            ui,
            tr!(
                "Закладка на 00:10 при длительности 5 с и отступе 3 с даёт отрезок 00:07—00:12",
                "A bookmark at 00:10 with duration 5 s and lead-in 3 s gives 00:07—00:12"
            ),
        );
    });
}

/// Куда складывать вырезанные отрезки.
fn show_output_dir(fields: &FragmentSettingsDialog, ui: &mut egui::Ui, actions: &mut Actions) {
    dialog::section(ui, tr!("Папка вывода", "Output folder"));

    dialog::card(ui, |ui| {
        ui.horizontal(|ui| {
            actions.pick_dir |= dialog::card_button(ui, tr!("Выбрать…", "Choose…")).clicked();

            if fields.output_dir.is_some() {
                actions.clear_dir |=
                    dialog::card_button(ui, tr!("Рядом с видео", "Next to the video"))
                        .on_hover_text(tr!(
                            "Складывать отрезки в папку исходного файла",
                            "Put fragments into the source file folder"
                        ))
                        .clicked();
            }
        });

        ui.add_space(12.0);

        let dir = match &fields.output_dir {
            Some(dir) => dir.to_string_lossy().to_string(),
            None => tr!("Рядом с исходным файлом", "Next to the source file").to_string(),
        };

        dialog::path_box(ui, &dir);
    });
}

/// Звук и способ нарезки — тумблерами.
fn show_advanced(fields: &mut FragmentSettingsDialog, ui: &mut egui::Ui) {
    dialog::section(ui, tr!("Дополнительно", "Advanced"));

    dialog::card(ui, |ui| {
        dialog::toggle_row(
            ui,
            &mut fields.audio_aac,
            tr!(
                "Звук в AAC — для Premiere Pro и After Effects",
                "AAC audio — for Premiere Pro and After Effects"
            ),
            tr!(
                "Монтажные программы Adobe не читают EAC3, DTS и подобные дорожки: \
                 отрезок открывается, но звука в нём для программы нет. AAC понимают \
                 все. Видео при этом копируется без перекодирования.",
                "Adobe editors cannot read EAC3, DTS and similar tracks: the fragment \
                 opens, but has no sound for them. Everyone understands AAC. Video is \
                 still copied without re-encoding."
            ),
        );

        ui.add_space(14.0);
        dialog::divider(ui);
        ui.add_space(14.0);

        dialog::toggle_row(
            ui,
            &mut fields.reencode,
            tr!(
                "Перекодировать вместо перепаковки",
                "Re-encode instead of remuxing"
            ),
            tr!(
                "Перепаковка быстрее в десятки раз и не теряет качества, но начинает \
                 отрезок с ближайшего опорного кадра. Перекодирование ставит старт \
                 точно по метке и выручает, когда монтажная программа не принимает \
                 исходный кодек.",
                "Remuxing is dozens of times faster and loses no quality, but starts \
                 the fragment at the nearest keyframe. Re-encoding starts exactly at \
                 the mark and helps when the editor rejects the source codec."
            ),
        );
    });
}

fn show_footer(ui: &mut egui::Ui, actions: &mut Actions) {
    egui::Frame::NONE
        .inner_margin(egui::Margin::symmetric(SIDE_PADDING, 14))
        .show(ui, |ui| {
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                actions.apply |= dialog::accent_button(ui, tr!("Сохранить", "Save")).clicked();
                actions.cancel |= dialog::outline_button(ui, tr!("Отмена", "Cancel")).clicked();
            });
        });
}

/// Выбор папки и закрытие окна.
fn apply_actions(app: &mut PithApp, actions: Actions) {
    if actions.pick_dir
        && let Some(dir) = rfd::FileDialog::new().pick_folder()
        && let Some(fields) = app.fragment_settings_dialog_mut()
    {
        fields.output_dir = Some(dir);
    }

    if actions.clear_dir
        && let Some(fields) = app.fragment_settings_dialog_mut()
    {
        fields.output_dir = None;
    }

    if actions.apply {
        app.apply_fragment_settings();
    } else if actions.cancel {
        app.close_fragment_settings();
    }
}
