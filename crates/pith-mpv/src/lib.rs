//! Движок воспроизведения Pith Player поверх libmpv.
//!
//! Приложение работает с mpv только через этот крейт — прямых вызовов
//! libmpv из интерфейса быть не должно (PLAN.md §12.4).

mod engine;
mod error;
mod options;
mod render;

pub use engine::{Engine, EngineEvent, PlaybackState};
pub use error::{MpvError, Result};
pub use options::{EngineOptions, HwDec};
pub use render::{FrameSize, ProcAddressLoader, RenderContext, SharedRenderContext};
