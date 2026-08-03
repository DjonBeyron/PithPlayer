//! Запуск внешних программ без консольного окна.
//!
//! `ffmpeg` и `ffprobe` — консольные приложения. Windows открывает им
//! окно консоли, и оно мелькает поверх плеера при каждом запуске: при
//! проверке наличия FFmpeg на старте и на каждый вырезанный отрезок.
//! Флаг `CREATE_NO_WINDOW` это убирает, не мешая читать вывод.

use std::process::Command;

/// Windows: не создавать окно консоли для дочернего процесса.
#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

/// Windows: пониженный приоритет для вспомогательной работы.
#[cfg(windows)]
const BELOW_NORMAL_PRIORITY_CLASS: u32 = 0x0000_4000;

/// Команда, которая не показывает консольное окно.
pub fn command(program: &str) -> Command {
    let mut command = Command::new(program);

    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(CREATE_NO_WINDOW);
    }

    command
}

/// То же, но с пониженным приоритетом.
///
/// Для подсобной работы вроде кадра предпросмотра: она не должна
/// отбирать процессор у воспроизведения. Плеер обязан оставаться
/// плавным, даже если кадр появится чуть позже.
pub fn background_command(program: &str) -> Command {
    let mut command = Command::new(program);

    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(CREATE_NO_WINDOW | BELOW_NORMAL_PRIORITY_CLASS);
    }

    command
}
