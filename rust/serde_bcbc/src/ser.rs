use serde::{
    Serialize,
    ser::{self, SerializeStruct, SerializeTuple},
};

use crate::{
    error::{Error, Result},
    raw::{self, Tag},
    ser::raw_ser::RawSerializer,
    value::{
        EMPTY_TUPLE, generics::Generics, r#type::Type, type_id::TypeId,
    },
};

mod raw_ser;

pub struct Serializer {
    out: Vec<u8>,
}

impl Serializer {
    pub fn new() -> Self {
        Self { out: vec![] }
    }

    pub fn into_output(self) -> Vec<u8> {
        self.out
    }
}

impl Serializer {
    fn write_uleb128<N: leb128::NumUnsigned>(&mut self, v: N) {
        leb128::encode(v, &mut self.out);
    }

    fn write_sleb128<N: leb128::NumSigned>(&mut self, v: N) {
        leb128::encode_signed(v, &mut self.out);
    }

    fn write_bytes(&mut self, v: &[u8]) {
        const _: () = assert!(usize::BITS <= u128::BITS);

        self.write_uleb128(v.len() as u128);
        for it in v {
            self.write_uleb128(*it);
        }
    }

    fn write_str(&mut self, v: &str) {
        const _: () = assert!(usize::BITS <= u128::BITS);

        self.write_uleb128(v.chars().count() as u128);
        for ch in v.chars() {
            self.write_uleb128(ch as u32);
        }
    }

    fn write_tag(&mut self, v: Tag) {
        self.out.push(v as u8);
    }

    fn raw(&mut self, layer: usize) -> RawSerializer<'_> {
        RawSerializer { ser: self, layer }
    }
}

impl Default for Serializer {
    fn default() -> Self {
        Self::new()
    }
}

impl<'ser> serde::Serializer for &'ser mut Serializer {
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
        self.write_tag(Tag::Bool);
        self.write_uleb128(v as u128);
        Ok(())
    }

    fn serialize_i8(self, v: i8) -> Result<Self::Ok> {
        self.write_tag(Tag::Int);
        self.write_sleb128(v);
        Ok(())
    }

    fn serialize_i16(self, v: i16) -> Result<Self::Ok> {
        self.write_tag(Tag::Int);
        self.write_sleb128(v);
        Ok(())
    }

    fn serialize_i32(self, v: i32) -> Result<Self::Ok> {
        self.write_tag(Tag::Int);
        self.write_sleb128(v);
        Ok(())
    }

    fn serialize_i64(self, v: i64) -> Result<Self::Ok> {
        self.write_tag(Tag::Int);
        self.write_sleb128(v);
        Ok(())
    }

    fn serialize_i128(self, v: i128) -> Result<Self::Ok> {
        self.write_tag(Tag::Int);
        self.write_sleb128(v);
        Ok(())
    }

    fn serialize_u8(self, v: u8) -> Result<Self::Ok> {
        self.write_tag(Tag::Uint);
        self.write_uleb128(v);
        Ok(())
    }

    fn serialize_u16(self, v: u16) -> Result<Self::Ok> {
        self.write_tag(Tag::Uint);
        self.write_uleb128(v);
        Ok(())
    }

    fn serialize_u32(self, v: u32) -> Result<Self::Ok> {
        self.write_tag(Tag::Uint);
        self.write_uleb128(v);
        Ok(())
    }

    fn serialize_u64(self, v: u64) -> Result<Self::Ok> {
        self.write_tag(Tag::Uint);
        self.write_uleb128(v);
        Ok(())
    }

    fn serialize_u128(self, v: u128) -> Result<Self::Ok> {
        self.write_tag(Tag::Uint);
        self.write_uleb128(v);
        Ok(())
    }

    fn serialize_f32(self, v: f32) -> Result<Self::Ok> {
        let _ = v;
        Err(anyhow::anyhow!("f32 is unsupported").into())
    }

    fn serialize_f64(self, v: f64) -> Result<Self::Ok> {
        let _ = v;
        Err(anyhow::anyhow!("f64 is unsupported").into())
    }

    fn serialize_char(self, v: char) -> Result<Self::Ok> {
        self.serialize_u32(v as u32)
    }

    fn serialize_str(self, v: &str) -> Result<Self::Ok> {
        self.write_tag(Tag::String);
        self.write_str(v);
        Ok(())
    }

    fn serialize_bytes(self, v: &[u8]) -> Result<Self::Ok> {
        self.write_tag(Tag::Bytes);
        self.write_bytes(v);

        Ok(())
    }

    fn serialize_none(self) -> Result<Self::Ok> {
        self.write_tag(Tag::Option);
        (false, Type::Unknown).serialize(&mut self.raw(0))
    }

    fn serialize_some<T>(self, value: &T) -> Result<Self::Ok>
    where
        T: ?Sized + serde::Serialize,
    {
        self.write_tag(Tag::Option);
        (true, value).serialize(&mut self.raw(0))
    }

    fn serialize_unit(self) -> Result<Self::Ok> {
        EMPTY_TUPLE.serialize(&mut self.raw(0))
    }

    fn serialize_unit_struct(
        self,
        name: &'static str,
    ) -> Result<Self::Ok> {
        SerializeStruct::end(self.serialize_struct(name, 0)?)
    }

    fn serialize_unit_variant(
        self,
        name: &'static str,
        variant_index: u32,
        variant: &'static str,
    ) -> Result<Self::Ok> {
        let _ = (name, variant);
        self.write_tag(Tag::Enum);
        (TypeId::Anonymous, variant_index).serialize(&mut self.raw(0))
    }

    fn serialize_newtype_struct<T>(
        self,
        name: &'static str,
        value: &T,
    ) -> Result<Self::Ok>
    where
        T: ?Sized + serde::Serialize,
    {
        let mut s = self.serialize_struct(name, 0)?;
        SerializeStruct::serialize_field(&mut s, "", value)?;
        SerializeStruct::end(s)
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
        let _ = (name, variant);
        self.write_tag(Tag::Choice);
        (TypeId::Anonymous, Generics::empty(), variant_index, value)
            .serialize(&mut self.raw(0))
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

        self.write_tag(Tag::Tuple);
        self.write_uleb128(len as u128);
        Ok(Compound::Tuple {
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
        self.serialize_struct(name, len)
    }

    fn serialize_tuple_variant(
        self,
        name: &'static str,
        variant_index: u32,
        variant: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleVariant> {
        let _ = (name, variant);
        self.write_tag(Tag::Choice);

        self.write_uleb128(4_u8);
        TypeId::Anonymous.serialize(&mut *self)?;
        Generics::empty().serialize(&mut *self)?;
        variant_index.serialize(&mut *self)?;
        self.serialize_tuple(len)
    }

    fn serialize_map(
        self,
        len: Option<usize>,
    ) -> Result<Self::SerializeMap> {
        let len =
            len.ok_or_else(|| anyhow::anyhow!("expect sized map"))?;
        self.write_tag(Tag::List);
        self.write_uleb128(2_u8);

        let is_empty = len == 0;
        self.serialize_bool(!is_empty)?;

        if is_empty {
            Type::Unknown.serialize(&mut *self)?;
        } else {
            const _: () = assert!(usize::BITS <= u128::BITS);
            self.write_tag(Tag::ListItems);
            self.write_uleb128(len as u128);
        }

        Ok(Compound::Tuple {
            ser: self,
            len,
            count: 0,
        })
    }

    fn serialize_struct(
        self,
        name: &'static str,
        len: usize,
    ) -> Result<Self::SerializeStruct> {
        if name == raw::TOKEN {
            return Ok(Compound::Raw {
                ser: self,
                len,
                count: 0,
            });
        }

        self.write_tag(Tag::Struct);
        self.write_uleb128(3_u8);
        TypeId::Anonymous.serialize(&mut *self)?;
        Generics::empty().serialize(&mut *self)?;
        self.serialize_tuple(len)
    }

    fn serialize_struct_variant(
        self,
        name: &'static str,
        variant_index: u32,
        variant: &'static str,
        len: usize,
    ) -> Result<Self::SerializeStructVariant> {
        let _ = variant;
        self.write_tag(Tag::Choice);

        self.write_uleb128(4_u8);
        TypeId::Anonymous.serialize(&mut *self)?;
        Generics::empty().serialize(&mut *self)?;
        variant_index.serialize(&mut *self)?;
        self.serialize_struct(name, len)
    }

    fn is_human_readable(&self) -> bool {
        false
    }
}

pub enum Compound<'ser> {
    Raw {
        ser: &'ser mut Serializer,
        len: usize,
        count: usize,
    },
    Tuple {
        ser: &'ser mut Serializer,
        len: usize,
        count: usize,
    },
    TupleRaw {
        ser: &'ser mut RawSerializer<'ser>,
        len: usize,
        count: usize,
    },
    __,
}

impl<'ser> ser::SerializeSeq for Compound<'ser> {
    type Ok = ();

    type Error = Error;

    fn serialize_element<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + ser::Serialize,
    {
        SerializeTuple::serialize_element(self, value)
    }

    fn end(self) -> Result<Self::Ok> {
        SerializeTuple::end(self)
    }
}

impl<'ser> ser::SerializeTuple for Compound<'ser> {
    type Ok = ();

    type Error = Error;

    fn serialize_element<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + ser::Serialize,
    {
        match self {
            Self::Tuple { ser, len, count } => {
                if len == count {
                    return Err(
                        anyhow::anyhow!("tuple expect {len}").into()
                    );
                }

                let res = value.serialize(&mut **ser);

                *count += 1;

                res
            }
            Self::TupleRaw { ser, len, count } => {
                if len == count {
                    return Err(anyhow::anyhow!(
                        "tuple raw expect {len}"
                    )
                    .into());
                }

                let res = if ser.layer >= 1 {
                    value.serialize(&mut RawSerializer {
                        ser: ser.ser,
                        layer: ser.layer - 1,
                    })
                } else {
                    value.serialize(&mut *ser.ser)
                };

                *count += 1;

                res
            }
            _ => unreachable!(),
        }
    }

    fn end(self) -> Result<Self::Ok> {
        let (Self::Tuple { len, count, .. }
        | Self::TupleRaw { len, count, .. }) = self
        else {
            unreachable!()
        };

        if len != count {
            Err(anyhow::anyhow!(
                "tuple(raw) expect {len}, but only {count}"
            )
            .into())
        } else {
            Ok(())
        }
    }
}

impl<'ser> ser::SerializeTupleStruct for Compound<'ser> {
    type Ok = ();

    type Error = Error;

    fn serialize_field<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + ser::Serialize,
    {
        SerializeStruct::serialize_field(self, "", value)
    }

    fn end(self) -> Result<Self::Ok> {
        SerializeStruct::end(self)
    }
}

impl<'ser> ser::SerializeTupleVariant for Compound<'ser> {
    type Ok = ();

    type Error = Error;

    fn serialize_field<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + ser::Serialize,
    {
        SerializeTuple::serialize_element(self, value)
    }

    fn end(self) -> Result<Self::Ok> {
        SerializeTuple::end(self)
    }
}

impl<'ser> ser::SerializeMap for Compound<'ser> {
    type Ok = ();

    type Error = Error;

    fn serialize_key<T>(&mut self, key: &T) -> Result<()>
    where
        T: ?Sized + ser::Serialize,
    {
        let Self::Tuple { ser, len, count } = self else {
            unreachable!()
        };

        if len == count {
            return Err(anyhow::anyhow!("map expect {len}").into());
        }

        ser.write_tag(Tag::Tuple);
        ser.write_uleb128(2_u8);
        key.serialize(&mut **ser)?;

        *count += 1;

        Ok(())
    }

    fn serialize_value<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + ser::Serialize,
    {
        let Self::Tuple { ser, .. } = self else {
            unreachable!()
        };

        value.serialize(&mut **ser)
    }

    fn end(self) -> Result<Self::Ok> {
        let Self::Tuple { len, count, .. } = self else {
            unreachable!()
        };

        if len != count {
            Err(anyhow::anyhow!("map expect {len}, but only {count}")
                .into())
        } else {
            Ok(())
        }
    }
}

impl<'ser> ser::SerializeStruct for Compound<'ser> {
    type Ok = ();

    type Error = Error;

    fn serialize_field<T>(
        &mut self,
        key: &'static str,
        value: &T,
    ) -> Result<()>
    where
        T: ?Sized + ser::Serialize,
    {
        match self {
            Compound::Raw { ser, len, count } => {
                if len == count {
                    return Err(
                        anyhow::anyhow!("raw expect {len}").into()
                    );
                }

                let res = if key == "$uints" {
                    value.serialize(&mut ser.raw(1))
                } else {
                    value.serialize(&mut ser.raw(0))
                };

                *count += 1;

                res
            }
            Compound::Tuple { .. } => {
                SerializeTuple::serialize_element(self, value)
            }
            _ => unreachable!(),
        }
    }

    fn end(self) -> Result<Self::Ok> {
        match self {
            Compound::Raw { len, count, .. } => {
                if len != count {
                    Err(anyhow::anyhow!(
                        "raw expect {len}, but only {count}"
                    )
                    .into())
                } else {
                    Ok(())
                }
            }
            Compound::Tuple { .. } => SerializeTuple::end(self),
            _ => unreachable!(),
        }
    }
}

impl<'ser> ser::SerializeStructVariant for Compound<'ser> {
    type Ok = ();

    type Error = Error;

    fn serialize_field<T>(
        &mut self,
        key: &'static str,
        value: &T,
    ) -> Result<()>
    where
        T: ?Sized + ser::Serialize,
    {
        SerializeStruct::serialize_field(self, key, value)
    }

    fn end(self) -> Result<Self::Ok> {
        SerializeStruct::end(self)
    }
}
