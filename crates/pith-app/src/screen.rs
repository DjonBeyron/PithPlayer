//! Рабочая область экрана — та его часть, что остаётся от панели задач.
//!
//! Нужна одному месту: окну, закрытому развёрнутым. Оконная система
//! разворачивает окно с анимацией, и пользователь видит, как оно
//! разъезжается на весь экран. Открыв окно сразу по рабочей области,
//! мы делаем эту анимацию незаметной: разъезжаться уже некуда.
//!
//! Обойтись без Win32 нельзя: eframe отдаёт размеры монитора только
//! в кадре, а окно создаётся до первого кадра. Вызовы здесь читающие —
//! ничего не меняют и не выделяют памяти.

/// Рабочая область монитора, на котором лежит точка.
///
/// `None` — система не ответила: окно тогда откроется как обычно,
/// это штатный исход, а не ошибка.
#[cfg(windows)]
pub fn work_area(x: f32, y: f32) -> Option<(egui::Pos2, egui::Vec2)> {
    use windows_sys::Win32::Foundation::POINT;
    use windows_sys::Win32::Graphics::Gdi::{
        GetMonitorInfoW, MONITOR_DEFAULTTONEAREST, MONITORINFO, MonitorFromPoint,
    };

    let point = POINT {
        x: x as i32,
        y: y as i32,
    };

    let mut info = MONITORINFO {
        cbSize: size_of::<MONITORINFO>() as u32,
        ..unsafe { std::mem::zeroed() }
    };

    // SAFETY: обе функции только читают. Монитор ищется ближайший, поэтому
    // описатель всегда пригоден; структура заполнена своим размером, как
    // того требует GetMonitorInfoW.
    let ok = unsafe {
        let monitor = MonitorFromPoint(point, MONITOR_DEFAULTTONEAREST);
        GetMonitorInfoW(monitor, &raw mut info) != 0
    };

    if !ok {
        tracing::debug!("рабочая область экрана недоступна");
        return None;
    }

    let area = info.rcWork;
    let size = egui::vec2(
        (area.right - area.left) as f32,
        (area.bottom - area.top) as f32,
    );

    if size.x < 1.0 || size.y < 1.0 {
        return None;
    }

    Some((egui::pos2(area.left as f32, area.top as f32), size))
}

/// Насколько внешний размер окна больше внутреннего: рамка и заголовок.
#[cfg(windows)]
fn frame() -> egui::Vec2 {
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        GetSystemMetrics, SM_CXPADDEDBORDER, SM_CXSIZEFRAME, SM_CYCAPTION,
    };

    // SAFETY: чтение системных величин, ничего не меняет и не выделяет.
    let (padding, side, caption) = unsafe {
        (
            GetSystemMetrics(SM_CXPADDEDBORDER),
            GetSystemMetrics(SM_CXSIZEFRAME),
            GetSystemMetrics(SM_CYCAPTION),
        )
    };

    let border = side + padding;

    egui::vec2((2 * border) as f32, (caption + 2 * border) as f32)
}

/// Каким создать окно, чтобы оно выглядело развёрнутым.
///
/// Возвращает положение и **внутренний** размер: именно его задают окну
/// при создании.
///
/// Повторяем то, что делает с окном сама Windows, разворачивая его:
/// прямоугольник окна выходит за рабочую область на толщину рамки —
/// у рамки боковые и нижняя части невидимы, и без этого запаса по краям
/// экрана остаются щели (замер: развёрнутое кнопкой окно 1936×1048
/// в точке −8,−8 при рабочей области 1920×1032 в нуле).
#[cfg(windows)]
pub fn full_screen_window(x: f32, y: f32) -> Option<(egui::Pos2, egui::Vec2)> {
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        GetSystemMetrics, SM_CXPADDEDBORDER, SM_CXSIZEFRAME, SM_CYCAPTION,
    };

    let (position, area) = work_area(x, y)?;

    // SAFETY: чтение системных величин, ничего не меняет и не выделяет.
    let (padding, side, caption) = unsafe {
        (
            GetSystemMetrics(SM_CXPADDEDBORDER),
            GetSystemMetrics(SM_CXSIZEFRAME),
            GetSystemMetrics(SM_CYCAPTION),
        )
    };

    // Насколько развёрнутое окно вылезает за край экрана с каждой стороны.
    let border = (side + padding) as f32;

    // Ширина внутренней части совпадает с рабочей областью: запас по бокам
    // ровно съедается рамкой. По высоте от неё отнимается заголовок.
    let inner = egui::vec2(area.x, area.y - caption as f32);

    (inner.x > 1.0 && inner.y > 1.0)
        .then_some((egui::pos2(position.x - border, position.y - border), inner))
}

#[cfg(not(windows))]
pub fn full_screen_window(_x: f32, _y: f32) -> Option<(egui::Pos2, egui::Vec2)> {
    None
}

/// Объявляет окно развёрнутым и задаёт, куда его сворачивать.
///
/// Окно мы открываем во весь экран сами, и система об этом не знает:
/// кнопка заголовка предлагает «развернуть», а сворачивать ей некуда —
/// она вернула бы окно к тому размеру, каким его создали, то есть снова
/// во весь экран.
///
/// Здесь мы сообщаем системе и то, и другое разом: окно развёрнуто,
/// а свернуть его нужно в `restore` — размер, который был до разворота.
/// Двигать при этом нечего: окно уже занимает в точности тот прямоугольник,
/// каким его сделал бы разворот.
#[cfg(windows)]
pub fn mark_maximized(window: &impl raw_window_handle::HasWindowHandle, restore: egui::Rect) {
    use raw_window_handle::RawWindowHandle;
    use windows_sys::Win32::Foundation::{POINT, RECT};
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        SW_SHOWMAXIMIZED, SetWindowPlacement, WINDOWPLACEMENT,
    };

    let Ok(handle) = window.window_handle() else {
        return;
    };

    let RawWindowHandle::Win32(win32) = handle.as_raw() else {
        return;
    };

    // В настройках записан внутренний размер окна, а система ждёт внешний:
    // без рамки и заголовка окно возвращалось бы каждый раз чуть меньше.
    let outer = egui::Rect::from_min_size(restore.min, restore.size() + frame());

    let placement = WINDOWPLACEMENT {
        length: size_of::<WINDOWPLACEMENT>() as u32,
        flags: 0,
        showCmd: SW_SHOWMAXIMIZED as u32,
        ptMinPosition: POINT { x: -1, y: -1 },
        ptMaxPosition: POINT { x: -1, y: -1 },
        rcNormalPosition: RECT {
            left: outer.min.x as i32,
            top: outer.min.y as i32,
            right: outer.max.x as i32,
            bottom: outer.max.y as i32,
        },
    };

    // SAFETY: описатель получен от самого окна, структура заполнена целиком
    // и живёт дольше вызова, её размер передан честно.
    let ok = unsafe {
        SetWindowPlacement(
            win32.hwnd.get() as windows_sys::Win32::Foundation::HWND,
            &raw const placement,
        ) != 0
    };

    tracing::debug!(получилось = ok, ?restore, "окно объявлено развёрнутым");
}

#[cfg(not(windows))]
pub fn mark_maximized(_window: &impl raw_window_handle::HasWindowHandle, _restore: egui::Rect) {}

#[cfg(not(windows))]
pub fn work_area(_x: f32, _y: f32) -> Option<(egui::Pos2, egui::Vec2)> {
    None
}
