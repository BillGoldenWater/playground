use std::{
    ops::{Deref, DerefMut},
    ptr::NonNull,
};

use ffmpeg_sys_next::{
    AVPacket, av_packet_alloc, av_packet_free, av_packet_unref,
};

pub struct Packet {
    inner: NonNull<AVPacket>,
}

impl Packet {
    pub fn new() -> Self {
        let pkt = unsafe { av_packet_alloc() };
        let inner = NonNull::new(pkt).unwrap();
        Self { inner }
    }

    pub fn as_ptr(&mut self) -> *mut AVPacket {
        self.inner.as_ptr()
    }

    pub fn unref(&mut self) {
        unsafe {
            av_packet_unref(self.inner.as_ptr());
        }
    }

    pub fn temp(&mut self) -> UnrefGuard<'_> {
        UnrefGuard { pkt: self }
    }
}

impl Default for Packet {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for Packet {
    fn drop(&mut self) {
        unsafe {
            let mut inner = self.inner.as_ptr();
            av_packet_free(&mut inner);
        }
    }
}

impl Deref for Packet {
    type Target = AVPacket;

    fn deref(&self) -> &Self::Target {
        unsafe { self.inner.as_ref() }
    }
}

impl DerefMut for Packet {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { self.inner.as_mut() }
    }
}

pub struct UnrefGuard<'pkt> {
    pkt: &'pkt mut Packet,
}

impl<'pkt> UnrefGuard<'pkt> {
    pub fn as_ptr(&mut self) -> *mut AVPacket {
        self.pkt.as_ptr()
    }
}

impl<'pkt> Drop for UnrefGuard<'pkt> {
    fn drop(&mut self) {
        self.pkt.unref();
    }
}

impl<'pkt> Deref for UnrefGuard<'pkt> {
    type Target = AVPacket;

    fn deref(&self) -> &Self::Target {
        self.pkt.deref()
    }
}

impl<'pkt> DerefMut for UnrefGuard<'pkt> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.pkt.deref_mut()
    }
}
