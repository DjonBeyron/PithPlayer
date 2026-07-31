//! Контекст отрисовки mpv поверх OpenGL.
//!
//! mpv рисует кадр в переданный framebuffer, приложение композитит поверх
//! свой интерфейс (PLAN.md §3).

use std::ffi::{CStr, CString, c_void};
use std::ptr;
use std::sync::Arc;

use libmpv2::Mpv;
use libmpv2::render::{
    OpenGLInitParams, RenderContext as MpvRenderContext, RenderParam, RenderParamApiType,
};

use crate::engine::Engine;
use crate::error::{MpvError, Result};

/// Загрузчик адресов функций OpenGL.
///
/// Тип совпадает с тем, что отдаёт eframe в `CreationContext::get_proc_address`,
/// поэтому приложение передаёт его напрямую.
pub type ProcAddressLoader = Arc<dyn Fn(&CStr) -> *const c_void + Send + Sync>;

/// Размер целевого буфера в пикселях.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FrameSize {
    pub width: i32,
    pub height: i32,
}

impl FrameSize {
    pub fn new(width: i32, height: i32) -> Self {
        Self {
            width: width.max(1),
            height: height.max(1),
        }
    }
}

/// Мост между сигнатурой libmpv и загрузчиком eframe.
///
/// libmpv требует именно указатель на функцию, а не замыкание, поэтому
/// загрузчик передаётся отдельным параметром контекста.
fn resolve_proc_address(loader: &ProcAddressLoader, name: &str) -> *mut c_void {
    match CString::new(name) {
        Ok(c_name) => loader(&c_name) as *mut c_void,
        // Имя с нулевым байтом прийти не может, но паниковать в обратном
        // вызове из C нельзя ни при каких условиях.
        Err(_) => ptr::null_mut(),
    }
}

/// Обёртка над контекстом отрисовки mpv.
pub struct RenderContext {
    inner: MpvRenderContext<'static>,
}

// SAFETY: libmpv запрещает одновременную работу с контекстом отрисовки
// из разных потоков, но не запрещает владеть им из другого потока.
// egui вызывает обратные вызовы отрисовки строго последовательно и только
// в потоке с активным контекстом OpenGL; другого доступа к контексту
// в приложении нет. Пометка нужна, чтобы контекст можно было передать
// в обратный вызов egui, который обязан быть `Send + Sync`.
unsafe impl Send for RenderContext {}
unsafe impl Sync for RenderContext {}

/// Контекст отрисовки для передачи в обратный вызов egui.
pub type SharedRenderContext = Arc<RenderContext>;

impl RenderContext {
    /// Создаёт контекст отрисовки для уже запущенного движка.
    ///
    /// # Безопасность времени жизни
    ///
    /// libmpv возвращает контекст, заимствующий `Mpv`. Движок хранит `Mpv`
    /// в куче, поэтому его адрес стабилен, а сам контекст живёт внутри
    /// `Engine` и уничтожается раньше `Mpv` за счёт порядка полей.
    /// Расширение времени жизни до `'static` здесь корректно.
    pub(crate) fn new(mpv: &Mpv, loader: ProcAddressLoader) -> Result<Self> {
        let mpv_static: &'static Mpv = unsafe { &*(mpv as *const Mpv) };

        let inner = mpv_static
            .create_render_context(vec![
                RenderParam::ApiType(RenderParamApiType::OpenGl),
                RenderParam::InitParams(OpenGLInitParams {
                    get_proc_address: resolve_proc_address,
                    ctx: loader,
                }),
            ])
            .map_err(|e| MpvError::Render(e.to_string()))?;

        tracing::info!("контекст отрисовки mpv создан");
        Ok(Self { inner })
    }

    /// Рисует текущий кадр в указанный framebuffer.
    ///
    /// `fbo` = 0 означает буфер окна. `flip` включён всегда: OpenGL считает
    /// начало координат снизу, видео — сверху.
    pub fn render(&self, fbo: i32, size: FrameSize) -> Result<()> {
        self.inner
            .render::<ProcAddressLoader>(fbo, size.width, size.height, true)
            .map_err(|e| MpvError::Render(e.to_string()))
    }

    /// Callback о готовности нового кадра.
    ///
    /// Вызывается из потока mpv. Внутри нельзя обращаться к API mpv —
    /// только разбудить интерфейс, чтобы он перерисовался.
    pub fn set_update_callback<F: Fn() + Send + 'static>(&mut self, callback: F) {
        self.inner.set_update_callback(callback);
    }

    /// Сообщает mpv, что кадр показан. Помогает точности тайминга.
    pub fn report_swap(&self) {
        self.inner.report_swap();
    }
}

impl Engine {
    /// Создаёт контекст отрисовки и сохраняет его внутри движка.
    ///
    /// Вызывается один раз при запуске, когда контекст OpenGL уже готов.
    /// `on_new_frame` вызывается из потока mpv при готовности кадра —
    /// внутри допустимо только разбудить интерфейс.
    pub fn init_render_context<F: Fn() + Send + 'static>(
        &mut self,
        loader: ProcAddressLoader,
        on_new_frame: F,
    ) -> Result<()> {
        let mut context = RenderContext::new(self.mpv_ref(), loader)?;
        context.set_update_callback(on_new_frame);
        self.set_render_context(Arc::new(context));
        Ok(())
    }

    /// Контекст для передачи в обратный вызов отрисовки.
    pub fn shared_render_context(&self) -> Option<SharedRenderContext> {
        self.render_context().cloned()
    }
}
