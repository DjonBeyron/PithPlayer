//! Выгрузка в CodeEditVideo: выбор места и токен GitHub.
//!
//! Кусок окна интеграций (`ui/integrations.rs`): отдельным файлом, чтобы
//! тот не разрастался.

use super::integrations::{show_field, show_hint, show_section, show_status};
use crate::app::{AccessStatus, IntegrationsState};
use crate::theme;
use crate::tr;

/// Где создаётся токен GitHub.
const TOKEN_URL: &str = "https://github.com/settings/personal-access-tokens/new";

/// Переключатель «Notion / CodeEditVideo».
pub(super) fn show_target(state: &mut IntegrationsState, ui: &mut egui::Ui) {
    show_section(ui, tr!("Куда выгружать отрезки", "Where to export"));

    ui.horizontal(|ui| {
        ui.selectable_value(&mut state.cev.enabled, false, "Notion");
        ui.selectable_value(&mut state.cev.enabled, true, "CodeEditVideo");
    });
}

/// Токен GitHub и проверка доступа. Возвращает `true`, если нажали проверку.
pub(super) fn show_cev(state: &mut IntegrationsState, ui: &mut egui::Ui) -> bool {
    show_section(ui, "CodeEditVideo");
    show_hint(
        ui,
        tr!(
            "Отрезки пишутся в базу db/records.json репозитория \
             DjonBeyron/codeeditvideo-projects: реплика, фильм и актёр. \
             Нужен токен GitHub с правом записи (Contents: Read and write).",
            "Fragments are written to db/records.json of the \
             DjonBeyron/codeeditvideo-projects repository: line, film and \
             actor. A GitHub token with write access is required \
             (Contents: Read and write)."
        ),
    );
    ui.hyperlink_to(egui::RichText::new(TOKEN_URL).size(11.0), TOKEN_URL);

    ui.add_space(10.0);

    show_field(
        ui,
        tr!("Токен GitHub", "GitHub token"),
        "github_pat_…",
        &mut state.cev.token,
        true,
    );

    let working = state.status == AccessStatus::Working;
    let ready = !state.cev.token.trim().is_empty();

    let clicked = ui
        .add_enabled(
            !working && ready,
            egui::Button::new(tr!("Проверить доступ", "Check access")),
        )
        .on_disabled_hover_text(tr!("Вставьте токен", "Paste the token"))
        .clicked();

    show_status(&state.status, ui);

    ui.label(
        egui::RichText::new(tr!(
            "Проверка только читает базу и ничего в неё не пишет.",
            "The check only reads the database and writes nothing."
        ))
        .color(theme::PANEL_MUTED)
        .size(11.0),
    );

    clicked
}
