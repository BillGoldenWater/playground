use std::ffi::CStr;

use ffmpeg_sys_next::av_strerror;

#[deprecated]
#[track_caller]
pub fn check_ret(ret: i32) {
    if ret >= 0 {
        return;
    }

    unsafe {
        let buf = &mut [0_i8; 256];
        let ret = av_strerror(ret, buf as _, buf.len());
        if ret < 0 {
            panic!("unable to find description for error code: {ret}");
        }
        panic!("{ret}: {:?}", CStr::from_ptr(buf as _));
    }
}
