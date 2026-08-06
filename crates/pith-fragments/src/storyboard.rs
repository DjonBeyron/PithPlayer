//! Мозаика миниатюр всего фильма — для мгновенной подсказки на полосе.
//!
//! Так устроен предпросмотр в браузерных плеерах: YouTube заранее собирает
//! кадры в одну картинку-сетку и при наведении просто показывает нужную
//! клетку. Ничего не декодируется, поэтому подсказка успевает за мышью.
//!
//! Здесь то же самое: один фоновый запуск `ffmpeg` после открытия файла
//! собирает сетку миниатюр, а плеер потом вырезает из неё клетки. Точный
//! кадр под курсором отдельно достаёт второй экземпляр mpv — мозаика
//! отвечает за скорость, он за точность.

use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};

/// Ширина одной миниатюры, точки.
const TILE_WIDTH: u32 = 160;

/// Клеток в ряду.
const COLUMNS: u32 = 10;

/// Сколько миниатюр собираем на фильм.
///
/// Двести клеток при ширине 160 дают картинку 1600×~1800 — она без
/// хлопот становится одной текстурой. Для полуторачасового фильма это
/// кадр каждые 27 секунд.
const MAX_TILES: u32 = 200;

/// Минимальный шаг между кадрами, секунды.
///
/// На коротком ролике двести клеток означали бы кадр каждые полсекунды:
/// разницы не видно, а сборка длится дольше самого ролика.
const MIN_INTERVAL: f64 = 2.0;

/// Сколько мозаик держим в кэше.
///
/// Каждая занимает несколько сотен килобайт. Сотня — это память о сотне
/// последних фильмов и меньше полусотни мегабайт на диске; всё, что
/// старше, проще собрать заново, чем хранить вечно.
const CACHE_LIMIT: usize = 100;

/// Готовая мозаика миниатюр.
#[derive(Debug, Clone, PartialEq)]
pub struct Storyboard {
    /// Шаг между соседними клетками, секунды.
    pub interval: f64,
    /// Клеток в ряду.
    pub columns: u32,
    /// Рядов в сетке.
    pub rows: u32,
    /// Сколько клеток заполнено: последний ряд бывает неполным.
    pub tiles: u32,
    /// Файл мозаики.
    pub path: PathBuf,
}

impl Storyboard {
    /// Номер клетки для указанной секунды.
    pub fn tile_at(&self, time: f64) -> u32 {
        if self.interval <= 0.0 || self.tiles == 0 {
            return 0;
        }

        let index = (time.max(0.0) / self.interval).floor();
        (index as u32).min(self.tiles - 1)
    }

    /// Середина клетки во времени — к ней и относится показанный кадр.
    pub fn tile_time(&self, tile: u32) -> f64 {
        tile as f64 * self.interval
    }
}

/// План сетки для фильма такой длительности.
///
/// Отдельно от сборки, чтобы имя файла в кэше можно было посчитать
/// заранее и не собирать мозаику дважды.
pub fn plan(duration: f64) -> Option<(f64, u32, u32)> {
    if !duration.is_finite() || duration <= 0.0 {
        return None;
    }

    let interval = (duration / MAX_TILES as f64).max(MIN_INTERVAL);
    let tiles = ((duration / interval).ceil() as u32).clamp(1, MAX_TILES);
    let rows = tiles.div_ceil(COLUMNS);

    Some((interval, tiles, rows))
}

/// Собирает мозаику или берёт готовую из кэша.
///
/// Работа долгая — вызывать только из фонового потока. `None` означает,
/// что `ffmpeg` недоступен или файл не читается: плеер в этом случае
/// обходится точными кадрами от mpv.
pub fn build(video: &Path, duration: f64, cache_dir: &Path) -> Option<Storyboard> {
    let (interval, tiles, rows) = plan(duration)?;

    let path = cache_dir.join(cache_name(video, interval, rows));

    let board = Storyboard {
        interval,
        columns: COLUMNS,
        rows,
        tiles,
        path,
    };

    // Готовая мозаика того же файла: собирать заново незачем.
    if board.path.exists() {
        tracing::debug!(?board.path, "мозаика миниатюр взята из кэша");
        return Some(board);
    }

    std::fs::create_dir_all(cache_dir).ok()?;

    let started = std::time::Instant::now();
    let output = crate::quiet::background_command("ffmpeg")
        .args(["-v", "error", "-y"])
        // Только опорные кадры: для миниатюр этого довольно, а проход
        // по файлу становится в разы короче.
        .args(["-skip_frame", "nokey"])
        .arg("-i")
        .arg(video)
        .args(["-an", "-sn", "-dn"])
        .args(["-vf", &filter(interval, rows)])
        // Вся сетка — это один выходной кадр.
        .args(["-frames:v", "1"])
        .args(["-q:v", "6"])
        .arg(&board.path)
        .output()
        .ok()?;

    if !output.status.success() || !board.path.exists() {
        tracing::warn!(
            код = ?output.status.code(),
            "мозаику миниатюр собрать не удалось"
        );
        return None;
    }

    tracing::info!(
        клеток = tiles,
        шаг = interval,
        за_мс = started.elapsed().as_millis(),
        "мозаика миниатюр собрана"
    );

    prune(cache_dir, CACHE_LIMIT);

    Some(board)
}

/// Оставляет в кэше только `limit` самых свежих мозаик.
///
/// Кэш растёт с каждым новым файлом, а сам себя не чистит: без предела
/// он однажды займёт заметную часть диска ради фильмов, которые смотрели
/// однажды и год назад.
fn prune(cache_dir: &Path, limit: usize) {
    let Ok(entries) = std::fs::read_dir(cache_dir) else {
        return;
    };

    let mut files: Vec<(std::time::SystemTime, PathBuf)> = entries
        .flatten()
        .filter_map(|entry| {
            let path = entry.path();
            let modified = entry.metadata().ok()?.modified().ok()?;
            path.extension()
                .is_some_and(|ext| ext == "jpg")
                .then_some((modified, path))
        })
        .collect();

    if files.len() <= limit {
        return;
    }

    // Самые старые — в начале списка, их и убираем.
    files.sort_by_key(|(modified, _)| *modified);

    for (_, path) in files.iter().take(files.len() - limit) {
        if let Err(e) = std::fs::remove_file(path) {
            tracing::debug!(?path, error = %e, "старую мозаику убрать не удалось");
        }
    }
}

/// Цепочка фильтров: прореживание, уменьшение, сборка в сетку.
fn filter(interval: f64, rows: u32) -> String {
    // Частота задаётся дробью: `fps=1/27.4` ffmpeg разбирает как деление,
    // и шаг получается ровно тот, по которому потом ищется клетка.
    format!("fps=1/{interval},scale={TILE_WIDTH}:-2,tile={COLUMNS}x{rows}")
}

/// Имя файла в кэше: свой у каждого файла и у каждой сетки.
///
/// В ключ идут путь, размер и время правки: перезаписанный под тем же
/// именем файл получит новую мозаику, а не чужие кадры.
fn cache_name(video: &Path, interval: f64, rows: u32) -> String {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    video.to_string_lossy().to_lowercase().hash(&mut hasher);

    if let Ok(meta) = std::fs::metadata(video) {
        meta.len().hash(&mut hasher);

        if let Ok(time) = meta.modified()
            && let Ok(age) = time.duration_since(std::time::UNIX_EPOCH)
        {
            age.as_secs().hash(&mut hasher);
        }
    }

    let key = hasher.finish();
    let step = (interval * 1000.0).round() as u64;

    format!("{key:016x}_{step}_{COLUMNS}x{rows}.jpg")
}

#[cfg(test)]
mod tests {
    use super::{COLUMNS, MAX_TILES, MIN_INTERVAL, Storyboard, filter, plan};
    use std::path::PathBuf;

    fn board(duration: f64) -> Storyboard {
        let (interval, tiles, rows) = plan(duration).expect("план сетки");
        Storyboard {
            interval,
            columns: COLUMNS,
            rows,
            tiles,
            path: PathBuf::from("мозаика.jpg"),
        }
    }

    #[test]
    fn длинный_фильм_укладывается_в_предел_клеток() {
        let (_, tiles, rows) = plan(2.0 * 3600.0).expect("план сетки");
        assert_eq!(tiles, MAX_TILES);
        assert_eq!(rows, MAX_TILES / COLUMNS);
    }

    #[test]
    fn короткий_ролик_не_дробится_слишком_мелко() {
        let (interval, tiles, _) = plan(60.0).expect("план сетки");
        assert_eq!(interval, MIN_INTERVAL);
        assert_eq!(tiles, 30);
    }

    #[test]
    fn нулевая_длительность_не_даёт_сетки() {
        assert!(plan(0.0).is_none());
        assert!(plan(f64::NAN).is_none());
    }

    #[test]
    fn клетка_находится_по_времени() {
        let board = board(1000.0);

        assert_eq!(board.tile_at(0.0), 0);
        assert_eq!(board.tile_at(board.interval * 3.5), 3);
        // За концом фильма клетки нет — берём последнюю.
        assert_eq!(board.tile_at(100_000.0), board.tiles - 1);
    }

    #[test]
    fn у_клетки_есть_своё_время() {
        let board = board(1000.0);
        assert_eq!(board.tile_time(0), 0.0);
        assert_eq!(board.tile_at(board.tile_time(7)), 7);
    }

    #[test]
    fn цепочка_фильтров_собирает_сетку() {
        assert_eq!(filter(5.0, 4), "fps=1/5,scale=160:-2,tile=10x4");
    }

    #[test]
    fn кэш_не_растёт_без_предела() {
        let dir = tempfile::tempdir().expect("временный каталог");

        for index in 0..5 {
            let path = dir.path().join(format!("мозаика{index}.jpg"));
            std::fs::write(&path, "кадры").expect("файл кэша");
            // Время правки задаёт порядок: без паузы все файлы ровесники.
            std::thread::sleep(std::time::Duration::from_millis(20));
        }

        super::prune(dir.path(), 2);

        let left = std::fs::read_dir(dir.path()).expect("чтение кэша").count();
        assert_eq!(left, 2, "остаются только самые свежие");
        assert!(
            dir.path().join("мозаика4.jpg").exists(),
            "последняя мозаика нужнее прочих"
        );
    }

    #[test]
    fn чужие_файлы_в_кэше_не_трогаются() {
        let dir = tempfile::tempdir().expect("временный каталог");
        let alien = dir.path().join("заметка.txt");
        std::fs::write(&alien, "не мозаика").expect("посторонний файл");

        super::prune(dir.path(), 0);

        assert!(alien.exists(), "чистим только свои мозаики");
    }
}
