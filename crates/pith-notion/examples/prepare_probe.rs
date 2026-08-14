//! Замер подготовки: сколько стоит она целиком и сколько — с памятью.
//!
//! Постоянная часть (база, заготовка строки, поле номера) меняется только
//! вместе со ссылками, и плеер держит её у себя. Остаётся один запрос —
//! последний номер. Проба показывает разницу.
//!
//! ```bash
//! cargo run -p pith-notion --release --example prepare_probe -- <токен> <ссылка на образец> <ссылка на рабочую>
//! ```

use std::time::Instant;

use pith_notion::{Notion, discover, parse_id, prepare_from};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();

    let [token, template, work] = args.as_slice() else {
        eprintln!("нужно: <токен> <ссылка на образец> <ссылка на рабочую>");
        std::process::exit(2);
    };

    let Some(notion) = Notion::new(token) else {
        eprintln!("токен пуст");
        std::process::exit(2);
    };

    let (Some(template), Some(work)) = (parse_id(template), parse_id(work)) else {
        eprintln!("в ссылке нет номера страницы");
        std::process::exit(2);
    };

    let at = Instant::now();
    let stable = match discover(&notion, &work, &template) {
        Ok(stable) => stable,
        Err(e) => {
            eprintln!("подготовка не вышла: {e}");
            std::process::exit(1);
        }
    };
    let discovered = at.elapsed();

    let at = Instant::now();
    let prepared = prepare_from(&notion, stable.clone());
    let numbered = at.elapsed();

    println!(
        "постоянная часть (4 запроса): {:.2} с",
        discovered.as_secs_f32()
    );
    println!(
        "последний номер (1 запрос): {:.2} с",
        numbered.as_secs_f32()
    );
    println!(
        "было {:.2} с · стало {:.2} с",
        (discovered + numbered).as_secs_f32(),
        numbered.as_secs_f32()
    );
    println!(
        "база {} · заготовка полей {} · поле номера {}",
        prepared.database,
        stable.sample.len(),
        stable.numbered
    );
}
