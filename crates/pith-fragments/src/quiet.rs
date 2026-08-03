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
