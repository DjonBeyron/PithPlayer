//! Место дочерних окон между запусками.
//!
//! Окна актёров и выгрузки — отдельные окна системы, и открываться они
//! должны там же, где их закрыли: хоть на втором мониторе. Главное окно
//! делает то же самое (`window_state.rs`), но у дочерних своё устройство:
//! их описание собирается заново на каждом показе.
//!
//! Положение берётся внешнее, размер — внутренний: именно в таком виде
//! их задают при создании. Смешаешь — окно будет расти на высоту заголовка
//! при каждом запуске.

use pith_store::WindowGeometry;

/// Достраивает описание окна сохранённым местом — **только при показе**.
///
/// Описание окна собирается заново на каждом кадре, и egui сравнивает его
/// с прежним: что изменилось — то и приказывает окну. Если каждый кадр
/// называть размер, окно будет спорить с пользователем — тот тянет за край,
/// а мы возвращаем сохранённый размер, и окно дёргается. То же после
/// разворота на весь экран и возврата обратно.
///
/// Поэтому место называется один раз, а дальше окно живёт само: `placed`
/// уже поднят, и размера с положением в описании больше нет.
///
/// Ничего не сохранено или запись бессмысленна — окно откроется там,
/// где решит система, размером по умолчанию.
pub(super) fn place(
    builder: egui::ViewportBuilder,
    saved: Option<WindowGeometry>,
    default_size: [f32; 2],
    placed: &mut bool,
) -> egui::ViewportBuilder {
    if *placed {
        return builder;
    }
    *placed = true;

    let Some(geometry) = saved.filter(WindowGeometry::is_sane) else {
        return builder.with_inner_size(default_size);
    };

    builder
        .with_position(egui::pos2(geometry.x, geometry.y))
        .with_inner_size([geometry.width, geometry.height])
}

/// Где сейчас стоит окно. Вызывается изнутри его же контекста.
///
/// `None`, пока система не сообщила размеров: на первом кадре их ещё нет,
/// а у свёрнутого окна они бессмысленны.
pub(super) fn geometry(ctx: &egui::Context) -> Option<WindowGeometry> {
    let (outer, inner) = ctx.input(|i| (i.viewport().outer_rect, i.viewport().inner_rect));
    let (outer, inner) = (outer?, inner?);

    if inner.width() < 1.0 || inner.height() < 1.0 {
        return None;
    }

    Some(WindowGeometry {
        x: outer.min.x,
        y: outer.min.y,
        width: inner.width(),
        height: inner.height(),
    })
}
