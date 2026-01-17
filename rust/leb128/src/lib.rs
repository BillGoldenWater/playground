#[derive(Debug, PartialEq, Eq)]
pub enum Error {
    EndOfData,
    DataTooBig,
}

pub type Result<T> = core::result::Result<T, Error>;

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

pub fn decode<T: NumUnsigned>(data: &[u8]) -> Result<T> {
    let mut res = T::from_u8(0);
    let mut shift = 0;
    let mut data = data.iter().copied();
    let mut byte = data.next().ok_or(Error::EndOfData)?;

    loop {
        res.shifted_or_assign(byte & 0x7F, shift);
        shift += 7;

        if byte & 0x80 == 0 {
            break;
        }

        if shift >= 128 {
            return Err(Error::DataTooBig);
        }

        byte = data.next().ok_or(Error::EndOfData)?;
    }

    Ok(res)
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

pub fn decode_signed<T: NumSigned>(data: &[u8]) -> Result<T> {
    let mut res = T::UnsignedVariant::from_u8(0);
    let mut shift = 0;
    let mut data = data.iter().copied();
    let mut byte = data.next().ok_or(Error::EndOfData)?;

    loop {
        res.shifted_or_assign(byte & 0x7F, shift);
        shift += 7;

        if byte & 0x80 == 0 {
            break;
        }

        if shift >= 128 {
            return Err(Error::DataTooBig);
        }

        byte = data.next().ok_or(Error::EndOfData)?;
    }

    let mut res = T::from_unsigned(res);

    if shift < T::UnsignedVariant::BITS && byte & 0x40 != 0 {
        res.one_fill_left(shift);
    }

    Ok(res)
}

#[cfg(test)]
mod tests {
    use std::iter::{once, repeat_n};

    use rand::{Rng, rng};

    use super::*;

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
    }

    #[test]
    fn uleb128_err() {
        let mut cases: Vec<(Vec<u8>, Result<u128>)> =
            vec![(repeat_n(0x80, 19).collect(), Err(Error::DataTooBig))];
        cases.extend((0_usize..=18).map(|it| {
            (repeat_n(0x80, it).collect(), Err(Error::EndOfData))
        }));

        for (idx, (data, expect)) in cases.into_iter().enumerate() {
            let output = decode(&data);
            assert_eq!(
                output, expect,
                "case: {idx}, decode: {data:?}, expect: {expect:?}"
            );
        }
    }

    #[test]
    fn uleb128_fuzzy() {
        let mut output = Vec::<u8>::new();
        for idx in 0..=1_000_000 {
            let v: u128 = rng().random();

            output.clear();

            encode(v, &mut output);
            let output = decode(&output).unwrap();

            assert_eq!(v, output, "case: {idx}, encode/decode: {v:?}");
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
    }

    #[test]
    fn sleb128_err() {
        let mut cases: Vec<(Vec<u8>, Result<i128>)> =
            vec![(repeat_n(0x80, 19).collect(), Err(Error::DataTooBig))];
        cases.extend((0_usize..=18).map(|it| {
            (repeat_n(0x80, it).collect(), Err(Error::EndOfData))
        }));

        for (idx, (data, expect)) in cases.into_iter().enumerate() {
            let output = decode_signed(&data);
            assert_eq!(
                output, expect,
                "case: {idx}, decode: {data:?}, expect: {expect:?}"
            );
        }
    }

    #[test]
    fn sleb128_fuzzy() {
        let mut output = Vec::<u8>::new();
        for idx in 0..=1_000_000 {
            let v: i128 = rng().random();

            output.clear();

            encode_signed(v, &mut output);
            let output = decode_signed(&output).unwrap();

            assert_eq!(v, output, "case: {idx}, encode/decode: {v:?}");
        }
    }
}
