use std::{fmt, marker::PhantomData};

use serde::{
    Serialize,
    de::{self, DeserializeOwned},
    ser::SerializeStruct as _,
};

use crate::{
    raw::{self, RawKey, Tag, TagKey},
    value::visitor_expect_tag,
};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub(crate) struct ListItems<T>(Box<[T]>);

struct ListItemsKey;
impl raw::RawKey for ListItemsKey {
    const KEY: &str = "$list_items";
}

impl<T: Serialize> serde::Serialize for ListItems<T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut s = serializer.serialize_struct(raw::TOKEN, 2)?;
        s.serialize_field(TagKey::KEY, &Tag::ListItems)?;
        s.serialize_field(ListItemsKey::KEY, &self.0)?;
        s.end()
    }
}

impl<'de, T: DeserializeOwned> serde::Deserialize<'de> for ListItems<T> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct ListItemsVisitor<T>(PhantomData<T>);
        impl<'de, T: DeserializeOwned> de::Visitor<'de> for ListItemsVisitor<T> {
            type Value = Box<[T]>;

            fn expecting(
                &self,
                formatter: &mut fmt::Formatter,
            ) -> fmt::Result {
                formatter.write_str("a bcbc list_items")
            }

            fn visit_map<A>(
                self,
                mut map: A,
            ) -> Result<Self::Value, A::Error>
            where
                A: de::MapAccess<'de>,
            {
                visitor_expect_tag(&mut map, Tag::ListItems, &self)?;

                let key = map.next_key::<raw::Key<ListItemsKey>>()?;
                if key.is_none() {
                    Err(de::Error::invalid_type(
                        de::Unexpected::Map,
                        &self,
                    ))
                } else {
                    map.next_value::<Box<[T]>>()
                }
            }
        }

        deserializer
            .deserialize_struct(
                raw::TOKEN,
                &[TagKey::KEY, ListItemsKey::KEY],
                ListItemsVisitor::<T>(PhantomData),
            )
            .map(Self)
    }
}
