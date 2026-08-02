//! Устройства вывода звука.
//!
//! Порт `AudioDeviceManager` из v4 (559 строк) — mpv отдаёт список и
//! принимает смену устройства на лету, своего перебора не нужно.

use crate::engine::Engine;
use crate::error::Result;

/// Значение mpv для «выбрать самому».
pub const AUTO_DEVICE: &str = "auto";

/// Устройство вывода звука.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AudioDevice {
    /// Внутреннее имя для свойства `audio-device`.
    pub name: String,
    /// Понятное название для меню.
    pub description: String,
}

impl AudioDevice {
    /// Подпись для меню.
    pub fn label(&self) -> String {
        if self.description.is_empty() {
            self.name.clone()
        } else {
            self.description.clone()
        }
    }

    /// Автоматический выбор — этот пункт mpv отдаёт первым в списке.
    pub fn is_auto(&self) -> bool {
        self.name == AUTO_DEVICE
    }
}

impl Engine {
    /// Устройства вывода звука, доступные сейчас.
    ///
    /// Читается по частям, как `track-list`: разбирать узел целиком
    /// не нужно, mpv отдаёт подсвойства строками.
    pub fn audio_devices(&self) -> Vec<AudioDevice> {
        let count = self.property_i64("audio-device-list/count").unwrap_or(0);

        (0..count)
            .filter_map(|index| {
                let name = self
                    .property_string(&format!("audio-device-list/{index}/name"))
                    .ok()?;

                if name.is_empty() {
                    return None;
                }

                Some(AudioDevice {
                    description: self
                        .property_string(&format!("audio-device-list/{index}/description"))
                        .unwrap_or_default(),
                    name,
                })
            })
            .collect()
    }

    /// Выбранное устройство. `auto` означает выбор самим mpv.
    pub fn audio_device(&self) -> String {
        self.property_string("audio-device")
            .unwrap_or_else(|_| AUTO_DEVICE.to_string())
    }

    /// Переключает вывод звука.
    ///
    /// mpv сам пересоздаёт звуковой выход, перезапуск плеера не нужен.
    pub fn set_audio_device(&mut self, name: &str) -> Result<()> {
        self.set_property_string("audio-device", name)?;
        tracing::info!(устройство = name, "вывод звука переключён");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn устройство(name: &str, description: &str) -> AudioDevice {
        AudioDevice {
            name: name.into(),
            description: description.into(),
        }
    }

    #[test]
    fn подпись_берётся_из_описания() {
        let device = устройство("wasapi/{0.0.0}", "Динамики (Realtek)");
        assert_eq!(device.label(), "Динамики (Realtek)");
    }

    #[test]
    fn без_описания_подписью_служит_имя() {
        assert_eq!(устройство("wasapi/x", "").label(), "wasapi/x");
    }

    #[test]
    fn автоматический_выбор_узнаётся_по_имени() {
        assert!(устройство("auto", "Autoselect device").is_auto());
        assert!(!устройство("wasapi/x", "Динамики").is_auto());
    }
}
