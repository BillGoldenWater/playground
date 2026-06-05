#[derive(Debug, PartialEq, Eq)]
pub enum Error<T: NumUnsigned> {
    EndOfData,
    DataTooBig { cur: T, shift: u32, byte: u8 },
    TrailingEmptyBytes,
}

pub type Result<T, UT> = core::result::Result<T, Error<UT>>;

pub trait NumUnsigned {
    const BITS: u32;

    fn from_u8(value: u8) -> Self;

    fn trunc_u8(&self) -> u8;
    fn all_zero(&self) -> bool;
    fn all_one(&self) -> bool;
    fn shr_assign(&mut self, rhs: u32);
    fn sar_assign(&mut self, rhs: u32);
    fn shifted_or_assign(&mut self, rhs: u8, shift: u32);
}

pub trait NumSigned {
    type UnsignedVariant: NumUnsigned;

    fn as_unsigned(&self) -> Self::UnsignedVariant;
    fn from_unsigned(value: Self::UnsignedVariant) -> Self;
    fn one_fill_left(&mut self, right: u32);
}

macro_rules! impl_num {
    ($ty:ty, $signed_ty:ty) => {
        impl NumUnsigned for $ty {
            const BITS: u32 = <$ty>::BITS;

            #[inline]
            fn from_u8(value: u8) -> $ty {
                value as $ty
            }

            #[inline]
            fn trunc_u8(&self) -> u8 {
                *self as u8
            }

            #[inline]
            fn all_zero(&self) -> bool {
                *self == 0
            }

            #[inline]
            fn all_one(&self) -> bool {
                *self == <$ty>::MAX
            }

            #[inline]
            fn shr_assign(&mut self, rhs: u32) {
                *self >>= rhs;
            }

            #[inline]
            fn sar_assign(&mut self, rhs: u32) {
                *self = ((*self as $signed_ty) >> rhs) as $ty;
            }

            #[inline]
            fn shifted_or_assign(&mut self, rhs: u8, shift: u32) {
                *self |= (rhs as $ty) << shift;
            }
        }

        impl NumSigned for $signed_ty {
            type UnsignedVariant = $ty;

            #[inline]
            fn as_unsigned(&self) -> Self::UnsignedVariant {
                *self as Self::UnsignedVariant
            }

            #[inline]
            fn from_unsigned(value: Self::UnsignedVariant) -> Self {
                value as $signed_ty
            }

            #[inline]
            fn one_fill_left(&mut self, right: u32) {
                *self = (*self as $ty | <$ty>::MAX.wrapping_shl(right))
                    as $signed_ty;
            }
        }
    };
}

impl_num!(u128, i128);
impl_num!(u64, i64);
impl_num!(u32, i32);
impl_num!(u16, i16);
impl_num!(u8, i8);

pub fn encode(mut value: impl NumUnsigned, output: &mut Vec<u8>) {
    loop {
        let byte = value.trunc_u8() & 0x7F;
        value.shr_assign(7);

        if value.all_zero() {
            output.push(byte);
            break;
        } else {
            output.push(byte | 0x80);
        }
    }
}

pub fn decode<T: NumUnsigned>(data: &[u8]) -> Result<T, T> {
    decode_resume(data.iter().copied(), T::from_u8(0), 0, 0)
}

pub fn decode_resume<T, I>(
    mut data: I,
    mut cur: T,
    mut shift: u32,
    mut byte: u8,
) -> Result<T, T>
where
    T: NumUnsigned,
    I: Iterator<Item = u8>,
{
    let mut first = byte == 0;
    if first {
        byte = data.next().ok_or(Error::EndOfData)?;
    }

    loop {
        cur.shifted_or_assign(byte & 0x7F, shift);

        if byte & 0x80 == 0 {
            if byte == 0 && !first {
                return Err(Error::TrailingEmptyBytes);
            }

            break;
        }

        shift += 7;
        if shift >= T::BITS {
            return Err(Error::DataTooBig {
                cur,
                shift: shift - 7,
                byte,
            });
        }

        byte = data.next().ok_or(Error::EndOfData)?;
        first = false;
    }

    if shift > T::BITS - 7 {
        // extra bits mask
        let mask = !((1 << (T::BITS - shift)) - 1);
        if mask & byte != 0 {
            return Err(Error::DataTooBig { cur, shift, byte });
        }
    }

    Ok(cur)
}

pub fn encode_signed(value: impl NumSigned, output: &mut Vec<u8>) {
    let mut value = value.as_unsigned();
    loop {
        let byte = value.trunc_u8() & 0x7F;
        value.sar_assign(7);

        let sign = byte & 0x40;
        if (value.all_zero() && sign == 0)
            || (value.all_one() && sign != 0)
        {
            output.push(byte);
            break;
        } else {
            output.push(byte | 0x80);
        }
    }
}

pub fn decode_signed<T: NumSigned>(
    data: &[u8],
) -> Result<T, T::UnsignedVariant> {
    decode_signed_resume(
        data.iter().copied(),
        T::UnsignedVariant::from_u8(0),
        0,
        0,
    )
}

pub fn decode_signed_resume<T, I>(
    mut data: I,
    mut cur: T::UnsignedVariant,
    mut shift: u32,
    mut byte: u8,
) -> Result<T, T::UnsignedVariant>
where
    T: NumSigned,
    I: Iterator<Item = u8>,
{
    let bits = T::UnsignedVariant::BITS;
    let mut last_byte = 0;
    let mut first = byte == 0;
    if first {
        byte = data.next().ok_or(Error::EndOfData)?;
    }

    loop {
        cur.shifted_or_assign(byte & 0x7F, shift);

        if byte & 0x80 == 0 {
            if !first {
                let pos = byte == 0 && last_byte & 0x40 == 0;
                let neg = byte == 0x7F && last_byte & 0x40 != 0;
                if pos || neg {
                    return Err(Error::TrailingEmptyBytes);
                }
            }
            break;
        }

        shift += 7;
        if shift >= bits {
            return Err(Error::DataTooBig {
                cur,
                shift: shift - 7,
                byte,
            });
        }

        last_byte = byte;
        byte = data.next().ok_or(Error::EndOfData)?;
        first = false;
    }

    if shift > bits - 7 {
        // extra bits mask
        let mask = !((1 << (bits - shift - 1)) - 1);
        if byte & 0x40 != 0 {
            if shift > bits - 7 && !(mask & byte | 0x80 | !mask) != 0 {
                return Err(Error::DataTooBig { cur, shift, byte });
            }
        } else {
            if shift > bits - 7 && mask & byte != 0 {
                return Err(Error::DataTooBig { cur, shift, byte });
            }
        }
    }

    let mut res = T::from_unsigned(cur);
    if shift < bits - 7 && byte & 0x40 != 0 {
        res.one_fill_left(shift + 7);
    }

    Ok(res)
}

#[cfg(test)]
mod tests {
    use std::{
        assert_matches,
        fmt::Debug,
        iter::{once, repeat_n},
    };

    use rand::{Rng, rng};

    use super::*;

    fn test_uleb128_resume<TTry, TResume>(
        data: &[u8],
    ) -> Result<TResume, TResume>
    where
        TTry: NumUnsigned + Debug,
        TResume: NumUnsigned + From<TTry>,
    {
        let mut data = data.iter().copied();
        let res =
            decode_resume::<TTry, _>(&mut data, TTry::from_u8(0), 0, 0);
        match res {
            Err(Error::DataTooBig { cur, shift, byte }) => {
                decode_resume::<TResume, _>(data, cur.into(), shift, byte)
            }
            Ok(it) => Ok(it.into()),
            it => unreachable!("{it:?}"),
        }
    }

    #[test]
    fn uleb128() {
        let mut cases: Vec<(u128, Vec<u8>)> = vec![
            (0, vec![0]),
            (1, vec![1]),
            (u128::MAX, repeat_n(0xFF, 18).chain(once(3)).collect()),
        ];
        cases.extend((0_usize..=15).map(|it| {
            (
                0x80 << (8 * it),
                repeat_n(0x80, it + 1 + it / 7)
                    .chain(once(1 << (it % 7)))
                    .collect(),
            )
        }));
        cases.extend((0_usize..=15).map(|it| {
            (
                (0x7F << (8 * it)) | ((1 << (8 * it)) - 1),
                repeat_n(0xFF, it + it.div_ceil(7))
                    .chain(once((1 << (((it + 6) % 7) + 1)) - 1))
                    .collect(),
            )
        }));

        let mut output = Vec::<u8>::new();
        for (idx, (v, encoded)) in cases.into_iter().enumerate() {
            output.clear();

            encode(v, &mut output);
            assert_eq!(
                output, encoded,
                "case: {idx}, encode: {v}, expect: {encoded:?}"
            );

            let output = decode(&encoded);
            assert_eq!(
                output,
                Ok(v),
                "case: {idx}, decode: {encoded:?}, expect: {v}"
            );
        }

        let mut data = Vec::<u8>::new();
        for v in [1 << 8, 1 << 15] {
            data.clear();
            encode(v, &mut data);

            let res = test_uleb128_resume::<u8, u16>(&data);
            assert_eq!(res, Ok(v));
            let res = test_uleb128_resume::<u8, u8>(&data);
            assert_eq!(res, decode::<u8>(&data));
        }
    }

    macro_rules! test_too_big {
        ($buf:ident, $input:expr, $out:ty) => {
            encode($input, &mut $buf);
            let res = decode::<$out>(&$buf);
            assert_matches!(res, Err(Error::DataTooBig { .. }));
            $buf.clear();
        };
    }

    #[test]
    fn uleb128_err() {
        let cases: Vec<(Vec<_>, Result<u128, _>)> = (0_usize..=18)
            .map(|it| {
                (repeat_n(0x80, it).collect(), Err(Error::EndOfData))
            })
            .collect();

        for (idx, (data, expect)) in cases.into_iter().enumerate() {
            let output = decode(&data);
            assert_eq!(
                output, expect,
                "case: {idx}, decode: {data:?}, expect: {expect:?}"
            );
        }

        let res = decode::<u128>(&repeat_n(0x80, 19).collect::<Vec<_>>());
        assert_matches!(
            res,
            Err(Error::DataTooBig {
                cur: 0,
                shift: 126,
                byte: 0x80
            })
        );

        let res = decode::<u128>(&[0x80, 0]);
        assert_matches!(res, Err(Error::TrailingEmptyBytes));
        let res = decode_resume::<u128, _>(once(0), 0, 7, 0x80);
        assert_matches!(res, Err(Error::TrailingEmptyBytes));

        let mut data = vec![];
        test_too_big!(data, u128::MAX, u64);
        test_too_big!(data, u64::MAX, u32);
        test_too_big!(data, u32::MAX, u16);
        test_too_big!(data, u16::MAX, u8);
        test_too_big!(data, u64::MAX as u128 + 1, u64);
        test_too_big!(data, u32::MAX as u64 + 1, u32);
        test_too_big!(data, u16::MAX as u32 + 1, u16);
        test_too_big!(data, u8::MAX as u16 + 1, u8);
    }

    #[test]
    fn uleb128_fuzzy() {
        let mut data = Vec::<u8>::new();
        for idx in 0..=1_000_000 {
            let v: u128 = rng().random();

            data.clear();

            encode(v, &mut data);
            let output = decode(&data).unwrap();

            assert_eq!(v, output, "case: {idx}, encode/decode: {v:?}");

            let mut data = data.iter().copied();
            let res = decode_resume::<u64, _>(&mut data, 0, 0, 0);
            let output = match res {
                Err(Error::DataTooBig { cur, shift, byte }) => {
                    decode_resume::<u128, _>(
                        data,
                        cur as u128,
                        shift,
                        byte,
                    )
                    .unwrap()
                }
                Ok(it) => it as u128,
                it => unreachable!("{it:?}"),
            };
            assert_eq!(v, output, "case: {idx}, encode/decode: {v:?}");
        }
    }

    fn test_sleb128_resume<TTry, TResume>(
        data: &[u8],
    ) -> Result<TResume, TResume::UnsignedVariant>
    where
        TTry: NumSigned + Debug,
        TTry::UnsignedVariant: Debug,
        TResume: NumSigned + From<TTry>,
        TResume::UnsignedVariant: From<TTry::UnsignedVariant>,
    {
        let mut data = data.iter().copied();
        let res = decode_signed_resume::<TTry, _>(
            &mut data,
            TTry::UnsignedVariant::from_u8(0),
            0,
            0,
        );
        match res {
            Err(Error::DataTooBig { cur, shift, byte }) => {
                decode_signed_resume::<TResume, _>(
                    data,
                    cur.into(),
                    shift,
                    byte,
                )
            }
            Ok(it) => Ok(it.into()),
            it => unreachable!("{it:?}"),
        }
    }

    #[test]
    fn sleb128() {
        let mut cases: Vec<(i128, Vec<u8>)> = vec![
            (0, vec![0]),
            (1, vec![1]),
            (-1, vec![0x7F]),
            (i128::MAX, repeat_n(0xFF, 18).chain(once(1)).collect()),
            (i128::MIN, repeat_n(0x80, 18).chain(once(0x7E)).collect()),
        ];
        cases.extend((0_usize..=14).map(|it| {
            (
                0x80 << (8 * it),
                repeat_n(0x80, it + 1 + it / 7)
                    .chain(once(
                        (1 << (it % 7))
                            | if it % 7 == 6 { 0x80 } else { 0 },
                    ))
                    .chain(once(0).take_while(|_| (it % 7) == 6))
                    .collect(),
            )
        }));
        cases.extend((0_usize..=15).map(|it| {
            let it_off = it + 6;
            (
                (0x7F << (8 * it)) | ((1 << (8 * it)) - 1),
                repeat_n(0xFF, it + it_off / 7)
                    .chain(once(
                        ((1 << ((it_off % 7) + 1)) - 1)
                            | if it_off % 7 == 6 { 0x80 } else { 0 },
                    ))
                    .chain(once(0).take_while(|_| (it_off % 7) == 6))
                    .collect(),
            )
        }));
        // TODO: ?, negative

        let mut output = Vec::<u8>::new();
        for (idx, (v, encoded)) in cases.into_iter().enumerate() {
            output.clear();

            encode_signed(v, &mut output);
            assert_eq!(
                output, encoded,
                "case: {idx}, encode: {v}, expect: {encoded:?}"
            );

            let output = decode_signed(&encoded);
            assert_eq!(
                output,
                Ok(v),
                "case: {idx}, decode: {encoded:?}, expect: {v}"
            );
        }

        let mut data = Vec::<u8>::new();
        for v in [1 << 8, 1 << 15] {
            data.clear();
            encode_signed(v, &mut data);

            let res = test_sleb128_resume::<i8, i16>(&data);
            assert_eq!(res, Ok(v));
            let res = test_sleb128_resume::<i8, i8>(&data);
            assert_eq!(res, decode_signed::<i8>(&data));
        }
    }

    macro_rules! test_too_big_signed {
        ($buf:ident, $input:expr, $out:ty) => {
            encode_signed($input, &mut $buf);
            let res = decode_signed::<$out>(&$buf);
            assert_matches!(res, Err(Error::DataTooBig { .. }));
            $buf.clear();
        };
    }

    #[test]
    fn sleb128_err() {
        let cases: Vec<(Vec<_>, Result<i128, _>)> = (0_usize..=18)
            .map(|it| {
                (repeat_n(0x80, it).collect(), Err(Error::EndOfData))
            })
            .collect();

        for (idx, (data, expect)) in cases.into_iter().enumerate() {
            let output = decode_signed(&data);
            assert_eq!(
                output, expect,
                "case: {idx}, decode: {data:?}, expect: {expect:?}"
            );
        }

        let res = decode_signed::<i128>(
            &repeat_n(0x80, 19).collect::<Vec<_>>(),
        );
        assert_matches!(
            res,
            Err(Error::DataTooBig {
                cur: 0,
                shift: 126,
                byte: 0x80
            })
        );

        let res = decode_signed::<i128>(&[0x80, 0]);
        assert_matches!(res, Err(Error::TrailingEmptyBytes));
        let res = decode_signed::<i128>(&[0xFF, 0x7F]);
        assert_matches!(res, Err(Error::TrailingEmptyBytes));

        let mut data = vec![];
        test_too_big_signed!(data, i128::MAX, i64);
        test_too_big_signed!(data, i64::MAX, i32);
        test_too_big_signed!(data, i32::MAX, i16);
        test_too_big_signed!(data, i16::MAX, i8);

        test_too_big_signed!(data, i128::MIN, i64);
        test_too_big_signed!(data, i64::MIN, i32);
        test_too_big_signed!(data, i32::MIN, i16);
        test_too_big_signed!(data, i16::MIN, i8);

        test_too_big_signed!(data, i64::MAX as i128 + 1, i64);
        test_too_big_signed!(data, i32::MAX as i64 + 1, i32);
        test_too_big_signed!(data, i16::MAX as i32 + 1, i16);
        test_too_big_signed!(data, i8::MAX as i16 + 1, i8);

        test_too_big_signed!(data, i64::MIN as i128 - 1, i64);
        test_too_big_signed!(data, i32::MIN as i64 - 1, i32);
        test_too_big_signed!(data, i16::MIN as i32 - 1, i16);
        test_too_big_signed!(data, i8::MIN as i16 - 1, i8);
    }

    #[test]
    fn sleb128_fuzzy() {
        let mut data = Vec::<u8>::new();
        for idx in 0..=1_000_000 {
            let v: i128 = rng().random();

            data.clear();

            encode_signed(v, &mut data);
            let output = decode_signed(&data).unwrap();

            assert_eq!(v, output, "case: {idx}, encode/decode: {v:?}");

            let mut data = data.iter().copied();
            let res = decode_signed_resume::<i64, _>(&mut data, 0, 0, 0);
            let output = match res {
                Err(Error::DataTooBig { cur, shift, byte }) => {
                    decode_signed_resume::<i128, _>(
                        data,
                        cur as u128,
                        shift,
                        byte,
                    )
                    .unwrap()
                }
                Ok(it) => it as i128,
                it => unreachable!("{it:?}"),
            };
            assert_eq!(v, output, "case: {idx}, encode/decode: {v:?}");
        }
    }
}
