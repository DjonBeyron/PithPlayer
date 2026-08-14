//! Окно выгрузки отрезков в Notion: вопрос, ход работы и отчёт.

use pith_notion::Report;

use crate::ui::export_report;

use crate::app::{ExportStage, LogKind, LogLine, PithApp};
use crate::theme;
use crate::tr;

/// Размер окна при первом показе.
///
/// По высоте — чтобы разом влезли все три карточки и кнопки: окно короче
/// заставляло прокручивать вопрос, на который отвечают в два нажатия.
const DEFAULT_SIZE: [f32; 2] = [470.0, 500.0];

/// Наименьший размер: уже него не помещается вопрос о названии.
const MIN_SIZE: [f32; 2] = [380.0, 280.0];

/// Отступ содержимого от краёв.
const PADDING: i8 = 14;

/// Что нажали.
enum Action {
    Start,
    Close,
}

/// Окно выгрузки — отдельное окно системы.
///
/// Не карточка посреди кадра: его уносят на второй экран и там оставляют,
/// а место запоминается между запусками. Так же устроены окна актёров
/// и интеграций.
pub fn show(app: &mut PithApp, ctx: &egui::Context) {
    // Спрятанное окно не рисуем: выгрузка при этом идёт дальше, а её ход
    // виден в панели отрезков.
    if !app.export_window_visible() {
        return;
    }

    let viewport = app.place_export_window(
        egui::ViewportBuilder::default()
            .with_title(tr!("Выгрузка в Notion", "Export to Notion"))
            .with_min_inner_size(MIN_SIZE),
        DEFAULT_SIZE,
    );

    let id = egui::ViewportId::from_hash_of("export");
    let mut action = None;

    ctx.show_viewport_immediate(id, viewport, |ctx, _class| {
        app.track_export_window(ctx);

        // Пока человек отвечает на вопрос, плеер уже спрашивает Notion:
        // три запроса подготовки — это около трёх секунд, и ждать их после
        // нажатия «Выгрузить» незачем.
        app.prefetch_export(ctx);

        egui::CentralPanel::default()
            .frame(
                egui::Frame::NONE
                    // Гамма окон настроек: тёмная подложка, карточки светлее
                    // её, синий акцент. Так же устроены нарезка и субтитры.
                    .fill(theme::DIALOG_BG)
                    .inner_margin(egui::Margin::same(PADDING)),
            )
            .show(ctx, |ui| {
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        action = show_body(app, ui);
                    });
            });

        // Пока идёт выгрузка, окно не закрывается: работа всё равно
        // продолжается, а следить за ней станет негде.
        let working = app.export_dialog().is_some_and(|d| d.is_working());
        let asked = ctx.input(|i| i.viewport().close_requested())
            || ctx.input(|i| i.key_pressed(egui::Key::Escape));

        if !working && asked {
            action = Some(Action::Close);
        }
    });

    match action {
        Some(Action::Start) => app.start_export(ctx),
        Some(Action::Close) => app.close_export(),
        None => {}
    }
}

/// Что рисуем в этом кадре.
///
/// Снимок состояния, а не ссылка на него: вопрос о названии правит окно
/// выгрузки прямо во время отрисовки, и держать заём на его поле нельзя.
enum View {
    Ask,
    Sound { done: usize, total: usize },
    Work { done: usize, total: usize },
    Done(Box<Report>),
    Fail(String),
}

fn show_body(app: &mut PithApp, ui: &mut egui::Ui) -> Option<Action> {
    crate::ui::panel_head::style_boxes(ui);

    let view = match &app.export_dialog()?.stage {
        ExportStage::Asking => View::Ask,
        ExportStage::Sounding { done, total } => View::Sound {
            done: *done,
            total: *total,
        },
        ExportStage::Working { done, total } => View::Work {
            done: *done,
            total: *total,
        },
        ExportStage::Done(report) => View::Done(Box::new(report.clone())),
        ExportStage::Failed(why) => View::Fail(why.clone()),
    };

    let journal: Vec<LogLine> = app
        .export_dialog()
        .map(|d| d.journal().to_vec())
        .unwrap_or_default();

    let action = match view {
        View::Ask => show_question(app, ui),
        View::Sound { done, total } => {
            show_stage(
                ui,
                &tr!(
                    format!("Транскрипция: {done} из {total} слов"),
                    format!("Transcription: {done} of {total} words")
                ),
                tr!(
                    "Спрашиваются только новые слова — известные берутся из памяти.",
                    "Only new words are looked up — known ones come from memory."
                ),
                done,
                total,
            );
            None
        }
        View::Work { done, total } => {
            show_progress(ui, done, total);
            None
        }
        View::Done(report) => export_report::show_report(ui, &report).then_some(Action::Close),
        View::Fail(why) => export_report::show_failure(ui, &why).then_some(Action::Close),
    };

    show_journal(app, ui, &journal);

    action
}

/// Журнал работы: что случилось и откуда взято значение.
///
/// Виден прямо в окне, а не только в файле журнала плеера: тому, кто нажал
/// кнопку, важно знать, взялось слово из памяти или поехало в сеть, — и знать
/// сразу. Прокрутка держится у конца: интересно всегда последнее.
///
/// Каждая строка помечена слева и покрашена по источнику: зелёное нашлось
/// в первом словаре, жёлтое — во втором, красное не нашлось нигде, синее —
/// взято из памяти, ничего не спрашивали. Цвет здесь не украшение: главный
/// вопрос к журналу — ходил ли плеер в сеть, и ответ должен читаться сразу.
fn show_journal(app: &mut PithApp, ui: &mut egui::Ui, lines: &[LogLine]) {
    if lines.is_empty() {
        return;
    }

    ui.add_space(12.0);

    let mut copy = false;

    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new(tr!("Журнал", "Journal"))
                .color(theme::TEXT_SECONDARY)
                .size(12.0),
        );

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            copy = ui
                .add(
                    egui::Button::new(
                        egui::RichText::new(tr!("Скопировать", "Copy"))
                            .color(theme::DIALOG_LABEL)
                            .size(11.0),
                    )
                    .fill(theme::DIALOG_FIELD)
                    .corner_radius(crate::ui::dialog::CARD_RADIUS),
                )
                .on_hover_text(tr!(
                    "Весь журнал в буфер обмена",
                    "The whole journal to the clipboard"
                ))
                .clicked();
        });
    });

    ui.add_space(4.0);

    egui::Frame::NONE
        .fill(theme::DIALOG_CARD)
        .corner_radius(crate::ui::dialog::CARD_RADIUS)
        .inner_margin(10)
        .show(ui, |ui| {
            ui.set_width(ui.available_width() - 20.0);

            egui::ScrollArea::vertical()
                .max_height(JOURNAL_HEIGHT)
                .stick_to_bottom(true)
                .show(ui, |ui| {
                    for line in lines {
                        show_line(ui, line);
                    }
                });
        });

    if copy {
        let text: Vec<String> = lines.iter().map(LogLine::plain).collect();

        app.copy_text_to_clipboard(&text.join("\r\n"));
    }
}

/// Одна строка журнала: метка слева, текст справа.
fn show_line(ui: &mut egui::Ui, line: &LogLine) {
    ui.horizontal_top(|ui| {
        ui.spacing_mut().item_spacing.x = 8.0;

        // Метка постоянной ширины — журнал читается столбцом.
        ui.allocate_ui_with_layout(
            egui::vec2(TAG_WIDTH, ui.available_height()),
            egui::Layout::left_to_right(egui::Align::Min),
            |ui| {
                ui.label(
                    egui::RichText::new(line.kind.tag())
                        .color(tag_color(line.kind))
                        .monospace()
                        .size(11.0),
                );
            },
        );

        ui.add(
            egui::Label::new(
                egui::RichText::new(&line.text)
                    .color(text_color(line.kind))
                    .monospace()
                    .size(11.0),
            )
            .wrap(),
        );
    });
}

/// Цвет метки.
fn tag_color(kind: LogKind) -> egui::Color32 {
    match kind {
        LogKind::Step => theme::DIALOG_MUTED,
        LogKind::Memory => theme::ACCENT,
        LogKind::First | LogKind::Done => theme::ACTOR_MARK,
        LogKind::Second => theme::PANEL_ACCENT,
        LogKind::Missing | LogKind::Failed => theme::ERROR,
    }
}

/// Цвет самого текста — тише метки, чтобы столбец меток читался первым.
fn text_color(kind: LogKind) -> egui::Color32 {
    match kind {
        LogKind::Missing | LogKind::Failed => theme::ERROR,
        LogKind::Done => theme::ACTOR_MARK,
        _ => theme::DIALOG_TEXT,
    }
}

/// Ширина столбца меток: столько занимает самая длинная — «wooordhunt».
const TAG_WIDTH: f32 = 74.0;

/// Сколько места отдано журналу.
const JOURNAL_HEIGHT: f32 = 190.0;

/// Ход работы: заголовок, полоса и пояснение под ней.
fn show_stage(ui: &mut egui::Ui, title: &str, note: &str, done: usize, total: usize) {
    ui.label(egui::RichText::new(title).color(theme::ACCENT));
    ui.add_space(6.0);

    let fraction = if total == 0 {
        0.0
    } else {
        done as f32 / total as f32
    };

    ui.add(egui::ProgressBar::new(fraction).show_percentage());

    ui.add_space(6.0);
    ui.add(
        egui::Label::new(
            egui::RichText::new(note)
                .color(theme::TEXT_DISABLED)
                .size(12.0),
        )
        .wrap(),
    );
}

/// Вопрос перед выгрузкой — сама форма в `ui/export_form.rs`.
fn show_question(app: &mut PithApp, ui: &mut egui::Ui) -> Option<Action> {
    match crate::ui::export_form::show(app, ui)? {
        crate::ui::export_form::FormAction::Start => Some(Action::Start),
        crate::ui::export_form::FormAction::Close => Some(Action::Close),
    }
}

/// Ход работы: строка за строкой.
fn show_progress(ui: &mut egui::Ui, done: usize, total: usize) {
    ui.label(
        egui::RichText::new(tr!(
            format!("Выгружено: {done} из {total}"),
            format!("Exported: {done} of {total}")
        ))
        .color(theme::ACCENT),
    );

    ui.add_space(6.0);

    let fraction = if total == 0 {
        0.0
    } else {
        done as f32 / total as f32
    };

    ui.add(egui::ProgressBar::new(fraction).show_percentage());

    ui.add_space(6.0);
    ui.label(
        egui::RichText::new(tr!(
            "Строка за строкой — Notion принимает их по одной.",
            "Row by row — Notion takes them one at a time."
        ))
        .color(theme::TEXT_DISABLED)
        .size(12.0),
    );
}
