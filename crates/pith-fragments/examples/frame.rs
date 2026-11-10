//! Ручная проверка: кадр зоны из видео.
//!
//! `cargo run -p pith-fragments --example frame -- <видео> <секунда> <x> <y> <w> <h> <выход.png>`
fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    let num = |i: usize| a[i].parse::<f32>().unwrap_or(0.0);
    let region = pith_fragments::Region {
        x: num(2),
        y: num(3),
        w: num(4),
        h: num(5),
    };

    let result = pith_fragments::capture_region(
        std::path::Path::new(&a[0]),
        f64::from(num(1)),
        region,
        std::path::Path::new(&a[6]),
    );
    println!("{result:?}");
}
