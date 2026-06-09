use std::{
    fmt::{self, Display as _, Formatter},
    marker::PhantomData,
};

use serde::{Deserializer, Serialize, Serializer, de};

pub(crate) const TOKEN: &str = "$serde_bcbc::private::raw";

pub(crate) trait RawKey {
    const KEY: &str;
}

#[derive(Debug)]
pub(crate) struct Key<T: RawKey>(PhantomData<T>);

impl<T: RawKey> Key<T> {
    pub(crate) fn new() -> Self {
        Self(PhantomData)
    }
}

impl<'de, T: RawKey> serde::Deserialize<'de> for Key<T> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct KeyVisitor<T: RawKey>(PhantomData<T>);

        impl<'de, T: RawKey> de::Visitor<'de> for KeyVisitor<T> {
            type Value = ();

            fn expecting(
                &self,
                formatter: &mut Formatter,
            ) -> fmt::Result {
                format_args!("a valid {} key", T::KEY).fmt(formatter)
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                if v == T::KEY {
                    Ok(())
                } else {
                    Err(de::Error::custom(
                        "expected key with custom name",
                    ))
                }
            }
        }

        deserializer
            .deserialize_identifier(KeyVisitor::<T>(PhantomData))?;
        Ok(Key::new())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub(crate) enum Tag {
    Uint = b'U',
    Int = b'I',
    Bool = b'F',
    Uints = b'N',
    Bytes = b'B',
    String = b'S',
    //
    ListItems = b'M',
    Generics = b'G',
    //
    Tuple = b'P',
    List = b'L',
    Option = b'O',
    //
    Alias = b'A',
    Enum = b'E',
    Choice = b'C',
    Struct = b'R',
    //
    Type = b'T',
    TypeId = b'D',
}

impl Tag {
    pub fn to_str(self) -> &'static str {
        match self {
            Tag::Uint => "Uint",
            Tag::Int => "Int",
            Tag::Bool => "Bool",
            Tag::Uints => "Uints",
            Tag::Bytes => "Bytes",
            Tag::String => "String",
            Tag::ListItems => "ListItems",
            Tag::Generics => "Generics",
            Tag::Tuple => "Tuple",
            Tag::List => "List",
            Tag::Option => "Option",
            Tag::Alias => "Alias",
            Tag::Enum => "Enum",
            Tag::Choice => "Choice",
            Tag::Struct => "Struct",
            Tag::Type => "Type",
            Tag::TypeId => "TypeId",
        }
    }
}

impl TryFrom<u8> for Tag {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        Ok(match value {
            b'U' => Self::Uint,
            b'I' => Self::Int,
            b'F' => Self::Bool,
            b'N' => Self::Uints,
            b'B' => Self::Bytes,
            b'S' => Self::String,
            //
            b'M' => Self::ListItems,
            b'G' => Self::Generics,
            //
            b'P' => Self::Tuple,
            b'L' => Self::List,
            b'O' => Self::Option,
            //
            b'A' => Self::Alias,
            b'E' => Self::Enum,
            b'C' => Self::Choice,
            b'R' => Self::Struct,
            //
            b'T' => Self::Type,
            b'D' => Self::TypeId,
            _ => return Err(()),
        })
    }
}

pub(crate) struct TagKey;
impl RawKey for TagKey {
    const KEY: &str = "$tag";
}

impl Serialize for Tag {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_u8(*self as u8)
    }
}

impl<'de> serde::Deserialize<'de> for Tag {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        fn map<E: de::Error>(
            r: Result<u8, std::num::TryFromIntError>,
        ) -> Result<Tag, E> {
            let tag =
                r.map_err(|it| E::custom(format!("invalid tag: {it}")))?;
            Tag::try_from(tag)
                .map_err(|_| E::custom(format!("unknown tag: {tag}")))
        }

        macro_rules! visit {
            ($fn_name:ident, $in_ty:ty) => {
                fn $fn_name<E>(self, v: $in_ty) -> Result<Self::Value, E>
                where
                    E: de::Error,
                {
                    map(v.try_into())
                }
            };
        }

        struct TagVisitor;
        impl<'de> de::Visitor<'de> for TagVisitor {
            type Value = Tag;

            fn expecting(
                &self,
                formatter: &mut Formatter,
            ) -> fmt::Result {
                formatter.write_str("a bcbc tag")
            }

            fn visit_u8<E>(self, v: u8) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                map((v as u16).try_into())
            }

            visit!(visit_u16, u16);
            visit!(visit_u32, u32);
            visit!(visit_u64, u64);
            visit!(visit_u128, u128);

            visit!(visit_i8, i8);
            visit!(visit_i16, i16);
            visit!(visit_i32, i32);
            visit!(visit_i64, i64);
            visit!(visit_i128, i128);
        }

        deserializer.deserialize_any(TagVisitor)
    }
}
