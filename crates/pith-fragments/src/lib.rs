//! Нарезка отрезков видео.
//!
//! Режим по умолчанию — перепаковка контейнера: видео и звук копируются
//! байт в байт, качество не теряется, скорость в десятки раз выше
//! перекодирования (PLAN.md §6.4).

mod command;
mod container;
mod crop;
mod keyframe;
mod quiet;
mod runner;
mod thumbnail;

pub use command::{FragmentJob, format_time};
pub use container::choose_container;
pub use crop::{Crop, detect as detect_crop};
pub use keyframe::{align_to_keyframe, align_to_keyframes};
pub use runner::{
    ExtractionOutcome, is_ffmpeg_available, run_job, sanitize, unique_output_path, warm_up,
};
pub use thumbnail::grab_frame;
