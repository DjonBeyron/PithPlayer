//! Цвет заголовка окна.
//!
//! Windows 11 красит заголовок материалом Mica: он подмешивает размытое
//! содержимое того, что лежит за окном. Из-за этого панель отливает то
//! синим, то зелёным — смотря какие обои. Плеер тёмный, и заголовок должен
//! быть таким же, как его нижняя панель, а не отражением рабочего стола.
//!
//! Другого пути, кроме DWM, нет: ни winit, ни eframe цвет заголовка не
//! задают. Отсюда единственный `unsafe` в приложении — вызов
//! `DwmSetWindowAttribute`. Он ничего не читает и не выделяет памяти:
//! передаёт системе четыре байта цвета для нашего же окна.
//!
//! На Windows 10 таких свойств нет — вызов возвращает ошибку, мы её
//! проглатываем, и заголовок остаётся системным. Это не повод падать.

use crate::theme;

/// Красит заголовок, его текст и рамку под тему плеера.
#[cfg(windows)]
pub fn apply(window: &impl raw_window_handle::HasWindowHandle) {
    use raw_window_handle::RawWindowHandle;
    use windows_sys::Win32::Graphics::Dwm::{
        DWMWA_BORDER_COLOR, DWMWA_CAPTION_COLOR, DWMWA_TEXT_COLOR,
    };

    let Ok(handle) = window.window_handle() else {
        tracing::debug!("окно не отдало свой описатель — заголовок остаётся системным");
        return;
    };

    let RawWindowHandle::Win32(win32) = handle.as_raw() else {
        return;
    };

    let hwnd = win32.hwnd.get() as windows_sys::Win32::Foundation::HWND;

    // Тот же цвет, что у нижней панели: окно должно выглядеть цельным.
    set(hwnd, DWMWA_CAPTION_COLOR, colorref(theme::PANEL_BG));
    set(hwnd, DWMWA_TEXT_COLOR, colorref(theme::TEXT_PRIMARY));
    // Рамка чуть светлее: без неё тёмное окно сливается с тёмным фоном.
    set(hwnd, DWMWA_BORDER_COLOR, colorref(theme::PANEL_BORDER));

    tracing::debug!("заголовок окна перекрашен под тему");
}

#[cfg(not(windows))]
pub fn apply(_window: &impl raw_window_handle::HasWindowHandle) {}

/// Задаёт одно свойство окна. Неудача не важна: на старых Windows
/// таких свойств просто нет.
#[cfg(windows)]
fn set(hwnd: windows_sys::Win32::Foundation::HWND, attribute: i32, value: u32) {
    // SAFETY: описатель получен от самого окна и живёт дольше вызова,
    // указатель ведёт на четыре байта на стеке, размер передан честно.
    let result = unsafe {
        windows_sys::Win32::Graphics::Dwm::DwmSetWindowAttribute(
            hwnd,
            attribute as u32,
            (&raw const value).cast(),
            size_of::<u32>() as u32,
        )
    };

    if result != 0 {
        tracing::debug!(attribute, result, "система не приняла цвет заголовка");
    }
}

/// Цвет в виде, который понимает GDI: `0x00BBGGRR`.
fn colorref(color: egui::Color32) -> u32 {
    let [red, green, blue, _] = color.to_array();

    u32::from(red) | (u32::from(green) << 8) | (u32::from(blue) << 16)
}

#[cfg(test)]
mod tests {
    use super::colorref;

    #[test]
    fn цвет_переставляет_байты_под_gdi() {
        // GDI хранит цвет наоборот: синий в старшем байте, красный в младшем.
        let color = egui::Color32::from_rgb(0x12, 0x34, 0x56);
        assert_eq!(colorref(color), 0x00_56_34_12);
    }

    #[test]
    fn прозрачность_в_цвет_не_попадает() {
        let color = egui::Color32::from_rgba_premultiplied(0x10, 0x20, 0x30, 0x40);
        assert_eq!(colorref(color) & 0xFF00_0000, 0);
    }
}
