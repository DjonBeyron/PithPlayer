//! Переключатель списков отрезков и диалог их настройки (PLAN.md §6.5).

use crate::app::{ListDialog, PithApp};
use crate::theme;
use crate::tr;

/// Ширина диалога.
const DIALOG_WIDTH: f32 = 380.0;

/// Пределы длительности и отступа, секунды.
const MAX_DURATION: u32 = 600;
const MAX_BUFFER: u32 = 120;

/// Отступ в начале названия списка — под кружок его цвета.
const DOT_SPACE: &str = "      ";

/// Радиус кружка и его отступ от левого края строки.
const DOT_RADIUS: f32 = 4.5;
const DOT_OFFSET: f32 = 14.0;

/// Содержимое меню выбора списка.
///
/// Саму плашку рисует шапка панели: там она согласована с остальными
/// её частями, а здесь остаются только пункты.
pub fn show_choices(app: &mut PithApp, ui: &mut egui::Ui) {
    let names = app.list_names();
    let Some(active) = app.active_list_name() else {
        return;
    };

    ui.set_min_width(180.0);

    let mut chosen = None;

    for (index, name) in names.iter().enumerate() {
        // Отступ под кружок: без него точка липла к первой букве.
        let label = egui::RichText::new(format!("{DOT_SPACE}{name}")).color(theme::TEXT_PRIMARY);

        let response = ui.selectable_label(*name == active, label);

        // Кружок цвета рисуем кистью: знака ● в шрифте может не быть.
        let center = egui::pos2(response.rect.left() + DOT_OFFSET, response.rect.center().y);
        ui.painter()
            .circle_filled(center, DOT_RADIUS, theme::list_color(index));

        if response.clicked() {
            chosen = Some(name.clone());
            ui.close();
        }
    }

    if let Some(name) = chosen
        && name != active
    {
        app.switch_list(&name);
    }
}

/// Содержимое меню операций: создать, настроить, дублировать, удалить.
pub fn show_actions(app: &mut PithApp, ui: &mut egui::Ui) {
    let mut action = None;
    let only_one = app.list_names().len() <= 1;

    {
        ui.set_min_width(180.0);

        if ui.button(tr!("Новый список…", "New list…")).clicked() {
            action = Some(Action::New);
            ui.close();
        }
        if ui
            .button(tr!("Настроить список…", "List settings…"))
            .clicked()
        {
            action = Some(Action::Settings);
            ui.close();
        }
        if ui
            .button(tr!("Дублировать список", "Duplicate list"))
            .clicked()
        {
            action = Some(Action::Duplicate);
            ui.close();
        }

        ui.separator();

        if ui
            .add_enabled(
                !only_one,
                egui::Button::new(tr!("Удалить список", "Delete list")),
            )
            .on_disabled_hover_text(tr!(
                "Последний список удалить нельзя",
                "The last list cannot be deleted"
            ))
            .clicked()
        {
            action = Some(Action::Delete);
            ui.close();
        }

        ui.separator();

        if ui
            .button(tr!("Настройки нарезки…", "Fragment settings…"))
            .on_hover_text(tr!(
                "Значения по умолчанию для новых списков",
                "Defaults for new lists"
            ))
            .clicked()
        {
            action = Some(Action::FragmentSettings);
            ui.close();
        }
    }

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
        .order(egui::Order::Foreground)
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
        .show(ctx, |ui| {
            ui.set_width(DIALOG_WIDTH);

            let Some(fields) = app.list_dialog_mut() else {
                return;
            };

            ui.label(
                egui::RichText::new(tr!("Имя списка", "List name")).color(theme::TEXT_SECONDARY),
            );
            let name = ui.add(
                egui::TextEdit::singleline(&mut fields.name)
                    .desired_width(f32::INFINITY)
                    .hint_text(tr!("Диалоги", "Dialogue")),
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
                apply |= ui.button(tr!("Сохранить", "Save")).clicked();
                cancel |= ui.button(tr!("Отмена", "Cancel")).clicked();
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

/// Подтверждение очистки списка.
///
/// Отдельное окно, а не сразу действие: закладки собираются по ходу
/// просмотра всего фильма, и вернуть их нечем.
pub fn show_clear_prompt(app: &mut PithApp, ctx: &egui::Context) {
    let Some((name, count)) = app.clear_list_prompt() else {
        return;
    };

    let mut confirm = false;
    let mut cancel = false;

    egui::Window::new(tr!("Очистить список?", "Clear the list?"))
        .order(egui::Order::Foreground)
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
        .show(ctx, |ui| {
            ui.set_width(DIALOG_WIDTH);

            ui.label(
                egui::RichText::new(tr!(
                    format!(
                        "Из списка «{name}» будут убраны все закладки: {count}. \
                         Сам список останется, вырезанные отрезки не тронутся."
                    ),
                    format!(
                        "All {count} bookmarks will be removed from «{name}». \
                         The list itself stays, cut fragments are untouched."
                    )
                ))
                .color(theme::TEXT_SECONDARY),
            );

            ui.add_space(14.0);
            ui.horizontal(|ui| {
                confirm |= ui.button(tr!("Очистить", "Clear")).clicked();
                cancel |= ui.button(tr!("Отмена", "Cancel")).clicked();
            });
        });

    cancel |= ctx.input(|i| i.key_pressed(egui::Key::Escape));

    // Enter не принимаем: потерять полсотни меток случайным нажатием
    // было бы обидно.
    if confirm {
        app.confirm_clear_list();
    } else if cancel {
        app.cancel_clear_list();
    }
}

/// Длительность фрагмента и отступ назад от метки.
fn show_numbers(fields: &mut ListDialog, ui: &mut egui::Ui) {
    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new(tr!("Длительность, с", "Duration, s")).color(theme::TEXT_SECONDARY),
        );
        ui.add(egui::DragValue::new(&mut fields.duration_sec).range(1..=MAX_DURATION));

        ui.add_space(12.0);

        ui.label(
            egui::RichText::new(tr!("Отступ назад, с", "Lead-in, s")).color(theme::TEXT_SECONDARY),
        );
        ui.add(egui::DragValue::new(&mut fields.buffer_sec).range(0..=MAX_BUFFER));
    });

    ui.label(
        egui::RichText::new(tr!(
            "Отрезок начинается за «отступ» до метки",
            "The fragment starts one lead-in before the mark"
        ))
        .color(theme::TEXT_DISABLED)
        .small(),
    );
}

/// Своя папка вывода списка.
fn show_output_dir(fields: &ListDialog, ui: &mut egui::Ui, pick: &mut bool, clear: &mut bool) {
    ui.label(
        egui::RichText::new(tr!("Папка вывода", "Output folder")).color(theme::TEXT_SECONDARY),
    );

    ui.horizontal(|ui| {
        *pick |= ui.button(tr!("Выбрать…", "Choose…")).clicked();

        if fields.output_dir.is_some() {
            *clear |= ui
                .button(tr!("Общая", "Shared"))
                .on_hover_text(tr!(
                    "Складывать в общую папку из настроек",
                    "Use the shared folder from settings"
                ))
                .clicked();
        }
    });

    let text = match &fields.output_dir {
        Some(dir) => dir.to_string_lossy().to_string(),
        None => tr!("Общая папка из настроек", "Shared folder from settings").to_string(),
    };

    ui.label(
        egui::RichText::new(text)
            .color(theme::TEXT_DISABLED)
            .small(),
    );
}
