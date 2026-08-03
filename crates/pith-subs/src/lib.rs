//! Разбор и поиск по субтитрам.
//!
//! Текущая реплика берётся у mpv свойством `sub-text`; этот крейт нужен
//! для поиска, которому требуется весь текст дорожки сразу.

mod extract;
mod parse;
mod quiet;
mod search;

pub use extract::{ExtractError, extract_track, is_ffmpeg_available, read_external};
pub use parse::{Cue, parse_srt};
pub use search::{SearchHit, cue_at, next_cue, previous_cue, search};
