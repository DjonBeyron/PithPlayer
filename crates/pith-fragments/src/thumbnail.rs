//! Кадр для предпросмотра при перемотке.
//!
//! Пока пользователь тянет ползунок, полезно видеть, что там происходит.
//! Кадр достаёт `ffmpeg` — отдельно от воспроизведения, поэтому картинка
//! в окне при этом не дёргается.

use std::path::Path;

/// Ширина миниатюры, точки. Высота считается по пропорциям кадра.
const WIDTH: u32 = 240;

/// Достаёт один кадр в указанной секунде.
///
/// Возвращает изображение в формате PNG. `None` означает, что `ffmpeg`
/// недоступен или кадра в этом месте нет.
pub fn grab_frame(video: &Path, position: f64) -> Option<Vec<u8>> {
    let output = crate::quiet::command("ffmpeg")
        .args(["-v", "error"])
        // Перемотка до `-i` и по опорным кадрам: точность здесь не нужна,
        // а декодировать от ближайшего опорного кадра — секунды.
        .args(["-ss", &crate::command::format_time(position.max(0.0))])
        .arg("-i")
        .arg(video)
        .args(["-frames:v", "1"])
        .args(["-vf", &format!("scale={WIDTH}:-2")])
        // Один кадр в поток вывода, без временного файла.
        .args(["-f", "image2", "-c:v", "png", "-"])
        .output()
        .ok()?;

    if !output.status.success() || output.stdout.is_empty() {
        return None;
    }

    Some(output.stdout)
}
