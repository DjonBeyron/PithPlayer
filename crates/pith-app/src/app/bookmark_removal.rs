//! Удаление закладки с таймером: нажал «убрать» — через 5 секунд закладка
//! исчезнет, если не отменить повторным нажатием.
//!
//! Таймер живёт в приложении, а не в окне панели: закрыли панель — закладка
//! всё равно будет удалена в срок.

use super::PithApp;

/// Сколько секунд даётся на отмену.
pub const REMOVAL_DELAY: f64 = 5.0;

/// Закладка, поставленная в очередь на удаление.
#[derive(Debug, Clone)]
pub struct PendingRemoval {
    /// Видео, которому принадлежит закладка (имя файла без расширения).
    video: String,
    /// Список закладки.
    list: String,
    time_ms: i64,
    /// Когда удалить, по времени кадра.
    deadline: f64,
}

impl PithApp {
    /// Сколько секунд осталось до удаления закладки активного списка.
    pub fn removal_left(&self, time_ms: i64) -> Option<f64> {
        let video = self.current_video_name()?;
        let list = self.current_bookmarks()?.active_list.clone();

        self.pending_removals
            .iter()
            .find(|r| r.video == video && r.list == list && r.time_ms == time_ms)
            .map(|r| (r.deadline - self.frame_time).max(0.0))
    }

    /// Нажатие на «убрать»: ставит закладку в очередь или отменяет удаление.
    pub fn toggle_removal(&mut self, time_ms: i64) {
        let Some(video) = self.current_video_name() else {
            return;
        };
        let Some(list) = self.current_bookmarks().map(|v| v.active_list.clone()) else {
            return;
        };

        if self.removal_left(time_ms).is_some() {
            self.pending_removals
                .retain(|r| !(r.video == video && r.list == list && r.time_ms == time_ms));
            self.show_notice(crate::tr!("Удаление отменено", "Removal cancelled"));
            return;
        }

        self.pending_removals.push(PendingRemoval {
            video,
            list,
            time_ms,
            deadline: self.frame_time + REMOVAL_DELAY,
        });
    }

    /// Каждый кадр: удаляет те, у кого вышел срок, и не даёт окну уснуть,
    /// пока таймеры идут.
    pub(super) fn process_removals(&mut self, ctx: &egui::Context) {
        if self.pending_removals.is_empty() {
            return;
        }

        let now = self.frame_time;
        let (due, waiting): (Vec<_>, Vec<_>) = std::mem::take(&mut self.pending_removals)
            .into_iter()
            .partition(|r| r.deadline <= now);
        self.pending_removals = waiting;

        for removal in due {
            self.remove_now(&removal);
        }

        if !self.pending_removals.is_empty() {
            ctx.request_repaint_after(std::time::Duration::from_millis(200));
        }
    }

    fn remove_now(&mut self, removal: &PendingRemoval) {
        let duration = self.settings.fragments.duration_sec;
        let buffer = self.settings.fragments.buffer_sec;

        let video = self
            .bookmarks
            .for_video_mut(&removal.video, duration, buffer);

        if let Some(list) = video.find_mut(&removal.list)
            && list.remove_at(removal.time_ms)
        {
            self.bookmarks.save();
            tracing::info!(время = removal.time_ms, "закладка удалена по таймеру");
        }
    }
}
