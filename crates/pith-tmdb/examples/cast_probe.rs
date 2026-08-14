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

    russian_names(&tmdb, &cast);

    Ok(())
}

/// Сколько имён база знает по-русски и во сколько это обходится.
///
/// Русское имя лежит не в составе, а в карточке человека — по запросу
/// на каждого. Замер отвечает на два вопроса разом: многим ли именам
/// вообще есть русский вид и сколько стоит спросить обо всех.
fn russian_names(tmdb: &Tmdb, cast: &[pith_tmdb::Actor]) {
    // Имена в составе уже приходят по-русски: запрос идёт с `language=ru-RU`,
    // и база отдаёт переведённое имя, если оно у неё есть. Считаем, скольким
    // его недостаёт, и только этих спрашиваем отдельно — вдруг русский вид
    // лежит в списке прочих имён.
    let latin: Vec<&pith_tmdb::Actor> = cast
        .iter()
        .filter(|actor| !actor.name.chars().any(|c| ('А'..='я').contains(&c)))
        .collect();

    println!(
        "по-русски в составе: {} из {}",
        cast.len() - latin.len(),
        cast.len()
    );

    if latin.is_empty() {
        return;
    }

    let started = std::time::Instant::now();
    let mut found = 0;

    for actor in &latin {
        match tmdb.russian_name(actor.id) {
            Ok(Some(russian)) => {
                found += 1;
                println!("  {:<34} → {russian}", actor.name);
            }
            Ok(None) => println!("  {:<34} → русского имени нет и в карточке", actor.name),
            Err(e) => println!("  {:<34} → не спросить: {e}", actor.name),
        }
    }

    println!(
        "из карточек добрали: {found} из {} · {:.1} с",
        latin.len(),
        started.elapsed().as_secs_f32(),
    );
}
