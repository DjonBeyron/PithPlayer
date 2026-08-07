//! Один экземпляр плеера на систему.
//!
//! Второй запуск не открывает новое окно, а передаёт путь уже работающему
//! плееру и завершается (PLAN.md §6.7).
//!
//! Связь идёт через локальный сокет: порт записывается в файл рядом
//! с данными. Так обходимся без вызовов WinAPI и без `unsafe`.

use std::io::{BufRead, BufReader, Write};
use std::net::{Ipv4Addr, SocketAddr, TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, Sender, channel};

use pith_store::DataPaths;

/// Файл с номером порта работающего экземпляра.
const PORT_FILE: &str = "instance.port";

/// Сколько ждать соединения с работающим экземпляром.
///
/// Соединение идёт на себя же, и слушает его отдельный поток: живой плеер
/// отвечает за доли миллисекунды, независимо от того, чем занят интерфейс.
/// Мёртвый порт брандмауэр просто молчит, и всё это время плеер стоял бы
/// с пустым окном. Прежние 600 мс были почти половиной времени запуска.
const CONNECT_TIMEOUT: std::time::Duration = std::time::Duration::from_millis(150);

/// Приёмник путей к файлам от других запусков плеера.
pub struct InstanceServer {
    receiver: Receiver<PathBuf>,
}

impl InstanceServer {
    /// Пути, присланные другими запусками с прошлого кадра.
    pub fn pending_files(&self) -> Vec<PathBuf> {
        self.receiver.try_iter().collect()
    }
}

/// Пытается стать основным экземпляром.
///
/// Возвращает `None`, если плеер уже запущен и путь ему передан —
/// тогда этот процесс должен просто завершиться.
///
/// При любой неполадке связи считаем себя основным: лучше открыть второе
/// окно, чем не открыть файл вовсе.
pub fn become_primary_or_forward(paths: &DataPaths, file: Option<&str>) -> Option<InstanceServer> {
    if let Some(port) = read_port(paths) {
        if forward_to_primary(port, file) {
            tracing::info!(port, "плеер уже запущен, файл передан ему");
            return None;
        }

        // Порт остался от прошлого запуска. Убираем запись сразу: иначе
        // каждый следующий запуск снова ждёт соединения с мертвецом.
        tracing::debug!(port, "порт прошлого запуска не отвечает");
        release(paths);
    }

    let listener = match TcpListener::bind(SocketAddr::from((Ipv4Addr::LOCALHOST, 0))) {
        Ok(listener) => listener,
        Err(e) => {
            tracing::warn!(error = %e, "не удалось занять локальный порт, работаю как отдельное окно");
            return Some(InstanceServer {
                receiver: channel().1,
            });
        }
    };

    let port = listener.local_addr().map(|a| a.port()).unwrap_or(0);
    write_port(paths, port);

    let (sender, receiver) = channel();
    spawn_listener(listener, sender);

    tracing::info!(port, "этот экземпляр стал основным");
    Some(InstanceServer { receiver })
}

/// Убирает файл с портом при выходе.
pub fn release(paths: &DataPaths) {
    let _ = std::fs::remove_file(paths.root().join(PORT_FILE));
}

/// Принимает подключения в фоне и складывает пути в канал.
fn spawn_listener(listener: TcpListener, sender: Sender<PathBuf>) {
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(stream) = stream else { continue };

            let mut line = String::new();
            if BufReader::new(stream).read_line(&mut line).is_err() {
                continue;
            }

            if let Some(path) = parse_message(&line) {
                tracing::info!(?path, "получен файл от другого запуска");
                if sender.send(path).is_err() {
                    // Приложение закрылось — слушать больше некому.
                    break;
                }
            }
        }
    });
}

/// Передаёт путь работающему экземпляру.
fn forward_to_primary(port: u16, file: Option<&str>) -> bool {
    let address = SocketAddr::from((Ipv4Addr::LOCALHOST, port));

    let Ok(mut stream) = TcpStream::connect_timeout(&address, CONNECT_TIMEOUT) else {
        // Порт остался от прошлого запуска, который уже закрыт.
        return false;
    };

    let message = format!("{}\n", file.unwrap_or_default());
    stream.write_all(message.as_bytes()).is_ok()
}

/// Разбирает сообщение: строка с путём либо пусто.
fn parse_message(line: &str) -> Option<PathBuf> {
    let trimmed = line.trim();
    (!trimmed.is_empty()).then(|| PathBuf::from(trimmed))
}

fn port_file(paths: &DataPaths) -> PathBuf {
    paths.root().join(PORT_FILE)
}

fn read_port(paths: &DataPaths) -> Option<u16> {
    std::fs::read_to_string(port_file(paths))
        .ok()?
        .trim()
        .parse()
        .ok()
}

fn write_port(paths: &DataPaths, port: u16) {
    if let Err(e) = paths.ensure_exists() {
        tracing::warn!(error = %e, "не удалось создать каталог данных");
        return;
    }

    if let Err(e) = std::fs::write(port_file(paths), port.to_string()) {
        tracing::warn!(error = %e, "не удалось записать номер порта");
    }
}

/// Существует ли файл — плеер не должен пытаться открыть несуществующее.
pub fn is_openable(path: &Path) -> bool {
    path.is_file()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn разбирает_путь_из_сообщения() {
        assert_eq!(
            parse_message("C:\\видео\\фильм.mkv\n"),
            Some(PathBuf::from("C:\\видео\\фильм.mkv"))
        );
    }

    #[test]
    fn пустое_сообщение_не_содержит_пути() {
        assert_eq!(parse_message("\n"), None);
        assert_eq!(parse_message("   "), None);
        assert_eq!(parse_message(""), None);
    }

    #[test]
    fn пробелы_по_краям_убираются() {
        assert_eq!(
            parse_message("  C:\\фильм.mp4  \n"),
            Some(PathBuf::from("C:\\фильм.mp4"))
        );
    }

    /// Второй запуск должен достучаться до первого и передать путь.
    #[test]
    fn передаёт_файл_работающему_экземпляру() {
        let dir = tempfile::tempdir().expect("временный каталог");
        let paths = DataPaths::with_root(dir.path());

        let server = become_primary_or_forward(&paths, None).expect("первый запуск — основной");

        let forwarded = become_primary_or_forward(&paths, Some("C:\\кино.mkv"));
        assert!(forwarded.is_none(), "второй запуск обязан передать файл");

        // Передача идёт через поток, поэтому даём ему сработать.
        std::thread::sleep(std::time::Duration::from_millis(300));

        let files = server.pending_files();
        assert_eq!(files, vec![PathBuf::from("C:\\кино.mkv")]);
    }

    /// Порт от закрытого экземпляра не должен мешать запуску.
    #[test]
    fn устаревший_порт_не_блокирует_запуск() {
        let dir = tempfile::tempdir().expect("временный каталог");
        let paths = DataPaths::with_root(dir.path());

        // Порт, на котором заведомо никто не слушает.
        std::fs::create_dir_all(dir.path()).expect("каталог");
        std::fs::write(dir.path().join(PORT_FILE), "1").expect("запись порта");

        assert!(
            become_primary_or_forward(&paths, Some("C:\\кино.mkv")).is_some(),
            "при недоступном порте плеер обязан запуститься сам"
        );
    }
}
