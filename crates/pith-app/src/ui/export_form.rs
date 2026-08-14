//! Вопрос перед выгрузкой: название картины, её вид и что делать потом.
//!
//! Собран из карточек набора `dialog` — тем же, чем окна настроек: одно
//! решение в одной карточке, главная кнопка внизу. Само окно и отчёт —
//! в `ui/export.rs`.

use crate::app::{NameLanguage, PithApp};
use crate::tr;
use crate::ui::dialog;

/// Что нажали в форме.
pub enum FormAction {
    Start,
    Close,
}

pub fn show(app: &mut PithApp, ui: &mut egui::Ui) -> Option<FormAction> {
    dialog::hint(
        ui,
        tr!(
            "Выгружается активный список отрезков.",
            "The active fragment list will be exported."
        ),
    );
    ui.add_space(12.0);

    show_name_card(app, ui);
    ui.add_space(10.0);
    show_kind_card(app, ui);
    ui.add_space(10.0);
    show_after_card(app, ui);

    show_prepare_failure(app, ui);

    ui.add_space(14.0);

    show_buttons(ui)
}

/// Отказ подготовки — виден до нажатия «Выгрузить».
///
/// Плеер спрашивает Notion, ещё пока открыт вопрос, и об отсутствии доступа
/// узнаёт заранее. Показать это сразу честнее, чем дать нажать и ответить
/// отказом.
fn show_prepare_failure(app: &PithApp, ui: &mut egui::Ui) {
    let Some(why) = app.export_dialog().and_then(|d| d.prepare_failed.clone()) else {
        return;
    };

    ui.add_space(10.0);
    ui.add(
        egui::Label::new(
            egui::RichText::new(tr!(
                format!("Notion не отвечает: {why}"),
                format!("Notion is not responding: {why}")
            ))
            .color(crate::theme::ERROR)
            .size(12.0),
        )
        .wrap(),
    );
}

/// Карточка названия: откуда взять, на каком языке и что вышло.
fn show_name_card(app: &mut PithApp, ui: &mut egui::Ui) {
    dialog::card(ui, |ui| {
        let Some(dialog_state) = app.export_dialog_mut() else {
            return;
        };

        ui.checkbox(
            &mut dialog_state.from_file_name,
            tr!("Взять название из файла", "Take the name from the file"),
        );

        // Язык — только у готового названия: набранное руками уже на том
        // языке, на котором его набрали.
        if dialog_state.from_file_name {
            ui.add_space(10.0);
            show_language(app, ui);
        }

        ui.add_space(10.0);
        show_name_field(app, ui);
    });
}

/// Переключатель языка названия.
///
/// Русское берётся из состава картины, английское — из имени файла.
/// Состав не запрашивали — русского нет, и кнопка не нажимается.
fn show_language(app: &mut PithApp, ui: &mut egui::Ui) {
    let Some(state) = app.export_dialog() else {
        return;
    };

    let has_russian = state.has_russian();
    let language = state.language;

    let chosen = ui
        .add_enabled_ui(has_russian, |ui| {
            dialog::segmented(
                ui,
                &[
                    (tr!("Рус", "Rus"), language == NameLanguage::Ru),
                    (tr!("Англ", "Eng"), language == NameLanguage::En),
                ],
            )
        })
        .inner;

    if !has_russian {
        ui.add_space(6.0);
        dialog::hint(
            ui,
            tr!(
                "Русское название появится, когда сформируете список актёров.",
                "The Russian name appears once you build the cast."
            ),
        );
    }

    if let Some(index) = chosen
        && let Some(state) = app.export_dialog_mut()
    {
        state.language = if index == 0 {
            NameLanguage::Ru
        } else {
            NameLanguage::En
        };
    }
}

/// Поле названия: готовое показываем, набранное даём править.
fn show_name_field(app: &mut PithApp, ui: &mut egui::Ui) {
    let Some(state) = app.export_dialog_mut() else {
        return;
    };

    // Готовое название показываем, но не правим: чтобы менять его руками,
    // снимают галочку — иначе непонятно, откуда взялся текст.
    if state.from_file_name {
        dialog::value_box(ui, state.chosen_title());
        return;
    }

    dialog::text_field(
        ui,
        &mut state.title,
        tr!("Название картины", "Title of the film"),
    );
}

/// Карточка вида картины и того, что уйдёт в Notion.
fn show_kind_card(app: &mut PithApp, ui: &mut egui::Ui) {
    dialog::card(ui, |ui| {
        let Some(state) = app.export_dialog() else {
            return;
        };

        let kind = state.kind;
        let film_name = state.film_name();

        let chosen = dialog::segmented(
            ui,
            &[
                (tr!("Сериал", "Series"), kind == pith_notion::Kind::Series),
                (tr!("Фильм", "Movie"), kind == pith_notion::Kind::Movie),
            ],
        );

        ui.add_space(10.0);
        dialog::divider(ui);
        ui.add_space(10.0);

        dialog::hint(
            ui,
            &tr!(
                format!("В Notion попадёт: {film_name}"),
                format!("Notion will get: {film_name}")
            ),
        );

        if let Some(index) = chosen
            && let Some(state) = app.export_dialog_mut()
        {
            state.kind = if index == 0 {
                pith_notion::Kind::Series
            } else {
                pith_notion::Kind::Movie
            };
        }
    });
}

/// Карточка «что сделать после выгрузки».
fn show_after_card(app: &mut PithApp, ui: &mut egui::Ui) {
    let can_extract = app.can_extract();
    let known = app.known_words();

    dialog::card(ui, |ui| {
        let Some(state) = app.export_dialog_mut() else {
            return;
        };

        // Транскрипция — самая долгая часть работы, и человек должен знать,
        // за что платит ожиданием. Известные слова берутся из хранилища
        // мгновенно, поэтому их число здесь и названо.
        dialog::toggle_row(
            ui,
            &mut state.transcribe,
            tr!("Транскрипция реплик", "Transcribe the lines"),
            &tr!(
                format!(
                    "Слово ищется в словарях и запоминается: известных уже {known}. \
                     Новое стоит около секунды."
                ),
                format!(
                    "Words are looked up in dictionaries and remembered: {known} known \
                     already. A new one costs about a second."
                )
            ),
        );

        ui.add_space(12.0);
        dialog::divider(ui);
        ui.add_space(12.0);

        // Нарезка требует FFmpeg. Нет его — переключатель не нажимается,
        // и видно, почему.
        ui.add_enabled_ui(can_extract, |ui| {
            dialog::toggle_row(
                ui,
                &mut state.cut_after,
                tr!("Сразу вырезать отрезки", "Cut the fragments right away"),
                tr!(
                    "Нарезка начнётся сама, как только строки лягут в Notion.",
                    "Cutting starts as soon as the rows land in Notion."
                ),
            );
        });

        if !can_extract {
            ui.add_space(6.0);
            dialog::hint(
                ui,
                tr!(
                    "Нужен ffmpeg.exe рядом с плеером.",
                    "ffmpeg.exe must sit next to the player."
                ),
            );
        }
    });
}

/// Кнопки внизу окна.
fn show_buttons(ui: &mut egui::Ui) -> Option<FormAction> {
    let mut action = None;

    ui.horizontal(|ui| {
        if dialog::accent_button(ui, tr!("Выгрузить", "Export")).clicked() {
            action = Some(FormAction::Start);
        }
        if dialog::outline_button(ui, tr!("Отмена", "Cancel")).clicked() {
            action = Some(FormAction::Close);
        }
    });

    action
}
