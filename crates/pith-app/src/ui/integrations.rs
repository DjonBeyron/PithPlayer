//! Окно интеграций: доступ к Notion и ключ базы фильмов.
//!
//! Отдельное окно, как у актёров: настройки вводят один раз, и закрывать
//! ими кадр незачем. Состояние и запросы — в `app/integrations.rs`.

use crate::app::{AccessStatus, IntegrationsState, PithApp};
use crate::theme;
use crate::tr;
use crate::ui::panel_head;

/// Размер окна при первом показе.
const DEFAULT_SIZE: [f32; 2] = [460.0, 480.0];

/// Наименьший размер: уже него поля со ссылками нечитаемы.
const MIN_SIZE: [f32; 2] = [380.0, 320.0];

/// Отступ содержимого от краёв.
const PADDING: i8 = 14;

/// Где взять ключ доступа к базе фильмов.
const TMDB_KEY_URL: &str = "https://www.themoviedb.org/settings/api";

/// Где заводится интеграция Notion.
const NOTION_URL: &str = "https://www.notion.so/my-integrations";

/// Сколько высоты оставить строке кнопок.
///
/// Область прокрутки иначе забирает всю оставшуюся высоту, и кнопки
/// уходят за нижний край окна — их просто не видно.
const BUTTONS_HEIGHT: f32 = 48.0;

/// Что нажали в окне.
enum Action {
    Check,
    Save,
    Close,
}

pub fn show(app: &mut PithApp, ctx: &egui::Context) {
    if !app.integrations_open() {
        return;
    }

    let viewport = egui::ViewportBuilder::default()
        .with_title(tr!("Интеграции", "Integrations"))
        .with_inner_size(DEFAULT_SIZE)
        .with_min_inner_size(MIN_SIZE);

    let id = egui::ViewportId::from_hash_of("integrations");
    let mut action = None;

    ctx.show_viewport_immediate(id, viewport, |ctx, _class| {
        egui::CentralPanel::default()
            .frame(
                egui::Frame::NONE
                    .fill(theme::PANEL_CARD)
                    .inner_margin(egui::Margin::same(PADDING)),
            )
            .show(ctx, |ui| {
                if let Some(chosen) = show_body(app.integrations_state(), ui) {
                    action = Some(chosen);
                }
            });

        if ctx.input(|i| i.viewport().close_requested()) {
            action = Some(Action::Close);
        }
    });

    match action {
        Some(Action::Check) => app.check_notion_access(ctx),
        Some(Action::Save) => app.save_integrations(),
        Some(Action::Close) => app.close_integrations(),
        None => {}
    }
}

fn show_body(state: &mut IntegrationsState, ui: &mut egui::Ui) -> Option<Action> {
    panel_head::style_boxes(ui);

    let mut action = None;

    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .max_height((ui.available_height() - BUTTONS_HEIGHT).max(0.0))
        .show(ui, |ui| {
            action = show_notion(state, ui);

            ui.add_space(16.0);
            show_tmdb(state, ui);
        });

    if let Some(chosen) = show_buttons(ui) {
        action = Some(chosen);
    }

    action
}

/// Всё про Notion: токен, страница базы и проверка доступа.
///
/// База одна на все картины, и настраивать в обычной жизни нечего:
/// на виду токен и страница с базой, образец убран вниз — его задают
/// один раз и больше не трогают.
fn show_notion(state: &mut IntegrationsState, ui: &mut egui::Ui) -> Option<Action> {
    show_section(ui, "Notion");
    // Без стрелок в подсказке: в шрифтах egui их нет, и на месте знака
    // выходит пустой квадрат (та же беда, что в окне актёров).
    show_hint(
        ui,
        tr!(
            "Страница должна быть открыта интеграции: в самой странице «…», \
             затем «Соединения» и ваша интеграция.",
            "The page must be shared with the integration: on the page \
             itself «…», then «Connections» and your integration."
        ),
    );
    ui.hyperlink_to(egui::RichText::new(NOTION_URL).size(11.0), NOTION_URL);

    ui.add_space(10.0);

    show_field(
        ui,
        tr!("Токен интеграции", "Integration token"),
        "ntn_…",
        &mut state.notion.token,
        true,
    );
    show_field(
        ui,
        tr!("Страница с базой отрезков", "Fragments database page"),
        tr!("Ссылка на страницу", "Link to the page"),
        &mut state.notion.work_page,
        false,
    );

    show_template_field(state, ui);

    ui.add_space(6.0);

    let working = state.status == AccessStatus::Working;
    let ready = state.notion.is_ready();

    let clicked = ui
        .add_enabled(
            !working && ready,
            egui::Button::new(tr!("Проверить доступ", "Check access")),
        )
        .on_disabled_hover_text(tr!("Заполните поля выше", "Fill in the fields above"))
        .clicked();

    show_status(&state.status, ui);

    clicked.then_some(Action::Check)
}

/// Страница-образец — в раскрывающемся разделе.
///
/// Из неё берутся значения строки-образца: `STATUS` и разделитель.
/// Меняют её раз в жизни, а место в окне она занимала наравне с нужным.
fn show_template_field(state: &mut IntegrationsState, ui: &mut egui::Ui) {
    egui::CollapsingHeader::new(
        egui::RichText::new(tr!("Образец строки", "Row template"))
            .color(theme::PANEL_MUTED)
            .size(12.0),
    )
    .id_salt("notion_template")
    .show(ui, |ui| {
        show_hint(
            ui,
            tr!(
                "Страница DIFF: из её единственной строки берутся значения \
                 для новых — STATUS и разделитель. Плеер её только читает.",
                "The DIFF page: values for new rows — STATUS and the \
                 separator — are taken from its only row. Read only."
            ),
        );
        ui.add_space(6.0);

        show_field(
            ui,
            tr!("Страница-образец", "Template page"),
            tr!(
                "Ссылка на страницу с базой",
                "Link to the page with the database"
            ),
            &mut state.notion.template_page,
            false,
        );
    });
}

/// Ключ базы фильмов — тот же, что просит окно актёров.
fn show_tmdb(state: &mut IntegrationsState, ui: &mut egui::Ui) {
    show_section(ui, tr!("База фильмов", "Movie database"));
    show_hint(
        ui,
        tr!(
            "Ключ бесплатный. Нужен короткий «API Key (v3 auth)», \
             а не длинный токен.",
            "The key is free. You need the short «API Key (v3 auth)», \
             not the long token."
        ),
    );
    ui.hyperlink_to(egui::RichText::new(TMDB_KEY_URL).size(11.0), TMDB_KEY_URL);

    ui.add_space(10.0);

    show_field(
        ui,
        tr!("Ключ TMDB", "TMDB key"),
        tr!("Вставьте ключ сюда", "Paste the key here"),
        &mut state.tmdb_key,
        true,
    );
}

/// Кнопки внизу окна.
fn show_buttons(ui: &mut egui::Ui) -> Option<Action> {
    ui.add_space(12.0);

    let mut action = None;

    ui.horizontal(|ui| {
        let save = egui::Button::new(
            egui::RichText::new(tr!("Сохранить", "Save"))
                .color(theme::PANEL_CARD)
                .strong(),
        )
        .fill(theme::PANEL_ACCENT)
        .min_size(egui::vec2(120.0, 30.0));

        if ui.add(save).clicked() {
            action = Some(Action::Save);
        }

        if ui
            .add(egui::Button::new(tr!("Закрыть", "Close")).min_size(egui::vec2(90.0, 30.0)))
            .clicked()
        {
            action = Some(Action::Close);
        }
    });

    action
}

/// Строка о том, что происходит: проверка, отказ, сохранение.
fn show_status(status: &AccessStatus, ui: &mut egui::Ui) {
    let (text, color) = match status {
        AccessStatus::Idle => return,
        AccessStatus::Working => (
            tr!("Спрашиваю Notion…", "Asking Notion…").to_string(),
            theme::PANEL_MUTED,
        ),
        AccessStatus::Ok => (
            tr!(
                "Обе страницы видны, базы найдены",
                "Both pages are visible, both databases are found"
            )
            .to_string(),
            theme::PANEL_ACCENT,
        ),
        AccessStatus::Failed(why) => (why.clone(), theme::ERROR),
        AccessStatus::Saved => (
            tr!("Настройки сохранены", "Settings saved").to_string(),
            theme::PANEL_ACCENT,
        ),
    };

    ui.add_space(6.0);
    ui.add(egui::Label::new(egui::RichText::new(text).color(color).size(12.0)).wrap());
}

/// Подпись раздела.
fn show_section(ui: &mut egui::Ui, title: &str) {
    ui.label(
        egui::RichText::new(title)
            .color(theme::TEXT_PRIMARY)
            .size(15.0)
            .strong(),
    );
    ui.add_space(4.0);
}

/// Пояснение мелким шрифтом.
fn show_hint(ui: &mut egui::Ui, text: &str) {
    ui.add(
        egui::Label::new(
            egui::RichText::new(text)
                .color(theme::PANEL_MUTED)
                .size(12.0),
        )
        .wrap(),
    );
}

/// Поле с подписью над ним.
///
/// Токен и ключ прячутся точками: окно открывают при посторонних не реже,
/// чем без них, а вставленное из буфера проверять глазами всё равно нечем.
fn show_field(ui: &mut egui::Ui, label: &str, hint: &str, value: &mut String, secret: bool) {
    ui.label(
        egui::RichText::new(label)
            .color(theme::TEXT_SECONDARY)
            .size(12.0),
    );
    ui.add_space(2.0);
    ui.add(
        egui::TextEdit::singleline(value)
            .hint_text(hint)
            .password(secret)
            .desired_width(f32::INFINITY),
    );
    ui.add_space(8.0);
}
