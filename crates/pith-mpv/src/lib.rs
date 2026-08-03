//! Движок воспроизведения Pith Player поверх libmpv.
//!
//! Приложение работает с mpv только через этот крейт — прямых вызовов
//! libmpv из интерфейса быть не должно (PLAN.md §12.4).

mod audio;
mod engine;
mod error;
mod options;
mod render;
mod tracks;
mod video;

pub use audio::{AUTO_DEVICE, AudioDevice};
pub use engine::{Engine, EngineEvent, PlaybackState};
pub use error::{MpvError, Result};
pub use options::{EngineOptions, HwDec};
pub use render::{FrameSize, ProcAddressLoader, RenderContext, SharedRenderContext};
pub use tracks::{Track, TrackKind};
