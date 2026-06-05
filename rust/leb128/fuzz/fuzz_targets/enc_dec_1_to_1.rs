#![no_main]

use leb128::{decode, decode_signed};
use libfuzzer_sys::fuzz_target;

macro_rules! gen_targets_unsigned {
    ($ty:ident, $ai:expr, $bi:expr) => {
        let a = decode::<$ty>($ai);
        let b = decode::<$ty>($bi);
        if let (Ok(a), Ok(b)) = (a, b)
            && a == b
            && $ai != $bi
        {
            panic!("found unsigned, {}", stringify!($ty));
        }
    };
}

macro_rules! gen_targets_signed {
    ($ty:ident, $ai:expr, $bi:expr) => {
        let a = decode_signed::<$ty>($ai);
        let b = decode_signed::<$ty>($bi);
        if let (Ok(a), Ok(b)) = (a, b)
            && a == b
            && $ai != $bi
        {
            panic!("found signed, {}", stringify!($ty));
        }
    };
}

fuzz_target!(|data: &[u8]| {
    let mid = data.len() / 2;

    let ai = &data[..mid];
    let Some(end) = ai.iter().position(|it| it & 0x80 == 0) else {
        return;
    };
    let ai = &ai[..=end];

    let bi = &data[mid..];
    let Some(end) = bi.iter().position(|it| it & 0x80 == 0) else {
        return;
    };
    let bi = &bi[..=end];

    gen_targets_unsigned!(u8, ai, bi);
    gen_targets_unsigned!(u16, ai, bi);
    gen_targets_unsigned!(u32, ai, bi);
    gen_targets_unsigned!(u64, ai, bi);
    gen_targets_unsigned!(u128, ai, bi);

    gen_targets_signed!(i8, ai, bi);
    gen_targets_signed!(i16, ai, bi);
    gen_targets_signed!(i32, ai, bi);
    gen_targets_signed!(i64, ai, bi);
    gen_targets_signed!(i128, ai, bi);
});
