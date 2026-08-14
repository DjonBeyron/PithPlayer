//! Проверка на живой базе: имя файла → картина → состав.
//!
//! Запуск:
//! `cargo run -p pith-tmdb --example cast_probe -- <ключ> "имя файла.mkv"`
//!
//! Ключ передаётся аргументом и в вывод не попадает.

use pith_tmdb::{PhotoSize, Tmdb};

fn main() {
    let mut args = std::env::args().skip(1);

    let (Some(key), Some(file_name)) = (args.next(), args.next()) else {
        eprintln!("укажите ключ и имя файла");
        std::process::exit(2);
    };

    if let Err(message) = probe(&key, &file_name) {
        eprintln!("{message}");
        std::process::exit(1);
    }
}

fn probe(key: &str, file_name: &str) -> Result<(), String> {
    let query = pith_tmdb::parse_file_name(file_name).ok_or("из имени файла не вышло названия")?;
    println!("ищу: «{}», год {:?}", query.title, query.year);

    let tmdb = Tmdb::new(key).ok_or("ключ пуст")?;

    let title = tmdb.find(&query).map_err(|e| e.to_string())?;
    println!(
        "нашлось: {} ({:?}), {}",
        title.name,
        title.year,
        if title.series {
            "сериал"
        } else {
            "фильм"
        }
    );

    let cast = tmdb.cast(&title).map_err(|e| e.to_string())?;
    println!("актёров: {}", cast.len());

    for actor in cast.iter().take(8) {
        let photo = actor
            .photo_url(PhotoSize::List)
            .unwrap_or_else(|| "без фото".into());
        println!("  {:<44} {photo}", actor.label());
    }

    // Русское имя спрашивается отдельно и только для выбранного.
    if let Some(first) = cast.first() {
        let russian = tmdb.russian_name(first.id).map_err(|e| e.to_string())?;
        println!("русское имя первого: {russian:?}");
    }

    Ok(())
}
