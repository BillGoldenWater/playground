use std::fmt;

use serde::{de, ser::SerializeStruct as _};

use crate::{
    raw::{self, RawKey, Tag, TagKey},
    value::{r#type::Type, visitor_expect_tag},
};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub(crate) struct Generics(Box<[Type]>);

impl From<Box<[Type]>> for Generics {
    fn from(value: Box<[Type]>) -> Self {
        Self(value)
    }
}

impl Generics {
    pub fn new(items: Box<[Type]>) -> Self {
        Self(items)
    }

    pub fn empty() -> Self {
        Self::new([].into())
    }
}

struct GenericsKey;
impl raw::RawKey for GenericsKey {
    const KEY: &str = "$generics";
}

impl serde::Serialize for Generics {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut s = serializer.serialize_struct(raw::TOKEN, 2)?;
        s.serialize_field(TagKey::KEY, &Tag::Generics)?;
        s.serialize_field(GenericsKey::KEY, &self.0)?;
        s.end()
    }
}

impl<'de> serde::Deserialize<'de> for Generics {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct GenericsVisitor;
        impl<'de> de::Visitor<'de> for GenericsVisitor {
            type Value = Box<[Type]>;

            fn expecting(
                &self,
                formatter: &mut fmt::Formatter,
            ) -> fmt::Result {
                formatter.write_str("a bcbc generics")
            }

            fn visit_map<A>(
                self,
                mut map: A,
            ) -> Result<Self::Value, A::Error>
            where
                A: de::MapAccess<'de>,
            {
                visitor_expect_tag(&mut map, Tag::Generics, &self)?;

                let key = map.next_key::<raw::Key<GenericsKey>>()?;
                if key.is_none() {
                    Err(de::Error::invalid_type(
                        de::Unexpected::Map,
                        &self,
                    ))
                } else {
                    map.next_value::<Box<[Type]>>()
                }
            }
        }

        deserializer
            .deserialize_struct(
                raw::TOKEN,
                &[TagKey::KEY, GenericsKey::KEY],
                GenericsVisitor,
            )
            .map(Self)
    }
}
