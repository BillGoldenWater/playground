use std::path::Path;

use super::Fp;
use crate::idx_data::{IdxData, IdxEntry};

#[derive(Debug)]
pub struct Mnist {
    pub(crate) labels: IdxData<u8>,
    pub(crate) images: IdxData<u8>,
    pub(crate) images_fp: IdxData<Fp>,
    pub(crate) len: usize,
}

impl Mnist {
    pub fn new(
        labels: impl AsRef<Path>,
        images: impl AsRef<Path>,
    ) -> Self {
        let labels = IdxData::<u8>::load(labels);
        let images = IdxData::<u8>::load(images);
        assert_eq!(images.len(), labels.len());
        let images_fp: IdxData<Fp> = (&images).into();

        let len = labels.len();

        Self {
            labels,
            images,
            images_fp,
            len,
        }
    }

    pub fn iter(&self) -> MnistIter<'_> {
        MnistIter {
            mnist: self,
            idx: 0,
        }
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

#[derive(Debug, Clone)]
pub struct MnistIter<'m> {
    pub(crate) mnist: &'m Mnist,
    pub(crate) idx: usize,
}

impl<'a> Iterator for MnistIter<'a> {
    type Item = MnistEntry<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.idx >= self.mnist.len {
            return None;
        }

        let label = self.mnist.labels.idx().index(self.idx)[0];
        let data = self.mnist.images.idx().index(self.idx);
        let data_fp = self.mnist.images_fp.idx().index(self.idx);
        self.idx += 1;

        Some(MnistEntry {
            label,
            data,
            data_fp,
        })
    }
}

#[derive(Debug)]
pub struct MnistEntry<'data> {
    pub(crate) label: u8,
    pub(crate) data: IdxEntry<'data, u8>,
    pub(crate) data_fp: IdxEntry<'data, Fp>,
}

impl<'data> MnistEntry<'data> {
    pub fn new(
        label: u8,
        data: IdxEntry<'data, u8>,
        data_fp: IdxEntry<'data, Fp>,
    ) -> Self {
        Self {
            label,
            data,
            data_fp,
        }
    }

    pub fn label(&self) -> u8 {
        self.label
    }

    pub fn data(&self) -> &IdxEntry<'data, u8> {
        &self.data
    }

    pub fn data_fp(&self) -> &IdxEntry<'data, Fp> {
        &self.data_fp
    }
}
