//! Ручная проверка распознавания.
//!
//! `cargo run --release -p pith-ocr --example read -- <папка моделей> <картина>…`
fn main() {
    let mut args = std::env::args().skip(1);
    let models = std::path::PathBuf::from(args.next().unwrap_or_default());

    for image in args {
        let started = std::time::Instant::now();
        let text = pith_ocr::recognize(&models, std::path::Path::new(&image));
        println!("{image}: {text:?} ({} мс)", started.elapsed().as_millis());
    }
}
