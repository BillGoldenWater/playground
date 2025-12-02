#![feature(portable_simd, slice_split_once)]

use std::{
    collections::HashMap,
    fs::File,
    io::{Read as _, Seek, SeekFrom},
    num::NonZero,
    simd::{Simd, cmp::SimdPartialEq},
};

// const NAME_CMP_LANES: usize = 16;
// const NAME_MAX_LEN: usize =
//     (100_usize.div_ceil(NAME_CMP_LANES)) * NAME_CMP_LANES;
const NAME_MAX_LEN: usize = 100;
type NameLenTy = u8;
const ACCUMULATOR_MASK: usize = (1 << 17) - 1;
const ACCUMULATOR_CAP: usize = ACCUMULATOR_MASK + 1;

const READ_BUF_LEN: usize = 8 * 1024 * 1024;
const SCAN_LANES: usize = 64;

fn main() {
    let mut file = File::options()
        .read(true)
        .open("./1brc/measurements.txt")
        .unwrap();
    let size = file.metadata().unwrap().len();

    let section_num = std::thread::available_parallelism()
        .map(NonZero::get)
        .unwrap_or(1) as u64;
    let section_size = (size / section_num).max(1);

    let mut threads = vec![];

    let mut buf = vec![0_u8; NAME_MAX_LEN * 2];
    let mut offset = 0;
    for _ in 0..section_num {
        if offset == size {
            break;
        }
        let mut end = offset + section_size;
        if end > size {
            end = size;
        }

        file.seek(SeekFrom::Start(end - 1)).unwrap();
        let n = file.read(&mut buf).unwrap();
        assert_ne!(n, 0);
        let idx = buf.iter().position(|it| *it == b'\n').unwrap();
        end += idx as u64;

        let handle = std::thread::spawn(move || {
            process_section(offset, end - offset)
        });
        threads.push(handle);

        offset = end;
    }

    let mut out: Out = HashMap::with_capacity(ACCUMULATOR_CAP);

    for ele in threads {
        let accumulator = ele.join().unwrap();
        accumulator.dump_to_hashmap(&mut out);
    }

    let mut out = out.into_iter().collect::<Vec<_>>();
    out.sort_unstable_by(|a, b| a.0.cmp(&b.0));

    let mut out_iter = out.into_iter().peekable();

    fn round(n: f64) -> f64 {
        (n * 10.).round() * 0.1
    }

    print!("{{");
    while let Some((name, value)) = out_iter.next() {
        let name = str::from_utf8(&name).unwrap();
        print!(
            "{name}={:.1}/{:.1}/{:.1}",
            round(value.min as f64 * 0.1),
            round(value.sum as f64 * 0.1 / value.count as f64),
            round(value.max as f64 * 0.1)
        );
        if out_iter.peek().is_some() {
            print!(", ");
        }
    }
    println!("}}");
}

fn process_section(start: u64, section_len: u64) -> Accumulator {
    const NEW_LINE: Simd<u8, 64> = Simd::<u8, SCAN_LANES>::splat(b'\n');

    let mut file = File::options()
        .read(true)
        .open("./1brc/measurements.txt")
        .unwrap();
    file.seek(SeekFrom::Start(start)).unwrap();

    let mut buf = vec![0_u8; READ_BUF_LEN];
    let mut read_base = 0;
    let mut remaining = section_len;

    let mut accumulator: Accumulator = Accumulator::new(ACCUMULATOR_CAP);

    loop {
        let n = if remaining == 0 {
            0
        } else {
            let start = read_base;
            let end = (start + remaining.min(usize::MAX as u64) as usize)
                .min(buf.len());
            let n = file.read(&mut buf[start..end]).unwrap();
            (remaining as usize).min(n)
        };
        remaining -= n as u64;

        if n != 0 {
            #[cfg(debug_assertions)]
            let start = std::time::Instant::now();

            let len = read_base + n;

            let mut last = 0;

            let (chunks, _) = buf[..len].as_chunks::<SCAN_LANES>();
            for (idx, chunk) in chunks.iter().enumerate() {
                let simd_offset = idx * SCAN_LANES;
                let chunk =
                    Simd::<u8, SCAN_LANES>::from_slice(&chunk[..]);
                let mask = chunk.simd_eq(NEW_LINE);

                let mut bits = mask.to_bitmask();
                while bits != 0 {
                    let idx =
                        simd_offset + bits.trailing_zeros() as usize;
                    process(&mut accumulator, &buf[last..idx]);
                    last = idx + 1;

                    bits &= bits - 1;
                }
            }

            buf.copy_within(last.., 0);
            read_base = len - last;

            #[cfg(debug_assertions)]
            println!("{:?}", start.elapsed());
        } else {
            if read_base == 0 {
                break;
            }
            let data = str::from_utf8(&buf[..read_base]).unwrap();
            assert!(data.ends_with('\n'), "{data:?}");
            data.lines().for_each(|line| {
                process(&mut accumulator, line.as_bytes());
            });
            break;
        }
    }

    accumulator
}

#[inline(always)]
fn process(acc: &mut Accumulator, it: &[u8]) {
    let (l, r) = {
        let end = it.len() - 4;
        let idx = if it[end] == b';' {
            end
        } else if it[end - 1] == b';' {
            end - 1
        } else {
            end - 2
        };
        (&it[..idx], &it[idx + 1..])
    };

    let temp = {
        let neg = r[0] == b'-';
        let start = neg as usize;

        let p_idx = r.len() - 2;

        let mut temp = (r[start] - b'0') as i16;
        if start + 1 != p_idx {
            temp = temp * 10 + (r[start + 1] - b'0') as i16;
        }
        // .
        temp = temp * 10 + (r[p_idx + 1] - b'0') as i16;

        if neg { -temp } else { temp }
    };

    acc.record(l, temp);
}

type Out = HashMap<Box<[u8]>, Value>;

/// https://en.wikipedia.org/wiki/Fowler%E2%80%93Noll%E2%80%93Vo_hash_function
#[inline(always)]
fn hash(data: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf29ce484222325;

    for &b in data.iter().take(4) {
        hash ^= b as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }

    // ============================================================

    // let (chunks, remainder) = data.as_chunks::<8>();
    //
    // for chunk in chunks.iter().take(1) {
    //     hash ^= u64::from_le_bytes(*chunk);
    //     hash = hash.wrapping_mul(0x100000001b3);
    // }

    // let (chunks, remainder) = remainder.as_chunks::<4>();
    //
    // for chunk in chunks {
    //     hash ^= u32::from_le_bytes(*chunk) as u64;
    //     hash = hash.wrapping_mul(0x100000001b3);
    // }

    // for &b in remainder {
    //     hash ^= b as u64;
    //     hash = hash.wrapping_mul(0x100000001b3);
    // }

    hash
}

#[derive(Debug, Clone, Copy)]
struct Key {
    len: NameLenTy,
    data: [u8; NAME_MAX_LEN],
}

impl Key {
    #[inline(always)]
    pub fn eq_name(&self, name: &[u8]) -> bool {
        if name.len() != self.len as usize {
            return false;
        }

        &self.data[..self.len as usize] == name

        // let (name_chunks, name_remainder) =
        //     name.as_chunks::<NAME_CMP_LANES>();
        // let (chunks, remainder) = self.data.as_chunks::<NAME_CMP_LANES>();
        // debug_assert!(remainder.is_empty());
        //
        // for (a, b) in chunks.iter().zip(name_chunks) {
        //     let a = Simd::<u8, NAME_CMP_LANES>::from_slice(a.as_slice());
        //     let b = Simd::<u8, NAME_CMP_LANES>::from_slice(b.as_slice());
        //     if !a.simd_eq(b).all() {
        //         return false;
        //     }
        // }
        //
        // let base = name_chunks.len() * NAME_CMP_LANES;
        // let a = &self.data[base..base + name_remainder.len()];
        // let b = name_remainder;
        // debug_assert_eq!(a.len(), b.len());
        //
        // for (a, b) in a.iter().zip(b) {
        //     if *a != *b {
        //         return false;
        //     }
        // }
        //
        // true
    }

    #[inline(always)]
    pub fn as_bytes(&self) -> &[u8] {
        &self.data[..self.len as usize]
    }
}

impl From<&[u8]> for Key {
    #[inline(always)]
    fn from(value: &[u8]) -> Self {
        debug_assert!(value.len() <= NAME_MAX_LEN);
        let mut data = [0; NAME_MAX_LEN];
        data[..value.len()].copy_from_slice(value);
        Self {
            len: value.len() as NameLenTy,
            data,
        }
    }
}

impl Default for Key {
    fn default() -> Self {
        Self {
            len: 0,
            data: [0; _],
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct Value {
    min: i16,
    max: i16,
    sum: i64,
    count: u64,
}

impl Value {
    pub fn merge(&mut self, other: &Self) {
        self.min = self.min.min(other.min);
        self.max = self.max.max(other.max);
        self.sum += other.sum;
        self.count += other.count;
    }
}

impl Default for Value {
    fn default() -> Self {
        Self {
            min: i16::MAX,
            max: i16::MIN,
            sum: 0,
            count: 0,
        }
    }
}

#[derive(Debug, Default, Clone, Copy)]
struct Slot {
    key: Key,
    value: Value,
}

struct Accumulator {
    capacity: usize,
    size: usize,
    slots: Box<[Slot]>,
}

impl Accumulator {
    pub fn new(capacity: usize) -> Self {
        let slots = vec![Default::default(); capacity].into_boxed_slice();
        Self {
            capacity,
            size: 0,
            slots,
        }
    }

    #[inline(always)]
    pub fn record(&mut self, name: &[u8], temp: i16) {
        assert!(self.size < self.capacity);

        let idx = hash(name);
        let mut idx = idx as usize & ACCUMULATOR_MASK;

        #[cfg(debug_assertions)]
        let old_idx = idx;

        let slot = loop {
            let slot = &mut self.slots[idx];
            if slot.key.len == 0 {
                break slot;
            }
            if slot.key.eq_name(name) {
                break slot;
            }

            idx += 1;
            if idx == ACCUMULATOR_CAP {
                idx = 0;
            }
            // dbg!(self.slots[idx], str::from_utf8(name).unwrap(), temp);
        };

        #[cfg(debug_assertions)]
        if old_idx.abs_diff(idx) > 1 {
            dbg!(old_idx.abs_diff(idx));
        }

        if slot.key.len == 0 {
            self.size += 1;

            slot.key = name.into();

            slot.value.min = temp;
            slot.value.max = temp;
            slot.value.sum = temp as i64;
            slot.value.count = 1;
        } else {
            let v = &mut slot.value;
            v.min = v.min.min(temp);
            v.max = v.max.max(temp);
            v.sum += temp as i64;
            v.count += 1;
        }
    }

    pub fn dump_to_hashmap(&self, out: &mut Out) {
        self.slots
            .iter()
            .filter(|it| it.key.len != 0)
            .for_each(|slot| {
                out.entry(Box::from(slot.key.as_bytes()))
                    .or_default()
                    .merge(&slot.value);
            });
    }
}
