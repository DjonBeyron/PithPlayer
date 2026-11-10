//! Ручной режим окна актёров: название картины, форма «добавить человека»
//! и действия над вписанными людьми. База фильмов и ключ не нужны.

use crate::app::PithApp;
use crate::theme;
use crate::tr;

/// Переключатель «База фильмов / Вручную» — вверху окна.
pub(super) fn show_mode_switch(app: &mut PithApp, ui: &mut egui::Ui) {
    let mut manual = app.manual_mode();
    let before = manual;

    ui.horizontal(|ui| {
        ui.selectable_value(&mut manual, false, tr!("База фильмов", "Movie database"));
        ui.selectable_value(&mut manual, true, tr!("Вручную", "Manual"));
    });

    if manual != before {
        app.set_manual_mode(manual);
    }

    ui.add_space(8.0);
}

/// Название картины и форма добавления. Список людей рисует общий код окна.
pub(super) fn show_manual(app: &mut PithApp, ui: &mut egui::Ui) {
    app.sync_manual_title();

    ui.label(
        egui::RichText::new(tr!("Фильм", "Film"))
            .color(theme::TEXT_SECONDARY)
            .size(12.0),
    );

    let title = ui.add(
        egui::TextEdit::singleline(&mut app.manual_form().title)
            .desired_width(f32::INFINITY)
            .hint_text(tr!("Название фильма", "Film title")),
    );

    if title.lost_focus() {
        app.save_manual_title();
    }

    ui.add_space(8.0);
    show_add_form(app, ui);
    ui.add_space(8.0);
}

/// Имя, роль и фотография нового человека.
fn show_add_form(app: &mut PithApp, ui: &mut egui::Ui) {
    ui.label(
        egui::RichText::new(tr!("Добавить персонажа", "Add a character"))
            .color(theme::TEXT_SECONDARY)
            .size(12.0),
    );

    let mut add = false;

    {
        let form = app.manual_form();

        let name = ui.add(
            egui::TextEdit::singleline(&mut form.name)
                .desired_width(f32::INFINITY)
                .hint_text(tr!("Имя", "Name")),
        );
        add |= name.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));

        ui.add(
            egui::TextEdit::singleline(&mut form.role)
                .desired_width(f32::INFINITY)
                .hint_text(tr!("Роль (необязательно)", "Role (optional)")),
        );
    }

    let mut paste = false;
    let mut file = false;

    ui.horizontal_wrapped(|ui| {
        paste = ui
            .button(tr!("Фото из буфера", "Photo from clipboard"))
            .clicked();
        file = ui
            .button(tr!("Фото из файла…", "Photo from file…"))
            .clicked();

        if app.manual_form().photo.is_some() {
            ui.label(
                egui::RichText::new(tr!("фото выбрано", "photo chosen"))
                    .color(theme::SUCCESS)
                    .size(12.0),
            );
        }
    });

    if paste && let Some(photo) = app.photo_from_clipboard() {
        app.manual_form().photo = Some(photo);
    }
    if file && let Some(photo) = app.photo_from_file() {
        app.manual_form().photo = Some(photo);
    }

    let ready = !app.manual_form().name.trim().is_empty();
    add |= ui
        .add_enabled(ready, egui::Button::new(tr!("Добавить", "Add")))
        .clicked();

    if add {
        app.manual_add_member();
    }
}

/// Пункты меню строки вписанного человека: фото и удаление.
pub(super) fn show_member_menu(app: &mut PithApp, ui: &mut egui::Ui, id: i64) {
    if ui
        .button(tr!("Фото из буфера", "Photo from clipboard"))
        .clicked()
    {
        if let Some(photo) = app.photo_from_clipboard() {
            app.manual_set_photo(id, Some(photo));
        }
        ui.close();
    }

    if ui
        .button(tr!("Фото из файла…", "Photo from file…"))
        .clicked()
    {
        if let Some(photo) = app.photo_from_file() {
            app.manual_set_photo(id, Some(photo));
        }
        ui.close();
    }

    if ui.button(tr!("Убрать из списка", "Remove")).clicked() {
        app.manual_remove_member(id);
        ui.close();
    }
}
