//! Перенос настроек и закладок версии 4.
//!
//! Файлы v4 только читаются (PLAN.md §6.10).

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::bookmarks::{BookmarkList, Bookmarks, DEFAULT_LIST, TimeBookmark, VideoBookmarks};
use crate::settings::Settings;

/// Переносит `settings.ini`: папку вывода и параметры нарезки.
///
/// Возвращает `true`, если что-то перенесено.
pub fn migrate_settings(v4_file: &Path, target: &mut Settings) -> bool {
    let Ok(content) = std::fs::read_to_string(v4_file) else {
        return false;
    };

    let values = parse_flat_ini(&content);
    let mut changed = false;

    if let Some(path) = values.get("RecordingPath") {
        target.fragments.output_dir = Some(PathBuf::from(path));
        changed = true;
    }
    if let Some(value) = values.get("RecordingDuration").and_then(|v| v.parse().ok()) {
        target.fragments.duration_sec = value;
        changed = true;
    }
    if let Some(value) = values
        .get("RecordingBufferSeconds")
        .and_then(|v| v.parse().ok())
    {
        target.fragments.buffer_sec = value;
        changed = true;
    }
    // `ReencodeAudioToAAC` сознательно не переносим. В v4 перекодирование
    // было включено, потому что иначе фрагмент начинался с чёрных кадров.
    // В v5 та же задача решена выравниванием по ключевому кадру, и режим
    // по умолчанию — перепаковка (PLAN.md, решение 7).

    changed
}

/// Переносит `subtitle_settings.ini`: теги автовыбора субтитров.
pub fn migrate_subtitle_priority(v4_file: &Path, target: &mut Settings) -> bool {
    let Ok(content) = std::fs::read_to_string(v4_file) else {
        return false;
    };

    let sections = parse_sectioned_ini(&content);
    let priority = &mut target.subtitle_priority;
    let mut changed = false;

    if let Some(main) = sections.get("MainSubtitles") {
        if let Some(tags) = main.get("Tags") {
            priority.main_tags = split_tags(tags);
            changed = true;
        }
        if let Some(enabled) = main.get("Enabled") {
            priority.main_enabled = enabled.eq_ignore_ascii_case("true");
        }
    }

    if let Some(secondary) = sections.get("SecondarySubtitles") {
        if let Some(tags) = secondary.get("Tags") {
            priority.secondary_tags = split_tags(tags);
            changed = true;
        }
        if let Some(enabled) = secondary.get("Enabled") {
            priority.secondary_enabled = enabled.eq_ignore_ascii_case("true");
        }
    }

    if let Some(blacklist) = sections.get("Blacklist")
        && let Some(tags) = blacklist.get("Tags")
    {
        priority.blacklist_tags = split_tags(tags);
        changed = true;
    }

    if let Some(processing) = sections.get("Processing")
        && let Some(skip) = processing.get("SkipUnmatchedTracks")
    {
        priority.skip_unmatched = skip.eq_ignore_ascii_case("true");
    }

    changed
}

/// Переносит `bookmarks.json`: плоские списки превращаются в список
/// с именем «Основной».
///
/// Возвращает число перенесённых видео.
pub fn migrate_bookmarks(
    v4_file: &Path,
    target: &mut Bookmarks,
    duration_sec: u32,
    buffer_sec: u32,
) -> usize {
    let Ok(content) = std::fs::read_to_string(v4_file) else {
        return 0;
    };

    let videos = match serde_json::from_str::<Vec<V4Video>>(content.trim_start_matches('\u{feff}'))
    {
        Ok(videos) => videos,
        Err(e) => {
            tracing::warn!(error = %e, "не удалось разобрать закладки версии 4");
            return 0;
        }
    };

    let mut moved = 0;

    for video in videos {
        if video.bookmarks.is_empty() {
            continue;
        }

        let bookmarks: Vec<TimeBookmark> = video
            .bookmarks
            .into_iter()
            .map(|b| TimeBookmark {
                time_ms: b.time_ms,
                name: b.custom_name.filter(|n| !n.trim().is_empty()),
            })
            .collect();

        let mut list = BookmarkList::new(DEFAULT_LIST, duration_sec, buffer_sec);
        list.bookmarks = bookmarks;

        target.insert_raw(VideoBookmarks {
            video_file_name: video.video_file_name,
            active_list: DEFAULT_LIST.to_string(),
            lists: vec![list],
        });
        moved += 1;
    }

    if moved > 0 {
        target.save();
    }

    tracing::info!(видео = moved, "перенос закладок из версии 4");
    moved
}

/// Запись видео в файле закладок v4.
#[derive(serde::Deserialize)]
struct V4Video {
    #[serde(rename = "VideoFileName")]
    video_file_name: String,
    #[serde(rename = "Bookmarks", default)]
    bookmarks: Vec<V4Bookmark>,
}

#[derive(serde::Deserialize)]
struct V4Bookmark {
    #[serde(rename = "TimeMs")]
    time_ms: i64,
    #[serde(rename = "CustomName")]
    custom_name: Option<String>,
}

fn split_tags(value: &str) -> Vec<String> {
    value
        .split(',')
        .map(|t| t.trim().to_lowercase())
        .filter(|t| !t.is_empty())
        .collect()
}

/// Разбирает INI без секций: `ключ=значение`.
fn parse_flat_ini(content: &str) -> HashMap<String, String> {
    content
        .lines()
        .filter_map(|line| line.trim().split_once('='))
        .map(|(k, v)| (k.trim().to_string(), v.trim().to_string()))
        .collect()
}

/// Разбирает INI с секциями.
fn parse_sectioned_ini(content: &str) -> HashMap<String, HashMap<String, String>> {
    let mut sections = HashMap::new();
    let mut current = String::new();

    for line in content.lines() {
        let line = line.trim();

        if line.is_empty() || line.starts_with(';') {
            continue;
        }

        if line.starts_with('[') && line.ends_with(']') {
            current = line[1..line.len() - 1].to_string();
            sections.entry(current.clone()).or_insert_with(HashMap::new);
            continue;
        }

        if let Some((key, value)) = line.split_once('=') {
            sections
                .entry(current.clone())
                .or_insert_with(HashMap::new)
                .insert(key.trim().to_string(), value.trim().to_string());
        }
    }

    sections
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::DataPaths;

    const SETTINGS_V4: &str = "\
LastFileFolder=D:\\Download
RecordingPath=C:\\PITH\\2. Video source\\# TEMP
RecordingDuration=18
RecordingBufferSeconds=5
ReencodeAudioToAAC=True
";

    const SUBTITLES_V4: &str = "\
[MainSubtitles]
Enabled=True
Tags=sdh,english,eng,Английский

[SecondarySubtitles]
Enabled=True
Tags=russian,rus,Русские

[Processing]
SkipUnmatchedTracks=True

[Blacklist]
Tags=Бразилия,Турция
";

    #[test]
    fn переносит_параметры_нарезки() {
        let dir = tempfile::tempdir().expect("каталог");
        let file = dir.path().join("settings.ini");
        std::fs::write(&file, SETTINGS_V4).expect("запись");

        let mut settings = Settings::default();
        assert!(migrate_settings(&file, &mut settings));

        assert_eq!(settings.fragments.duration_sec, 18);
        assert_eq!(settings.fragments.buffer_sec, 5);
        assert_eq!(
            settings.fragments.output_dir,
            Some(PathBuf::from("C:\\PITH\\2. Video source\\# TEMP"))
        );
        assert!(
            !settings.fragments.reencode,
            "перекодирование из v4 не переносится: в v5 по умолчанию перепаковка"
        );
    }

    #[test]
    fn переносит_теги_субтитров_включая_кириллицу() {
        let dir = tempfile::tempdir().expect("каталог");
        let file = dir.path().join("subtitle_settings.ini");
        std::fs::write(&file, SUBTITLES_V4).expect("запись");

        let mut settings = Settings::default();
        assert!(migrate_subtitle_priority(&file, &mut settings));

        let priority = &settings.subtitle_priority;
        assert!(priority.main_tags.contains(&"английский".to_string()));
        assert!(priority.secondary_enabled, "вторые субтитры были включены");
        assert!(priority.blacklist_tags.contains(&"бразилия".to_string()));
        assert!(priority.skip_unmatched);
    }

    #[test]
    fn переносит_закладки_в_список_по_умолчанию() {
        let dir = tempfile::tempdir().expect("каталог");

        // Данные версии 4 лежат в своей папке: у файлов одинаковые имена,
        // и в общем каталоге хранилище v5 приняло бы чужой файл за свой.
        let v4_dir = dir.path().join("v4");
        std::fs::create_dir_all(&v4_dir).expect("каталог версии 4");
        let file = v4_dir.join("bookmarks.json");

        let json = r#"[
            {
                "VideoFileName": "Фильм",
                "Bookmarks": [
                    {"TimeMs": 372398, "CustomName": "Реплика"},
                    {"TimeMs": 635036, "CustomName": ""}
                ]
            }
        ]"#;
        std::fs::write(&file, json).expect("запись");

        let mut store = Bookmarks::load(DataPaths::with_root(dir.path()));
        assert_eq!(migrate_bookmarks(&file, &mut store, 18, 5), 1);

        let video = store.for_video("Фильм").expect("видео найдено");
        assert_eq!(video.active_list, DEFAULT_LIST);
        assert_eq!(video.lists[0].bookmarks.len(), 2);
        assert_eq!(video.lists[0].bookmarks[0].name.as_deref(), Some("Реплика"));
        assert_eq!(
            video.lists[0].bookmarks[1].name, None,
            "пустое название не переносится"
        );
        assert_eq!(video.lists[0].duration_sec, 18);
    }

    #[test]
    fn отсутствующие_файлы_не_ошибка() {
        let dir = tempfile::tempdir().expect("каталог");
        let mut settings = Settings::default();
        let mut store = Bookmarks::load(DataPaths::with_root(dir.path()));

        assert!(!migrate_settings(
            &dir.path().join("нет.ini"),
            &mut settings
        ));
        assert!(!migrate_subtitle_priority(
            &dir.path().join("нет.ini"),
            &mut settings
        ));
        assert_eq!(
            migrate_bookmarks(&dir.path().join("нет.json"), &mut store, 18, 5),
            0
        );
    }

    #[test]
    fn видео_без_закладок_пропускается() {
        let dir = tempfile::tempdir().expect("каталог");
        let v4_dir = dir.path().join("v4");
        std::fs::create_dir_all(&v4_dir).expect("каталог версии 4");

        let file = v4_dir.join("bookmarks.json");
        std::fs::write(&file, r#"[{"VideoFileName": "Пусто", "Bookmarks": []}]"#).expect("запись");

        let mut store = Bookmarks::load(DataPaths::with_root(dir.path()));
        assert_eq!(migrate_bookmarks(&file, &mut store, 18, 5), 0);
    }
}
