//! Состав актёров из базы фильмов TMDB.
//!
//! Крейт знает только про базу: разбирает имя файла, ищет картину, отдаёт
//! состав с путями к фотографиям. Ни окон, ни хранения — этим занимается
//! приложение (PLAN.md §12.4).
//!
//! Ключ доступа к базе бесплатный и выдаётся сразу после регистрации
//! на themoviedb.org. Хранится он в настройках плеера, в поставку
//! не попадает.

mod client;
mod error;
mod model;
mod net;
mod normalize;

pub use client::{Tmdb, fetch_photo};
pub use error::{Result, TmdbError};
pub use model::{Actor, PhotoSize, Title};
pub use net::use_proxy;
pub use normalize::{Query, parse as parse_file_name};
