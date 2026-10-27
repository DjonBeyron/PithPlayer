//! Режим разработчика: открывается тремя щелчками по номеру версии
//! в окне обновления. Пока там одна настройка — папка для журналов.
//!
//! Значения живут в `dev.json` (`pith_store::DevSettings`) и применяются
//! при следующем запуске: логирование стартует раньше всего остального.

use std::time::{Duration, Instant};

use pith_store::DevSettings;

use super::PithApp;

/// Сколько щелчков открывают режим.
const CLICKS_TO_OPEN: u8 = 3;

/// Щелчки, разделённые дольше этого срока, считаются заново.
const CLICK_WINDOW: Duration = Duration::from_millis(1500);

#[derive(Default)]
pub struct DevPanel {
    pub open: bool,
    /// Папка журналов, как её набрали в поле.
    pub log_dir: String,
    /// Настройки записаны, ждут перезапуска.
    pub saved: bool,
    clicks: u8,
    last_click: Option<Instant>,
}

impl PithApp {
    pub fn dev_panel(&mut self) -> &mut DevPanel {
        &mut self.update.dev
    }

    /// Щелчок по номеру версии: третий подряд открывает режим.
    pub fn dev_version_clicked(&mut self) {
        let dev = &mut self.update.dev;
        let now = Instant::now();

        let fresh = dev
            .last_click
            .is_some_and(|t| now.duration_since(t) <= CLICK_WINDOW);
        dev.clicks = if fresh { dev.clicks + 1 } else { 1 };
        dev.last_click = Some(now);

        if dev.clicks >= CLICKS_TO_OPEN {
            dev.clicks = 0;
            dev.open = !dev.open;

            if dev.open {
                dev.log_dir = DevSettings::load(&self.data_paths).log_dir;
                dev.saved = false;
            }
        }
    }

    /// Окно выбора папки для журналов.
    pub fn dev_pick_folder(&mut self) {
        let start = self.update.dev.log_dir.clone();
        let mut dialog = rfd::FileDialog::new();

        if !start.trim().is_empty() {
            dialog = dialog.set_directory(start.trim());
        }

        if let Some(dir) = dialog.pick_folder() {
            self.update.dev.log_dir = dir.to_string_lossy().into_owned();
            self.update.dev.saved = false;
        }
    }

    /// Записывает папку журналов.
    pub fn dev_save(&mut self) {
        let settings = DevSettings {
            log_dir: self.update.dev.log_dir.trim().to_string(),
        };

        settings.save(&self.data_paths);
        self.update.dev.saved = true;

        tracing::info!(папка = %settings.log_dir, "папка журналов сохранена");
    }
}
