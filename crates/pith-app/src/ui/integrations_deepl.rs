//! Раздел DeepL в окне интеграций: токен, языки перевода и проверка ключа.
//!
//! Правится сразу и сохраняется в `deepl.json`: это не черновик, как
//! у Notion, — токену нечем вытеснить рабочее значение.

use super::integrations::{show_field, show_hint, show_section, show_status};
use crate::app::{AccessStatus, PithApp};
use crate::tr;

/// Где берётся ключ.
const KEY_URL: &str = "https://www.deepl.com/your-account/keys";

/// Возвращает `true`, если нажали проверку ключа.
pub(super) fn show_deepl(app: &mut PithApp, ui: &mut egui::Ui) -> bool {
    ui.add_space(16.0);
    show_section(ui, "DeepL");
    show_hint(
        ui,
        tr!(
            "Наведите мышь на реплику в кадре — рядом появится значок «A·Я»; \
             щелчок покажет перевод. Ключ бесплатного тарифа заканчивается на :fx.",
            "Hover a subtitle line — an «A·Я» icon appears; click it to see the \
             translation. A free-plan key ends with :fx."
        ),
    );
    ui.hyperlink_to(egui::RichText::new(KEY_URL).size(11.0), KEY_URL);
    ui.add_space(10.0);

    let before = app.deepl().settings.clone();

    show_field(
        ui,
        tr!("Ключ DeepL", "DeepL key"),
        "…:fx",
        &mut app.deepl().settings.token,
        true,
    );

    ui.label(
        egui::RichText::new(tr!("Переводить на", "Translate to"))
            .color(crate::theme::TEXT_SECONDARY)
            .size(12.0),
    );
    ui.add_space(2.0);

    ui.horizontal_wrapped(|ui| {
        for (code, ru, en) in pith_store::DEEPL_LANGUAGES {
            let targets = &mut app.deepl().settings.targets;
            let mut on = targets.iter().any(|t| t == code);

            if ui.checkbox(&mut on, tr!(ru, en)).changed() {
                if on {
                    targets.push(code.to_string());
                } else {
                    targets.retain(|t| t != code);
                }
            }
        }
    });

    ui.add_space(6.0);

    let working = matches!(app.deepl().check, Some(AccessStatus::Working));
    let ready = !app.deepl().settings.token.trim().is_empty();

    let clicked = ui
        .add_enabled(
            !working && ready,
            egui::Button::new(tr!("Проверить ключ", "Check the key")),
        )
        .on_disabled_hover_text(tr!("Вставьте ключ", "Paste the key"))
        .clicked();

    if let Some(status) = app.deepl().check.clone() {
        show_status(&status, ui);
    }

    if app.deepl().settings != before {
        app.save_deepl();
    }

    clicked
}
