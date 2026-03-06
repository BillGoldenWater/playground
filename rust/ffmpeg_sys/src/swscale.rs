use std::ptr::NonNull;

use ffmpeg_sys_next::{
    SwsContext, sws_alloc_context, sws_freeContext, sws_scale_frame,
};

use crate::{error::AVResult, frame::Frame};

pub struct Context {
    inner: NonNull<SwsContext>,
}

impl Context {
    pub fn new() -> Self {
        let ctx = unsafe { sws_alloc_context() };
        let inner = NonNull::new(ctx).unwrap();
        Self { inner }
    }

    pub fn scale_frame(
        &mut self,
        dst: &mut Frame,
        src: &Frame,
    ) -> crate::Result<()> {
        unsafe {
            sws_scale_frame(
                self.inner.as_ptr(),
                dst.as_ptr(),
                src.as_ptr_const(),
            )
        }
        .wrap_unit()
    }
}

impl Drop for Context {
    fn drop(&mut self) {
        unsafe {
            sws_freeContext(self.inner.as_ptr());
        }
    }
}

impl Default for Context {
    fn default() -> Self {
        Self::new()
    }
}
