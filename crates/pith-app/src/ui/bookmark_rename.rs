//! Окно переименования закладки — отдельное окно системы.
//!
//! Как окна актёров и выгрузки: его двигают куда угодно, хоть на второй
//! экран, и открывается оно там же, где его закрыли.

use crate::app::PithApp;
use crate::theme;
use crate::tr;

/// Размер окна при первом показе.
const DEFAULT_SIZE: [f32; 2] = [440.0, 270.0];

/// Наименьший размер.
const MIN_SIZE: [f32; 2] = [320.0, 220.0];

/// Отступ содержимого от краёв.
const PADDING: i8 = 14;

pub fn show(app: &mut PithApp, ctx: &egui::Context) {
    if app.bookmark_rename().is_none() {
        return;
    }

    let viewport = app.place_rename_window(
        egui::ViewportBuilder::default()
            .with_title(tr!("Название закладки", "Bookmark name"))
            .with_min_inner_size(MIN_SIZE)
            .with_always_on_top(),
        DEFAULT_SIZE,
    );

    let id = egui::ViewportId::from_hash_of("bookmark_rename");
    let mut apply = false;
    let mut cancel = false;

    ctx.show_viewport_immediate(id, viewport, |ctx, _class| {
        app.track_rename_window(ctx);

        egui::CentralPanel::default()
            .frame(
                egui::Frame::NONE
                    .fill(theme::PANEL_CARD)
                    .inner_margin(egui::Margin::same(PADDING)),
            )
            .show(ctx, |ui| {
                let (a, c) = show_body(app, ui);
                apply |= a;
                cancel |= c;
            });

        apply |= ctx.input(|i| i.key_pressed(egui::Key::Enter));
        cancel |= ctx.input(|i| i.key_pressed(egui::Key::Escape))
            || ctx.input(|i| i.viewport().close_requested());
    });

    if apply {
        app.apply_bookmark_rename();
    } else if cancel {
        app.close_bookmark_rename();
    }
}

/// Поля и кнопки. Возвращает «сохранить» и «отмена».
fn show_body(app: &mut PithApp, ui: &mut egui::Ui) -> (bool, bool) {
    let mut autopaste = app.settings().rename_autopaste;
    let mut apply = false;
    let mut cancel = false;

    {
        let Some(fields) = app.bookmark_rename_mut() else {
            return (false, false);
        };

        let name_id = egui::Id::new("bookmark_name_field");
        let actor_id = egui::Id::new("bookmark_actor_field");

        let name = ui.add(
            egui::TextEdit::singleline(&mut fields.name)
                .id(name_id)
                .desired_width(f32::INFINITY)
                .hint_text(tr!("Реплика из фильма", "A line from the film")),
        );

        // Актёр отдельным полем: в списке отрезков его не видно, а имя
        // закладки идёт в имя вырезанного файла — мешать их нельзя.
        ui.add_space(8.0);
        ui.label(
            egui::RichText::new(tr!("Актёр", "Actor"))
                .color(theme::TEXT_SECONDARY)
                .small(),
        );
        let actor = ui.add(
            egui::TextEdit::singleline(&mut fields.actor)
                .id(actor_id)
                .desired_width(f32::INFINITY)
                .hint_text(tr!("Имя (роль)", "Name (role)")),
        );

        // Фокус при открытии: пустое название — ему, можно печатать сразу;
        // заполненное — актёру, с курсором в конце.
        if fields.focus_pending {
            fields.focus_pending = false;

            if fields.focus_actor {
                actor.request_focus();
                put_cursor_at_end(ui.ctx(), actor_id, &fields.actor);
            } else {
                name.request_focus();
                put_cursor_at_end(ui.ctx(), name_id, &fields.name);
            }
        }
    }

    ui.add_space(8.0);

    if ui
        .checkbox(
            &mut autopaste,
            tr!(
                "Вставлять из буфера обмена автоматически",
                "Paste from the clipboard automatically"
            ),
        )
        .changed()
    {
        app.set_rename_autopaste(autopaste);
    }

    ui.add_space(6.0);
    ui.label(
        egui::RichText::new(tr!(
            "Название становится именем вырезанного файла. \
             Пустое поле вернёт подпись по времени.",
            "The name becomes the file name of the cut fragment. \
             Leave it empty to fall back to the timestamp.",
        ))
        .color(theme::TEXT_DISABLED)
        .small(),
    );

    ui.add_space(12.0);
    ui.horizontal(|ui| {
        apply = ui.button(tr!("Сохранить", "Save")).clicked();
        cancel = ui.button(tr!("Отмена", "Cancel")).clicked();
    });

    (apply, cancel)
}

/// Ставит курсор поля в конец текста.
fn put_cursor_at_end(ctx: &egui::Context, id: egui::Id, text: &str) {
    if let Some(mut state) = egui::TextEdit::load_state(ctx, id) {
        let end = egui::text::CCursor::new(text.chars().count());
        state
            .cursor
            .set_char_range(Some(egui::text::CCursorRange::one(end)));
        state.store(ctx, id);
    }
}
