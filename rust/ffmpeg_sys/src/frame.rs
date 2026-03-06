use std::{
    ops::{Deref, DerefMut},
    ptr::NonNull,
};

use ffmpeg_sys_next::{AVFrame, av_frame_alloc, av_frame_free};

pub struct Frame {
    inner: NonNull<AVFrame>,
}

impl Frame {
    pub fn new() -> Self {
        let frame = unsafe { av_frame_alloc() };
        let inner = NonNull::new(frame).unwrap();
        Self { inner }
    }

    pub fn as_ptr(&mut self) -> *mut AVFrame {
        self.inner.as_ptr()
    }

    pub fn as_ptr_const(&self) -> *const AVFrame {
        self.inner.as_ptr()
    }
}

impl Drop for Frame {
    fn drop(&mut self) {
        unsafe {
            let mut inner = self.inner.as_ptr();
            av_frame_free(&mut inner);
        }
    }
}

impl Default for Frame {
    fn default() -> Self {
        Self::new()
    }
}

impl Deref for Frame {
    type Target = AVFrame;

    fn deref(&self) -> &Self::Target {
        unsafe { self.inner.as_ref() }
    }
}

impl DerefMut for Frame {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { self.inner.as_mut() }
    }
}
