use std::fmt;

use serde::{de, ser::SerializeStruct as _};

use crate::{
    raw::{self, RawKey, Tag, TagKey},
    value::visitor_expect_tag,
};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct Uints(Box<[u128]>);

impl Uints {
    pub fn new(items: Box<[u128]>) -> Self {
        Self::from(items)
    }
}

impl From<Box<[u128]>> for Uints {
    fn from(value: Box<[u128]>) -> Self {
        Self(value)
    }
}

struct UintsKey;
impl raw::RawKey for UintsKey {
    const KEY: &str = "$uints";
}

impl serde::Serialize for Uints {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut s = serializer.serialize_struct(raw::TOKEN, 2)?;
        s.serialize_field(TagKey::KEY, &Tag::Uints)?;
        s.serialize_field(UintsKey::KEY, &self.0)?;
        s.end()
    }
}

impl<'de> serde::Deserialize<'de> for Uints {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct UintsVisitor;
        impl<'de> de::Visitor<'de> for UintsVisitor {
            type Value = Box<[u128]>;

            fn expecting(
                &self,
                formatter: &mut fmt::Formatter,
            ) -> fmt::Result {
                formatter.write_str("a bcbc raw uints")
            }

            fn visit_map<A>(
                self,
                mut map: A,
            ) -> Result<Self::Value, A::Error>
            where
                A: de::MapAccess<'de>,
            {
                visitor_expect_tag(&mut map, Tag::Uints, &self)?;

                let key = map.next_key::<raw::Key<UintsKey>>()?;
                if key.is_none() {
                    Err(de::Error::invalid_type(
                        de::Unexpected::Map,
                        &self,
                    ))
                } else {
                    map.next_value::<Box<[u128]>>()
                }
            }
        }

        deserializer
            .deserialize_struct(
                raw::TOKEN,
                &[TagKey::KEY, UintsKey::KEY],
                UintsVisitor,
            )
            .map(Self)
    }
}
