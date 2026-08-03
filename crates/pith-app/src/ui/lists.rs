//! Переключатель списков отрезков и диалог их настройки (PLAN.md §6.5).

use crate::app::{ListDialog, PithApp};
use crate::theme;

/// Ширина диалога.
const DIALOG_WIDTH: f32 = 380.0;

/// Пределы длительности и отступа, секунды.
const MAX_DURATION: u32 = 600;
const MAX_BUFFER: u32 = 120;

/// Строка выбора списка и кнопки операций над ним.
///
/// Возвращать действия наружу не требуется: панель рисуется по `&mut PithApp`,
/// и команды применяются сразу.
pub fn show_switcher(app: &mut PithApp, ui: &mut egui::Ui) {
    let names = app.list_names();
    let Some(active) = app.active_list_name() else {
        return;
    };

    let mut chosen = None;

    ui.horizontal(|ui| {
        // Кружок цвета: тем же цветом отрезки списка отмечены на полосе.
        let (rect, _) = ui.allocate_exact_size(egui::vec2(12.0, 12.0), egui::Sense::hover());
        ui.painter()
            .circle_filled(rect.center(), 5.0, app.active_list_color());

        egui::ComboBox::from_id_salt("bookmark_lists")
            .selected_text(&active)
            .width(200.0)
            .show_ui(ui, |ui| {
                for name in &names {
                    if ui.selectable_label(*name == active, name).clicked() {
                        chosen = Some(name.clone());
                    }
                }
            });

        show_actions_menu(app, ui);
    });

    if let Some(name) = chosen
        && name != active
    {
        app.switch_list(&name);
    }
}

/// Меню операций: создать, настроить, дублировать, удалить.
fn show_actions_menu(app: &mut PithApp, ui: &mut egui::Ui) {
    let mut action = None;
    let only_one = app.list_names().len() <= 1;

    ui.menu_button(crate::ui::icons::SETTINGS.text(), |ui| {
        ui.set_min_width(180.0);

        if ui.button("Новый список…").clicked() {
            action = Some(Action::New);
            ui.close();
        }
        if ui.button("Настроить список…").clicked() {
            action = Some(Action::Settings);
            ui.close();
        }
        if ui.button("Дублировать список").clicked() {
            action = Some(Action::Duplicate);
            ui.close();
        }

        ui.separator();

        if ui
            .add_enabled(!only_one, egui::Button::new("Удалить список"))
            .on_disabled_hover_text("Последний список удалить нельзя")
            .clicked()
        {
            action = Some(Action::Delete);
            ui.close();
        }

        ui.separator();

        if ui
            .button("Настройки нарезки…")
            .on_hover_text("Значения по умолчанию для новых списков")
            .clicked()
        {
            action = Some(Action::FragmentSettings);
            ui.close();
        }
    })
    .response
    .on_hover_text("Действия со списком");

    match action {
        Some(Action::New) => app.open_new_list_dialog(),
        Some(Action::Settings) => app.open_list_settings_dialog(),
        Some(Action::Duplicate) => app.duplicate_active_list(),
        Some(Action::Delete) => app.delete_active_list(),
        Some(Action::FragmentSettings) => app.open_fragment_settings(),
        None => {}
    }
}

enum Action {
    New,
    Settings,
    Duplicate,
    Delete,
    FragmentSettings,
}

/// Диалог создания и настройки списка.
pub fn show_dialog(app: &mut PithApp, ctx: &egui::Context) {
    let Some(dialog) = app.list_dialog().cloned() else {
        return;
    };

    let mut apply = false;
    let mut cancel = false;
    let mut pick_dir = false;
    let mut clear_dir = false;

    egui::Window::new(dialog.title())
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
        .show(ctx, |ui| {
            ui.set_width(DIALOG_WIDTH);

            let Some(fields) = app.list_dialog_mut() else {
                return;
            };

            ui.label(egui::RichText::new("Имя списка").color(theme::TEXT_SECONDARY));
            let name = ui.add(
                egui::TextEdit::singleline(&mut fields.name)
                    .desired_width(f32::INFINITY)
                    .hint_text("Диалоги"),
            );

            if fields.focus_pending {
                name.request_focus();
                fields.focus_pending = false;
            }

            ui.add_space(8.0);
            show_numbers(fields, ui);
            ui.add_space(8.0);

            show_output_dir(fields, ui, &mut pick_dir, &mut clear_dir);

            if let Some(error) = &fields.error {
                ui.add_space(6.0);
                ui.label(egui::RichText::new(error).color(theme::ERROR));
            }

            ui.add_space(12.0);
            ui.horizontal(|ui| {
                apply |= ui.button("Сохранить").clicked();
                cancel |= ui.button("Отмена").clicked();
            });
        });

    // Enter сохраняет, Escape закрывает — привычное поведение диалога.
    // Проверяем на уровне контекста: поле ввода забирает нажатия себе.
    apply |= ctx.input(|i| i.key_pressed(egui::Key::Enter));
    cancel |= ctx.input(|i| i.key_pressed(egui::Key::Escape));

    if pick_dir
        && let Some(dir) = rfd::FileDialog::new().pick_folder()
        && let Some(fields) = app.list_dialog_mut()
    {
        fields.output_dir = Some(dir);
    }

    if clear_dir && let Some(fields) = app.list_dialog_mut() {
        fields.output_dir = None;
    }

    if apply {
        app.apply_list_dialog();
    } else if cancel {
        app.close_list_dialog();
    }
}

/// Длительность фрагмента и отступ назад от метки.
fn show_numbers(fields: &mut ListDialog, ui: &mut egui::Ui) {
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new("Длительность, с").color(theme::TEXT_SECONDARY));
        ui.add(egui::DragValue::new(&mut fields.duration_sec).range(1..=MAX_DURATION));

        ui.add_space(12.0);

        ui.label(egui::RichText::new("Отступ назад, с").color(theme::TEXT_SECONDARY));
        ui.add(egui::DragValue::new(&mut fields.buffer_sec).range(0..=MAX_BUFFER));
    });

    ui.label(
        egui::RichText::new("Отрезок начинается за «отступ» до метки")
            .color(theme::TEXT_DISABLED)
            .small(),
    );
}

/// Своя папка вывода списка.
fn show_output_dir(fields: &ListDialog, ui: &mut egui::Ui, pick: &mut bool, clear: &mut bool) {
    ui.label(egui::RichText::new("Папка вывода").color(theme::TEXT_SECONDARY));

    ui.horizontal(|ui| {
        *pick |= ui.button("Выбрать…").clicked();

        if fields.output_dir.is_some() {
            *clear |= ui
                .button("Общая")
                .on_hover_text("Складывать в общую папку из настроек")
                .clicked();
        }
    });

    let text = match &fields.output_dir {
        Some(dir) => dir.to_string_lossy().to_string(),
        None => "Общая папка из настроек".to_string(),
    };

    ui.label(
        egui::RichText::new(text)
            .color(theme::TEXT_DISABLED)
            .small(),
    );
}
