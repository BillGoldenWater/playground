use super::{Compound, Error};
use crate::error::Result;

pub struct RawSerializer<'ser> {
    pub(super) ser: &'ser mut super::Serializer,
    pub(super) layer: usize,
}

impl<'ser> serde::Serializer for &'ser mut RawSerializer<'ser> {
    type Ok = ();

    type Error = Error;

    type SerializeSeq = Compound<'ser>;

    type SerializeTuple = Compound<'ser>;

    type SerializeTupleStruct = Compound<'ser>;

    type SerializeTupleVariant = Compound<'ser>;

    type SerializeMap = Compound<'ser>;

    type SerializeStruct = Compound<'ser>;

    type SerializeStructVariant = Compound<'ser>;

    fn serialize_bool(self, v: bool) -> Result<Self::Ok> {
        self.ser.serialize_u8(v as u8)
    }

    fn serialize_i8(self, v: i8) -> Result<Self::Ok> {
        self.ser.write_sleb128(v);
        Ok(())
    }

    fn serialize_i16(self, v: i16) -> Result<Self::Ok> {
        self.ser.write_sleb128(v);
        Ok(())
    }

    fn serialize_i32(self, v: i32) -> Result<Self::Ok> {
        self.ser.write_sleb128(v);
        Ok(())
    }

    fn serialize_i64(self, v: i64) -> Result<Self::Ok> {
        self.ser.write_sleb128(v);
        Ok(())
    }

    fn serialize_i128(self, v: i128) -> Result<Self::Ok> {
        self.ser.write_sleb128(v);
        Ok(())
    }

    fn serialize_u8(self, v: u8) -> Result<Self::Ok> {
        self.ser.write_uleb128(v);
        Ok(())
    }

    fn serialize_u16(self, v: u16) -> Result<Self::Ok> {
        self.ser.write_uleb128(v);
        Ok(())
    }

    fn serialize_u32(self, v: u32) -> Result<Self::Ok> {
        self.ser.write_uleb128(v);
        Ok(())
    }

    fn serialize_u64(self, v: u64) -> Result<Self::Ok> {
        self.ser.write_uleb128(v);
        Ok(())
    }

    fn serialize_u128(self, v: u128) -> Result<Self::Ok> {
        self.ser.write_uleb128(v);
        Ok(())
    }

    fn serialize_f32(self, v: f32) -> Result<Self::Ok> {
        let _ = v;
        unreachable!()
    }

    fn serialize_f64(self, v: f64) -> Result<Self::Ok> {
        let _ = v;
        unreachable!()
    }

    fn serialize_char(self, v: char) -> Result<Self::Ok> {
        let _ = v;
        unreachable!()
    }

    fn serialize_str(self, v: &str) -> Result<Self::Ok> {
        let _ = v;
        unreachable!()
    }

    fn serialize_bytes(self, v: &[u8]) -> Result<Self::Ok> {
        let _ = v;
        unreachable!()
    }

    fn serialize_none(self) -> Result<Self::Ok> {
        unreachable!()
    }

    fn serialize_some<T>(self, value: &T) -> Result<Self::Ok>
    where
        T: ?Sized + serde::Serialize,
    {
        let _ = value;
        unreachable!()
    }

    fn serialize_unit(self) -> Result<Self::Ok> {
        unreachable!()
    }

    fn serialize_unit_struct(
        self,
        name: &'static str,
    ) -> Result<Self::Ok> {
        let _ = name;
        unreachable!()
    }

    fn serialize_unit_variant(
        self,
        name: &'static str,
        variant_index: u32,
        variant: &'static str,
    ) -> Result<Self::Ok> {
        let _ = (name, variant_index, variant);
        unreachable!()
    }

    fn serialize_newtype_struct<T>(
        self,
        name: &'static str,
        value: &T,
    ) -> Result<Self::Ok>
    where
        T: ?Sized + serde::Serialize,
    {
        let _ = (name, value);
        unreachable!()
    }

    fn serialize_newtype_variant<T>(
        self,
        name: &'static str,
        variant_index: u32,
        variant: &'static str,
        value: &T,
    ) -> Result<Self::Ok>
    where
        T: ?Sized + serde::Serialize,
    {
        let _ = (name, variant_index, variant, value);
        unreachable!()
    }

    fn serialize_seq(
        self,
        len: Option<usize>,
    ) -> Result<Self::SerializeSeq> {
        let len =
            len.ok_or_else(|| anyhow::anyhow!("expect sized sequence"))?;
        self.serialize_tuple(len)
    }

    fn serialize_tuple(self, len: usize) -> Result<Self::SerializeTuple> {
        const _: () = assert!(usize::BITS <= u128::BITS);
        self.ser.write_uleb128(len as u128);

        Ok(Compound::TupleRaw {
            ser: self,
            len,
            count: 0,
        })
    }

    fn serialize_tuple_struct(
        self,
        name: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleStruct> {
        let _ = (name, len);
        unreachable!()
    }

    fn serialize_tuple_variant(
        self,
        name: &'static str,
        variant_index: u32,
        variant: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleVariant> {
        let _ = (name, variant_index, variant, len);
        unreachable!()
    }

    fn serialize_map(
        self,
        len: Option<usize>,
    ) -> Result<Self::SerializeMap> {
        let _ = len;
        unreachable!()
    }

    fn serialize_struct(
        self,
        name: &'static str,
        len: usize,
    ) -> Result<Self::SerializeStruct> {
        let _ = (name, len);
        unreachable!()
    }

    fn serialize_struct_variant(
        self,
        name: &'static str,
        variant_index: u32,
        variant: &'static str,
        len: usize,
    ) -> Result<Self::SerializeStructVariant> {
        let _ = (name, variant_index, variant, len);
        unreachable!()
    }

    fn is_human_readable(&self) -> bool {
        false
    }
}
