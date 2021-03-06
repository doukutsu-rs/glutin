#![cfg(target_os = "android")]

use crate::api::egl::{Context as EglContext, NativeDisplay, SurfaceType as EglSurfaceType};
use crate::CreationError::{self, OsError};
use crate::{Api, ContextError, GlAttributes, PixelFormat, PixelFormatRequirements, Rect};

use crate::platform::android::EventLoopExtAndroid;
use glutin_egl_sys as ffi;
use parking_lot::Mutex;
use winit;
use winit::dpi;
use winit::event_loop::EventLoopWindowTarget;
use winit::window::WindowBuilder;

use std::sync::Arc;

#[derive(Debug)]
struct AndroidContext {
    egl_context: EglContext,
    stopped: Option<Mutex<bool>>,
}

#[derive(Debug)]
pub struct Context(Arc<AndroidContext>);

#[derive(Debug)]
struct AndroidSyncEventHandler(Arc<AndroidContext>);

impl Context {
    #[inline]
    pub fn new_windowed<T>(
        wb: WindowBuilder,
        el: &EventLoopWindowTarget<T>,
        pf_reqs: &PixelFormatRequirements,
        gl_attr: &GlAttributes<&Self>,
    ) -> Result<(winit::window::Window, Self), CreationError> {
        let win = wb.build(el)?;
        let gl_attr = gl_attr.clone().map_sharing(|c| &c.0.egl_context);
        let nwin = unsafe { ndk_glue::native_window() };
        if nwin.is_none() {
            return Err(OsError("Android's native window is null".to_string()));
        }
        let native_display = NativeDisplay::Android;
        let egl_context = EglContext::new(
            pf_reqs,
            &gl_attr,
            native_display,
            EglSurfaceType::Window,
            |c, _| Ok(c[0]),
        )
        .and_then(|p| p.finish(nwin.as_ref().unwrap().ptr().as_ptr() as *const _))?;
        let ctx = Arc::new(AndroidContext {
            egl_context,
            stopped: Some(Mutex::new(false)),
        });

        let context = Context(ctx.clone());

        Ok((win, context))
    }

    pub unsafe fn surface_destroyed(&self) {
        self.0.egl_context.on_surface_destroyed();
    }

    pub unsafe fn surface_created(&self, nwin: ffi::egl::EGLNativeWindowType) {
        self.0.egl_context.on_surface_created(nwin);
    }

    #[inline]
    pub fn new_headless<T>(
        _el: &EventLoopWindowTarget<T>,
        pf_reqs: &PixelFormatRequirements,
        gl_attr: &GlAttributes<&Context>,
        size: dpi::PhysicalSize<u32>,
    ) -> Result<Self, CreationError> {
        let gl_attr = gl_attr.clone().map_sharing(|c| &c.0.egl_context);
        let context = EglContext::new(
            pf_reqs,
            &gl_attr,
            NativeDisplay::Android,
            EglSurfaceType::PBuffer,
            |c, _| Ok(c[0]),
        )?;
        let egl_context = context.finish_pbuffer(size)?;
        let ctx = Arc::new(AndroidContext { egl_context, stopped: None });
        Ok(Context(ctx))
    }

    #[inline]
    pub unsafe fn make_current(&self) -> Result<(), ContextError> {
        if let Some(ref stopped) = self.0.stopped {
            let stopped = stopped.lock();
            if *stopped {
                return Err(ContextError::ContextLost);
            }
        }

        self.0.egl_context.make_current()
    }

    #[inline]
    pub unsafe fn make_not_current(&self) -> Result<(), ContextError> {
        if let Some(ref stopped) = self.0.stopped {
            let stopped = stopped.lock();
            if *stopped {
                return Err(ContextError::ContextLost);
            }
        }

        self.0.egl_context.make_not_current()
    }

    #[inline]
    pub fn resize(&self, _: u32, _: u32) {}

    #[inline]
    pub fn is_current(&self) -> bool {
        self.0.egl_context.is_current()
    }

    #[inline]
    pub fn get_proc_address(&self, addr: &str) -> *const core::ffi::c_void {
        self.0.egl_context.get_proc_address(addr)
    }

    #[inline]
    pub fn swap_buffers(&self) -> Result<(), ContextError> {
        if let Some(ref stopped) = self.0.stopped {
            let stopped = stopped.lock();
            if *stopped {
                return Err(ContextError::ContextLost);
            }
        }
        self.0.egl_context.swap_buffers()
    }

    #[inline]
    pub fn swap_buffers_with_damage(
        &self,
        rects: &[Rect],
    ) -> Result<(), ContextError> {
        Err(ContextError::OsError(
            "buffer damage not suported".to_string(),
        ))
    }

    #[inline]
    pub fn swap_buffers_with_damage_supported(&self) -> bool {
        //self.0.egl_context.swap_buffers_with_damage_supported()
        false
    }

    #[inline]
    pub fn get_api(&self) -> Api {
        self.0.egl_context.get_api()
    }

    #[inline]
    pub fn get_pixel_format(&self) -> PixelFormat {
        self.0.egl_context.get_pixel_format()
    }

    #[inline]
    pub unsafe fn raw_handle(&self) -> ffi::EGLContext {
        self.0.egl_context.raw_handle()
    }

    #[inline]
    pub unsafe fn get_egl_display(&self) -> ffi::EGLDisplay {
        self.0.egl_context.get_egl_display()
    }
}
