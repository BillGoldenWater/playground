use std::{
    any::TypeId,
    io::{BufWriter, Write},
    ops::Deref,
    path::Path,
};

use bytemuck::{Pod, cast_slice, cast_slice_mut};
use strum::IntoEnumIterator;
use yansi::{Paint as _, Style};

use crate::{Fp, min_max_tracker::MinMaxTracker};

#[derive(Debug, Clone, Copy, PartialEq, Eq, strum::EnumIter)]
#[repr(u8)]
pub enum DataType {
    U8 = 0x08,
    I8 = 0x09,
    I16 = 0x0B,
    I32 = 0x0C,
    F32 = 0x0D,
    F64 = 0x0E,
}

impl DataType {
    pub fn size(&self) -> usize {
        match self {
            DataType::U8 => 1,
            DataType::I8 => 1,
            DataType::I16 => 2,
            DataType::I32 => 4,
            DataType::F32 => 4,
            DataType::F64 => 8,
        }
    }
}

impl TryFrom<u8> for DataType {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        Self::iter().find(|&it| it as u8 == value).ok_or(())
    }
}

impl TryFrom<TypeId> for DataType {
    type Error = ();

    fn try_from(value: TypeId) -> Result<Self, Self::Error> {
        Self::iter().find(|&it| value == it.into()).ok_or(())
    }
}

impl From<DataType> for TypeId {
    fn from(value: DataType) -> Self {
        match value {
            DataType::U8 => TypeId::of::<u8>(),
            DataType::I8 => TypeId::of::<i8>(),
            DataType::I16 => TypeId::of::<i16>(),
            DataType::I32 => TypeId::of::<i32>(),
            DataType::F32 => TypeId::of::<f32>(),
            DataType::F64 => TypeId::of::<f64>(),
        }
    }
}

#[derive(Debug)]
pub struct IdxData<T> {
    dimensions: Box<[usize]>,
    data: Vec<T>,
}

impl<T> IdxData<T> {
    pub fn new(dimensions: Box<[usize]>, data: Vec<T>) -> Self {
        assert!(!dimensions.is_empty());
        assert_eq!(data.len(), dimensions.iter().product::<usize>());
        Self { dimensions, data }
    }

    pub fn load(path: impl AsRef<Path>) -> Self
    where
        T: Pod,
    {
        let mut raw_data =
            std::fs::read(path).unwrap().into_boxed_slice();
        assert_eq!(&raw_data[..2], &[0, 0]);
        let data_type = DataType::try_from(raw_data[2]).unwrap();
        assert_eq!(TypeId::of::<T>(), data_type.into());
        let dimension_count = raw_data[3] as usize;
        assert!(dimension_count > 0);
        let mut dimensions =
            vec![0_usize; dimension_count].into_boxed_slice();

        let mut data_offset: usize = 4;
        for size in dimensions.iter_mut() {
            *size = u32::from_be_bytes(
                raw_data[data_offset..data_offset + 4]
                    .try_into()
                    .unwrap(),
            ) as usize;
            data_offset += 4;
        }

        let len = dimensions.iter().product::<usize>();
        let raw_data = &mut raw_data[data_offset..];

        assert!(raw_data.len() == len * data_type.size());

        if cfg!(target_endian = "little") && data_type.size() > 1 {
            raw_data
                .chunks_exact_mut(data_type.size())
                .for_each(|it| it.reverse());
        }

        let mut data = vec![T::zeroed(); len];
        cast_slice_mut::<_, u8>(&mut data).copy_from_slice(raw_data);

        Self { dimensions, data }
    }

    pub fn push(&mut self, data: &[T])
    where
        T: Copy,
    {
        let len = self.dimensions.iter().skip(1).product::<usize>();
        assert_eq!(len, data.len());

        self.data.extend(data);
        self.dimensions[0] += 1;
    }

    pub fn save(&self, path: impl AsRef<Path>)
    where
        T: Pod,
    {
        let data_type = DataType::try_from(TypeId::of::<T>()).unwrap();
        let file = std::fs::OpenOptions::new()
            .write(true)
            .truncate(true)
            .create(true)
            .open(path)
            .unwrap();
        let mut file = BufWriter::new(file);

        file.write_all(&[
            0,
            0,
            data_type as u8,
            u8::try_from(self.dimensions.len()).unwrap(),
        ])
        .unwrap();

        for &it in &self.dimensions {
            file.write_all(&u32::try_from(it).unwrap().to_be_bytes())
                .unwrap();
        }

        let mut buf = vec![0_u8; data_type.size() * self.data.len()]
            .into_boxed_slice();
        buf.chunks_exact_mut(data_type.size())
            .zip(
                cast_slice::<_, u8>(&self.data)
                    .chunks_exact(data_type.size()),
            )
            .for_each(|(out, v)| {
                out.copy_from_slice(v);
                if cfg!(target_endian = "little") {
                    out.reverse();
                }
            });

        file.write_all(&buf).unwrap();

        file.flush().unwrap();
    }

    pub fn idx(&self) -> IdxEntry<'_, T> {
        IdxEntry {
            dimensions: &self.dimensions,
            data: &self.data,
        }
    }

    pub fn dimensions(&self) -> &[usize] {
        &self.dimensions
    }

    pub fn len(&self) -> usize {
        self.dimensions[0]
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn data(&self) -> &[T] {
        &self.data
    }
}

impl IdxData<Fp> {
    pub fn to_u8_normalize(&self) -> IdxData<u8> {
        let mut mm = MinMaxTracker::new(0.);
        self.data.iter().for_each(|&it| mm.update(it));

        IdxData {
            dimensions: self.dimensions.clone(),
            data: self
                .data
                .iter()
                .map(|&it| {
                    let mut it = it - mm.min();
                    it /= mm.range();
                    it *= 255.;
                    it as u8
                })
                .collect(),
        }
    }

    pub fn to_u8_normalize_trunc_neg(&self) -> IdxData<u8> {
        let mut mm = MinMaxTracker::new(0.);
        self.data.iter().for_each(|&it| mm.update(it));

        IdxData {
            dimensions: self.dimensions.clone(),
            data: self
                .data
                .iter()
                .map(|&it| {
                    let mut it = it / mm.max();
                    it *= 255.;
                    it as u8
                })
                .collect(),
        }
    }
}

impl From<&IdxData<u8>> for IdxData<Fp> {
    fn from(value: &IdxData<u8>) -> Self {
        Self {
            dimensions: value.dimensions.clone(),
            data: value.data.iter().map(|&it| it as Fp / 255.).collect(),
        }
    }
}

impl From<&IdxData<Fp>> for IdxData<u8> {
    fn from(value: &IdxData<Fp>) -> Self {
        Self {
            dimensions: value.dimensions.clone(),
            data: value
                .data
                .iter()
                .map(|&it| (it.clamp(0., 1.) * 255.) as u8)
                .collect(),
        }
    }
}

#[derive(Debug)]
pub struct IdxEntry<'idx, T> {
    dimensions: &'idx [usize],
    data: &'idx [T],
}

impl<'idx, T> IdxEntry<'idx, T> {
    pub fn new(dimensions: &'idx [usize], data: &'idx [T]) -> Self {
        assert!(!dimensions.is_empty());
        assert_eq!(data.len(), dimensions.iter().product::<usize>());
        Self { dimensions, data }
    }

    pub fn index(&self, index: usize) -> Self {
        assert!(!self.dimensions.is_empty());
        let size = self.dimensions[0];
        assert!(index < size);

        let dims_rest = &self.dimensions[1..];
        let rest_dim = dims_rest.iter().product::<usize>();

        let offset = rest_dim * index;

        Self {
            dimensions: dims_rest,
            data: &self.data[offset..offset + rest_dim],
        }
    }

    pub fn iter(&self) -> IdxEntryIter<'_, T> {
        IdxEntryIter {
            entry: self,
            idx: 0,
        }
    }

    pub fn dimensions(&self) -> &'idx [usize] {
        self.dimensions
    }

    pub fn data(&self) -> &'idx [T] {
        self.data
    }
}

impl<'idx> IdxEntry<'idx, u8> {
    pub fn println_as_image(&self) {
        assert_eq!(self.dimensions.len(), 2);

        for row in self.iter() {
            for &it in &*row {
                print!(
                    "{}",
                    "\u{2588}\u{2588}"
                        .paint(Style::new().rgb(it, it, it))
                );
            }
            println!();
        }
    }
}

impl<'idx, T> Deref for IdxEntry<'idx, T> {
    type Target = [T];

    fn deref(&self) -> &'idx Self::Target {
        self.data()
    }
}

pub struct IdxEntryIter<'idx, T> {
    entry: &'idx IdxEntry<'idx, T>,
    idx: usize,
}

impl<'idx, T> Iterator for IdxEntryIter<'idx, T> {
    type Item = IdxEntry<'idx, T>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.idx >= self.entry.dimensions[0] {
            return None;
        }

        let res = self.entry.index(self.idx);
        self.idx += 1;

        Some(res)
    }
}
