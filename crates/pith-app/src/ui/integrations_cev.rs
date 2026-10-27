//! Выгрузка в CodeEditVideo: выбор места и токен GitHub.
//!
//! Кусок окна интеграций (`ui/integrations.rs`): отдельным файлом, чтобы
//! тот не разрастался.

use super::integrations::{show_field, show_hint, show_section, show_status};
use crate::app::{AccessStatus, IntegrationsState};
use crate::theme;
use crate::tr;

/// Где создаётся токен GitHub.
/// Список токенов пользователя.
const TOKENS_URL: &str = "https://github.com/settings/personal-access-tokens";

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

    if matches!(state.status, AccessStatus::Failed(_)) {
        show_token_help(ui);
    }

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

/// Где посмотреть токен и как завести новый.
///
/// GitHub показывает токен один раз, при создании, — потом посмотреть его
/// нельзя, можно только создать новый. Поэтому подсказка объясняет и то,
/// и другое: многим нужен именно «посмотреть».
pub(super) fn show_token_help(ui: &mut egui::Ui) {
    ui.add_space(8.0);
    ui.label(
        egui::RichText::new(tr!(
            "Нет доступа к базе. Что проверить:",
            "No access to the database. Check:"
        ))
        .color(theme::TEXT_PRIMARY)
        .size(12.0)
        .strong(),
    );
    ui.add(
        egui::Label::new(
            egui::RichText::new(tr!(
                "1. Список своих токенов: github.com/settings/personal-access-tokens. \
                 Сам токен там уже не показывается — GitHub отдаёт его один раз, \
                 при создании.\n\
                 2. Если свой потерян или истёк — «Generate new token». \
                 Repository access: Only select repositories → codeeditvideo-projects. \
                 Permissions → Contents: Read and write.\n\
                 3. Скопируйте токен сразу и вставьте в поле выше.\n\
                 4. Токен должен быть от аккаунта DjonBeyron, у которого есть доступ \
                 к репозиторию.",
                "1. Your tokens: github.com/settings/personal-access-tokens. \
                 The token value is not shown there again — GitHub shows it once, \
                 at creation.\n\
                 2. If it is lost or expired — «Generate new token». \
                 Repository access: Only select repositories → codeeditvideo-projects. \
                 Permissions → Contents: Read and write.\n\
                 3. Copy the token at once and paste it above.\n\
                 4. The token must belong to the DjonBeyron account that has access \
                 to the repository."
            ))
            .color(theme::PANEL_MUTED)
            .size(11.0),
        )
        .wrap(),
    );
    ui.hyperlink_to(
        egui::RichText::new(tr!("Открыть список токенов", "Open the token list")).size(11.0),
        TOKENS_URL,
    );
}
