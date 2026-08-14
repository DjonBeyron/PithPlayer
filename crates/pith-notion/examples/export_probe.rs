//! Проверка на живом Notion: выгрузка отрезков, без плеера.
//!
//! Повторяет то, что делает кнопка «Выгрузить в Notion»: берёт заготовку
//! строки у образца и складывает строки в рабочую базу.
//!
//! ```bash
//! cargo run -p pith-notion --example export_probe -- <токен> <ссылка на образец> <ссылка на рабочую страницу> "Проба" 3
//! ```

use pith_notion::{Kind, Notion, Row, export, film_name, parse_id, prepare};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();

    let [token, template, work, title, count] = args.as_slice() else {
        eprintln!("нужно: <токен> <ссылка на образец> <ссылка на рабочую> <название> <строк>");
        std::process::exit(2);
    };

    let Some(notion) = Notion::new(token) else {
        eprintln!("токен пуст");
        std::process::exit(2);
    };

    // Ссылки хранятся целиком, номер достаётся при обращении — как в плеере.
    let (Some(template), Some(work)) = (parse_id(template), parse_id(work)) else {
        eprintln!("в ссылке нет номера страницы");
        std::process::exit(2);
    };

    let count: usize = count.parse().unwrap_or(3);
    let name = film_name(Kind::Series, title);
    let rows = образцы(count);

    println!("картина: {name}");

    // Подготовка отдельно: в плеере она идёт заранее, пока открыт вопрос.
    let prepared = match prepare(&notion, &work, &template) {
        Ok(prepared) => prepared,
        Err(e) => {
            eprintln!("подготовка не вышла: {e}");
            std::process::exit(1);
        }
    };

    let report = export(&notion, &prepared, &name, &rows, |done, total| {
        println!("  строка {done} из {total}");
    });

    println!("база: {}", report.database_id);
    println!(
        "создано: {}, с номера {}, без актёра: {}, отказов: {}, заготовка взята: {}",
        report.created,
        report.first_number,
        report.without_actor,
        report.failed.len(),
        report.sample_taken
    );

    for (number, reason) in &report.failed {
        println!("  строка {number}: {reason}");
    }
}

/// Отрезки для пробы: у каждой второй закладки актёра нет.
fn образцы(count: usize) -> Vec<Row> {
    (1..=count)
        .map(|number| Row {
            number,
            text: format!("Проба {number}: what do you mean by that?"),
            actor: (number % 2 == 1).then(|| "Лили Коллинз (Emily)".to_string()),
            // Транскрипцию проба не запрашивает: словари медленные,
            // а проверяется здесь выгрузка.
            sounds: Vec::new(),
        })
        .collect()
}
