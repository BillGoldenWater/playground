use std::{
    ffi::{CString, c_char},
    fmt::{self, Formatter},
    ops::Deref,
};

use ffmpeg_sys_next::av_strerror;

pub struct Error(i32);

impl Deref for Error {
    type Target = i32;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl fmt::Debug for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Error")
            .field(&self.0)
            .field(&format!("{self}"))
            .finish()
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut buf = vec![0_u8; 4096];
        let cstring = unsafe {
            av_strerror(
                self.0,
                buf.as_mut_ptr() as *mut c_char,
                buf.len(),
            );
            let idx = buf.iter().position(|it| *it == 0).unwrap();
            buf.truncate(idx + 1);
            CString::from_vec_with_nul(buf).unwrap()
        };
        f.write_str(&cstring.to_string_lossy())
    }
}

impl std::error::Error for Error {}

pub type Result<T> = std::result::Result<T, Error>;

pub trait AVResult {
    fn err_or<T>(&self, v: T) -> Result<T>;
    fn wrap_ret(&self) -> Result<i32>;
    fn wrap_unit(&self) -> Result<()> {
        self.err_or(())
    }
}

impl AVResult for i32 {
    fn err_or<T>(&self, v: T) -> Result<T> {
        if *self < 0 { Err(Error(*self)) } else { Ok(v) }
    }

    fn wrap_ret(&self) -> Result<i32> {
        self.err_or(*self)
    }
}
