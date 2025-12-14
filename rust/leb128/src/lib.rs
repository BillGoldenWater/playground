#[derive(Debug, PartialEq, Eq)]
pub enum Error {
    EndOfData,
    DataTooBig,
}

pub type Result<T> = core::result::Result<T, Error>;

pub fn encode(mut value: u128, output: &mut Vec<u8>) {
    loop {
        let byte = value as u8 & 0x7F;
        value >>= 7;

        if value == 0 {
            output.push(byte);
            break;
        } else {
            output.push(byte | 0x80);
        }
    }
}

pub fn decode(data: &[u8]) -> Result<u128> {
    let mut res = 0;
    let mut shift = 0;
    let mut data = data.iter().copied();
    let mut byte = data.next().ok_or(Error::EndOfData)?;

    loop {
        res |= ((byte & 0x7F) as u128) << shift;
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

pub fn encode_signed(mut value: i128, output: &mut Vec<u8>) {
    loop {
        let byte = value as u8 & 0x7F;
        value >>= 7;

        let sign = byte & 0x40;
        if (value == 0 && sign == 0) || (value == -1 && sign != 0) {
            output.push(byte);
            break;
        } else {
            output.push(byte | 0x80);
        }
    }
}

pub fn decode_signed(data: &[u8]) -> Result<i128> {
    let mut res = 0;
    let mut shift = 0;
    let mut data = data.iter().copied();
    let mut byte = data.next().ok_or(Error::EndOfData)?;

    loop {
        res |= ((byte & 0x7F) as u128) << shift;
        shift += 7;

        if byte & 0x80 == 0 {
            break;
        }

        if shift >= 128 {
            return Err(Error::DataTooBig);
        }

        byte = data.next().ok_or(Error::EndOfData)?;
    }

    if shift < u128::BITS && byte & 0x40 != 0 {
        res |= u128::MAX.wrapping_shl(shift);
    }

    Ok(res as i128)
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
            let v = rng().random();

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
            let v = rng().random();

            output.clear();

            encode_signed(v, &mut output);
            let output = decode_signed(&output).unwrap();

            assert_eq!(v, output, "case: {idx}, encode/decode: {v:?}");
        }
    }
}
