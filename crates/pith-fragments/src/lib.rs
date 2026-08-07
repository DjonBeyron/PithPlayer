//! Нарезка отрезков видео.
//!
//! Режим по умолчанию — перепаковка контейнера: видео и звук копируются
//! байт в байт, качество не теряется, скорость в десятки раз выше
//! перекодирования (PLAN.md §6.4).

mod command;
mod container;
mod crop;
mod quiet;
mod runner;
mod storyboard;
mod time;

pub use command::FragmentJob;
pub use container::choose_container;
pub use crop::{Crop, detect as detect_crop};
pub use runner::{
    ExtractionOutcome, is_ffmpeg_available, run_job, sanitize, unique_output_path, warm_up,
};
pub use storyboard::{Storyboard, build as build_storyboard};
pub use time::format_time;
