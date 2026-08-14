//! Загрузка установщика нового выпуска.
//!
//! Файл пишется потоком, а не целиком в память: установщик весит под сорок
//! мегабайт. Пишется он рядом с будущим именем, но под `.part`, и только
//! дописанный до конца получает настоящее имя — недокачанный установщик
//! не должен и выглядеть готовым к запуску.

use std::fs::File;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use super::{Installer, UpdateError};

/// Каким куском читаем ответ.
const CHUNK: usize = 64 * 1024;

/// Загружает установщик в указанную папку и возвращает путь к нему.
///
/// `progress` зовётся по мере чтения: сколько пришло и сколько всего.
/// Уже загруженный установщик того же размера берётся с диска — сеть
/// для этого не нужна.
pub fn download(
    installer: &Installer,
    into: &Path,
    mut progress: impl FnMut(u64, u64),
) -> Result<PathBuf, UpdateError> {
    let target = into.join(&installer.name);

    if ready(&target, installer.size) {
        tracing::info!(файл = %target.display(), "установщик уже загружен");
        progress(installer.size, installer.size);
        return Ok(target);
    }

    std::fs::create_dir_all(into).map_err(|e| UpdateError::Io(e.to_string()))?;

    let part = target.with_extension("part");
    let written = fetch(&installer.url, &part, installer.size, &mut progress)?;

    // Недокачанный файл до имени установщика не допускаем: запускать
    // обрубок нельзя, а по имени его от целого не отличить.
    if installer.size > 0 && written != installer.size {
        let _ = std::fs::remove_file(&part);

        return Err(UpdateError::Incomplete {
            got: written,
            expected: installer.size,
        });
    }

    std::fs::rename(&part, &target).map_err(|e| UpdateError::Io(e.to_string()))?;

    tracing::info!(файл = %target.display(), байт = written, "установщик загружен");
    Ok(target)
}

/// Лежит ли готовый файл нужного размера.
fn ready(target: &Path, size: u64) -> bool {
    size > 0
        && std::fs::metadata(target)
            .map(|meta| meta.len() == size)
            .unwrap_or(false)
}

/// Читает ответ в файл, отчитываясь о ходе.
fn fetch(
    url: &str,
    part: &Path,
    total: u64,
    progress: &mut impl FnMut(u64, u64),
) -> Result<u64, UpdateError> {
    let response = super::net::downloading()
        .get(url)
        .header("User-Agent", super::AGENT)
        .call()
        .map_err(|e| match e {
            ureq::Error::StatusCode(code) => UpdateError::Refused(code),
            other => UpdateError::Network(other.to_string()),
        })?;

    let mut reader = response.into_body().into_reader();
    let mut file = File::create(part).map_err(|e| UpdateError::Io(e.to_string()))?;

    let mut buffer = vec![0_u8; CHUNK];
    let mut written = 0_u64;

    loop {
        let read = reader
            .read(&mut buffer)
            .map_err(|e| UpdateError::Network(e.to_string()))?;

        if read == 0 {
            break;
        }

        file.write_all(&buffer[..read])
            .map_err(|e| UpdateError::Io(e.to_string()))?;

        written += read as u64;
        progress(written, total.max(written));
    }

    file.flush().map_err(|e| UpdateError::Io(e.to_string()))?;

    Ok(written)
}
