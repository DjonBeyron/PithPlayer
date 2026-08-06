//! Предпросмотр кадра на полосе перемотки.
//!
//! Источников два, и они дополняют друг друга.
//!
//! Мозаика миниатюр собирается в фоне сразу после открытия файла: пока
//! мышь скользит по полосе, клетка из неё показывается мгновенно — ничего
//! декодировать не нужно. Так работает предпросмотр в браузерных плеерах.
//!
//! Точный кадр достаёт второй экземпляр mpv (см. [`super::preview_source`]):
//! он держит файл открытым и отвечает за десятки миллисекунд. Им уточняется
//! то место, где пользователь остановился.

use std::sync::mpsc::{Receiver, channel};

use pith_fragments::Storyboard;

use super::PithApp;
use super::preview_source::FrameSource;

/// Насколько должно измениться место, чтобы просить новый точный кадр.
///
/// Мышь двигается непрерывно, и на каждый пиксель кадр не нужен: соседние
/// места отличаются на доли секунды и выглядят одинаково.
const STEP: f64 = 0.2;

/// Точный кадр от второго экземпляра mpv.
struct ExactFrame {
    /// Время, которое просили.
    time: f64,
    texture: egui::TextureHandle,
}

/// Мозаика миниатюр, готовая к показу.
struct Board {
    layout: Storyboard,
    texture: egui::TextureHandle,
}

/// Что показать в окошке предпросмотра.
pub struct PreviewImage<'a> {
    pub texture: &'a egui::TextureHandle,
    /// Какую часть картинки показывать: у мозаики это одна клетка.
    pub uv: egui::Rect,
    /// Размер этой части в точках — нужен, чтобы сохранить пропорции кадра.
    pub size: egui::Vec2,
}

/// Состояние предпросмотра.
#[derive(Default)]
pub struct PreviewState {
    /// Точный кадр под курсором.
    frame: Option<ExactFrame>,
    /// Мозаика миниатюр всего фильма.
    board: Option<Board>,
    /// Сборка мозаики идёт в фоне.
    building: Option<Receiver<Option<(Storyboard, egui::ColorImage)>>>,
    /// Мозаику собрать не удалось — второй раз не пробуем.
    board_failed: bool,
    /// Поток со вторым экземпляром mpv.
    source: Option<FrameSource>,
    /// Для какого места точный кадр уже заказан.
    requested: Option<f64>,
}

impl PithApp {
    /// Что показать в окошке предпросмотра для этого места.
    ///
    /// Точный кадр показывается, когда он про это самое место; иначе —
    /// клетка мозаики. Пока не готово ни то ни другое, окошко пустое,
    /// но своего размера: прыгать под курсором оно не должно.
    pub fn preview_image(&self, time: f64) -> Option<PreviewImage<'_>> {
        let board = self.preview.board.as_ref();

        if let Some(frame) = self.preview.frame.as_ref() {
            // Клетка мозаики отвечает за место лишь приблизительно —
            // точный кадр лучше, пока он про то же место.
            let precise =
                board.is_none_or(|board| (frame.time - time).abs() <= board.layout.interval / 2.0);

            if precise {
                return Some(PreviewImage {
                    texture: &frame.texture,
                    uv: egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0)),
                    size: frame.texture.size_vec2(),
                });
            }
        }

        let board = board?;
        Some(board.tile(time))
    }

    /// Отмечает место, для которого нужен точный кадр.
    ///
    /// Пока ползунок под пальцем и мозаика готова, точный кадр не просим:
    /// второй экземпляр mpv читает тот же файл, что и основной, и мешает
    /// ему перематывать — именно от этого перемотка «залипала». Клетка
    /// мозаики за это время всё равно успевает смениться, а точный кадр
    /// подтянется, как только рука остановится.
    pub fn request_preview(&mut self, time: f64) {
        if self.scrubbing && self.preview.board.is_some() {
            // Забываем прошлый заказ: когда ползунок отпустят, кадр этого
            // места нужно будет попросить заново.
            self.preview.requested = None;
            return;
        }

        if let Some(known) = self.preview.requested
            && (known - time).abs() < STEP
        {
            return;
        }

        self.preview.requested = Some(time);

        if let Some(source) = self.preview.source.as_ref() {
            source.request(time);
        }
    }

    /// Убирает предпросмотр: курсор ушёл с полосы.
    ///
    /// Мозаику и второй экземпляр mpv не трогаем — они про весь файл,
    /// а не про одно наведение, и собирать их заново было бы расточительно.
    pub fn clear_preview(&mut self) {
        self.preview.frame = None;
        self.preview.requested = None;
    }

    /// Забывает всё о прошлом файле.
    pub(super) fn reset_preview(&mut self) {
        self.preview = PreviewState::default();
    }

    /// Готовит источники и забирает готовое. Вызывается каждый кадр.
    pub(super) fn poll_preview(&mut self, ctx: &egui::Context) {
        self.ensure_frame_source(ctx);
        self.ensure_storyboard();
        self.take_ready_board(ctx);
        self.take_ready_frame(ctx);
    }

    /// Запускает поток точных кадров для текущего файла.
    fn ensure_frame_source(&mut self, ctx: &egui::Context) {
        if self.preview.source.is_some() {
            return;
        }

        let Some(path) = self.current_path.clone() else {
            return;
        };

        let ctx = ctx.clone();
        self.preview.source = Some(FrameSource::spawn(&path, move || ctx.request_repaint()));
    }

    /// Запускает фоновую сборку мозаики.
    ///
    /// Длительность известна не сразу после открытия файла, поэтому
    /// попытка повторяется каждый кадр, пока не выйдет.
    fn ensure_storyboard(&mut self) {
        if self.preview.board.is_some()
            || self.preview.building.is_some()
            || self.preview.board_failed
        {
            return;
        }

        let Some(path) = self.current_path.clone() else {
            return;
        };

        let duration = self
            .engine()
            .map(|e| e.state().duration)
            .unwrap_or_default();

        if duration <= 0.0 {
            return;
        }

        let cache = self.data_paths.thumbnails();
        let (sender, receiver) = channel();
        self.preview.building = Some(receiver);

        // Сборка идёт минуты на длинном фильме, а разбор готовой картинки
        // стоит десятков миллисекунд — и то и другое мимо кадра интерфейса.
        std::thread::spawn(move || {
            let board = pith_fragments::build_storyboard(&path, duration, &cache)
                .and_then(|board| load_image(&board.path).map(|image| (board, image)));

            let _ = sender.send(board);
        });
    }

    /// Забирает собранную мозаику.
    fn take_ready_board(&mut self, ctx: &egui::Context) {
        let Some(receiver) = self.preview.building.as_ref() else {
            return;
        };

        let Ok(result) = receiver.try_recv() else {
            return;
        };

        self.preview.building = None;

        let Some((layout, image)) = result else {
            tracing::info!("мозаика миниатюр недоступна — остаются точные кадры");
            self.preview.board_failed = true;
            return;
        };

        self.preview.board = Some(Board {
            texture: ctx.load_texture("предпросмотр_мозаика", image, texture_options()),
            layout,
        });
    }

    /// Забирает готовый точный кадр.
    fn take_ready_frame(&mut self, ctx: &egui::Context) {
        let Some((time, data)) = self.preview.source.as_ref().and_then(|s| s.take_frame()) else {
            return;
        };

        let Some(image) = decode_image(&data) else {
            tracing::warn!("кадр предпросмотра не разобрался");
            return;
        };

        self.preview.frame = Some(ExactFrame {
            time,
            texture: ctx.load_texture("предпросмотр_кадр", image, texture_options()),
        });
    }
}

impl Board {
    /// Клетка мозаики для этого места.
    fn tile(&self, time: f64) -> PreviewImage<'_> {
        let index = self.layout.tile_at(time);
        let column = (index % self.layout.columns) as f32;
        let row = (index / self.layout.columns) as f32;

        let columns = self.layout.columns as f32;
        let rows = self.layout.rows as f32;

        let uv = egui::Rect::from_min_size(
            egui::pos2(column / columns, row / rows),
            egui::vec2(1.0 / columns, 1.0 / rows),
        );

        let size = self.texture.size_vec2() / egui::vec2(columns, rows);

        PreviewImage {
            texture: &self.texture,
            uv,
            size,
        }
    }
}

/// Сглаживание при уменьшении: миниатюра и так мелкая.
fn texture_options() -> egui::TextureOptions {
    egui::TextureOptions::LINEAR
}

/// Читает картинку с диска в вид, понятный egui.
fn load_image(path: &std::path::Path) -> Option<egui::ColorImage> {
    decode_image(&std::fs::read(path).ok()?)
}

fn decode_image(data: &[u8]) -> Option<egui::ColorImage> {
    let image = image::load_from_memory(data).ok()?;
    let rgba = image.to_rgba8();
    let size = [rgba.width() as usize, rgba.height() as usize];

    Some(egui::ColorImage::from_rgba_unmultiplied(
        size,
        rgba.as_raw(),
    ))
}
