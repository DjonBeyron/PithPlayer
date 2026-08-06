//! Поток, который достаёт точные кадры предпросмотра.
//!
//! Второй экземпляр mpv держит файл открытым, но его команды блокируют
//! поток: перемотка и снимок занимают десятки миллисекунд, и делать их
//! в кадре интерфейса нельзя. Поэтому экземпляр живёт в своём потоке,
//! а интерфейс только оставляет ему пожелание и забирает готовое.
//!
//! Пожелание всегда одно: пока рука ведёт мышь, промежуточные места
//! не нужны — важно последнее.

use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::sync::{Arc, Condvar, Mutex};
use std::thread::JoinHandle;

/// Что поток должен сделать дальше.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Request {
    /// Ждать пожелания.
    Idle,
    /// Достать кадр этой секунды.
    Frame(f64),
    /// Закончить работу.
    Stop,
}

/// Общее место для пожеланий: интерфейс кладёт, поток забирает.
type Wanted = Arc<(Mutex<Request>, Condvar)>;

/// Источник точных кадров текущего файла.
pub struct FrameSource {
    wanted: Wanted,
    frames: Receiver<(f64, Vec<u8>)>,
    worker: Option<JoinHandle<()>>,
}

impl FrameSource {
    /// Запускает поток для файла. `repaint` будит интерфейс, когда кадр готов.
    pub fn spawn(path: &Path, repaint: impl Fn() + Send + 'static) -> Self {
        let wanted: Wanted = Arc::new((Mutex::new(Request::Idle), Condvar::new()));
        let (sender, frames) = channel();

        let path = path.to_path_buf();
        let thread_wanted = Arc::clone(&wanted);

        let worker = std::thread::Builder::new()
            .name("предпросмотр".into())
            .spawn(move || run(&path, &thread_wanted, &sender, repaint))
            .ok();

        Self {
            wanted,
            frames,
            worker,
        }
    }

    /// Просит кадр этой секунды. Прошлое неотработанное пожелание забывается.
    pub fn request(&self, time: f64) {
        let (lock, signal) = &*self.wanted;

        if let Ok(mut request) = lock.lock() {
            // Остановку не перебиваем: поток уже уходит.
            if *request == Request::Stop {
                return;
            }

            *request = Request::Frame(time);
            signal.notify_one();
        }
    }

    /// Забирает готовый кадр, если он есть.
    pub fn take_frame(&self) -> Option<(f64, Vec<u8>)> {
        // Копившиеся кадры устарели: показываем самый свежий.
        let mut last = None;
        while let Ok(frame) = self.frames.try_recv() {
            last = Some(frame);
        }
        last
    }
}

impl Drop for FrameSource {
    fn drop(&mut self) {
        {
            let (lock, signal) = &*self.wanted;
            if let Ok(mut request) = lock.lock() {
                *request = Request::Stop;
            }
            signal.notify_all();
        }

        // Ждём поток: он держит экземпляр mpv, а тот — открытый файл.
        // Брошенный на произвол, он мешал бы закрытию плеера.
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

/// Тело потока: открыть файл и отвечать на пожелания, пока не попросят уйти.
fn run(path: &Path, wanted: &Wanted, sender: &Sender<(f64, Vec<u8>)>, repaint: impl Fn()) {
    let mut engine = match pith_mpv::PreviewEngine::new(shot_path()) {
        Ok(engine) => engine,
        Err(e) => {
            tracing::warn!(error = %e, "предпросмотр: экземпляр mpv не запустился");
            return;
        }
    };

    if let Err(e) = engine.load_file(path) {
        tracing::warn!(error = %e, "предпросмотр: файл не открылся");
        return;
    }

    while let Some(time) = next_request(wanted) {
        match engine.grab(time) {
            Ok(data) => {
                if sender.send((time, data)).is_err() {
                    // Интерфейс больше не слушает — работать не для кого.
                    break;
                }
                repaint();
            }
            Err(e) => tracing::debug!(time, error = %e, "предпросмотр: кадр не получен"),
        }
    }

    tracing::debug!("предпросмотр: поток завершён");
}

/// Ждёт следующее пожелание. `None` — пора уходить.
fn next_request(wanted: &Wanted) -> Option<f64> {
    let (lock, signal) = &**wanted;
    let mut request = lock.lock().ok()?;

    loop {
        match *request {
            Request::Stop => return None,
            Request::Frame(time) => {
                // Забираем: пока идёт съёмка, интерфейс успеет попросить
                // другое место, и оно не должно потеряться.
                *request = Request::Idle;
                return Some(time);
            }
            Request::Idle => request = signal.wait(request).ok()?,
        }
    }
}

/// Свой файл снимков у каждого запуска плеера.
///
/// Два открытых плеера писали бы в один и тот же файл и показывали бы
/// кадры друг друга.
fn shot_path() -> PathBuf {
    std::env::temp_dir().join(format!("pith_preview_{}.jpg", std::process::id()))
}

#[cfg(test)]
mod tests {
    use super::{Request, Wanted, next_request};
    use std::sync::{Arc, Condvar, Mutex};

    fn wanted(request: Request) -> Wanted {
        Arc::new((Mutex::new(request), Condvar::new()))
    }

    #[test]
    fn пожелание_забирается_один_раз() {
        let wanted = wanted(Request::Frame(12.5));

        assert_eq!(next_request(&wanted), Some(12.5));
        assert_eq!(*wanted.0.lock().unwrap(), Request::Idle);
    }

    #[test]
    fn остановка_прекращает_ожидание() {
        let wanted = wanted(Request::Stop);
        assert_eq!(next_request(&wanted), None);
    }

    #[test]
    fn новое_пожелание_вытесняет_прежнее() {
        let wanted = wanted(Request::Frame(3.0));

        {
            let (lock, _) = &*wanted;
            *lock.lock().unwrap() = Request::Frame(90.0);
        }

        assert_eq!(next_request(&wanted), Some(90.0), "важно последнее место");
    }
}
