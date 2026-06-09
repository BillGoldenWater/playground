use std::fmt;

use serde::{de, ser::SerializeStruct as _};

use crate::{
    raw::{self, RawKey, Tag, TagKey},
    value::{EMPTY_TUPLE, EmptyTuple, visitor_expect_tag},
};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub enum TypeId {
    #[default]
    Anonymous,
    Std(u128),
    Thirdparty,
}

struct TypeIdKey;
impl raw::RawKey for TypeIdKey {
    const KEY: &str = "$type_id";
}

impl serde::Serialize for TypeId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut s = serializer.serialize_struct(raw::TOKEN, 2)?;
        s.serialize_field(TagKey::KEY, &Tag::TypeId)?;
        match self {
            Self::Anonymous => {
                s.serialize_field(TypeIdKey::KEY, &(b'x', EMPTY_TUPLE))?;
            }
            Self::Std(id) => {
                s.serialize_field(TypeIdKey::KEY, &(b'y', id))?;
            }
            Self::Thirdparty => todo!(),
        }
        s.end()
    }
}

impl<'de> serde::Deserialize<'de> for TypeId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct TypeIdContent(TypeId);
        impl<'de> serde::Deserialize<'de> for TypeIdContent {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: de::Deserializer<'de>,
            {
                struct TypeIdContentVisitor;
                impl<'de> de::Visitor<'de> for TypeIdContentVisitor {
                    type Value = TypeIdContent;

                    fn expecting(
                        &self,
                        formatter: &mut fmt::Formatter,
                    ) -> fmt::Result {
                        formatter.write_str("bcbc type id content")
                    }

                    fn visit_seq<A>(
                        self,
                        mut seq: A,
                    ) -> Result<Self::Value, A::Error>
                    where
                        A: de::SeqAccess<'de>,
                    {
                        let Some(tag) = seq.next_element::<u8>()? else {
                            return Err(de::Error::invalid_type(
                                de::Unexpected::Seq,
                                &self,
                            ));
                        };

                        let type_id = match tag {
                            b'x' => {
                                seq.next_element::<EmptyTuple>()?.ok_or(
                                    de::Error::invalid_value(
                                        de::Unexpected::Seq,
                                        &"empty tuple",
                                    ),
                                )?;
                                TypeId::Anonymous
                            }
                            b'y' => {
                                let it = seq
                                    .next_element::<u128>()?
                                    .ok_or(de::Error::invalid_value(
                                        de::Unexpected::Seq,
                                        &"uint",
                                    ))?;
                                TypeId::Std(it)
                            }
                            b'z' => {
                                todo!()
                            }
                            _ => {
                                return Err(de::Error::invalid_value(
                                    de::Unexpected::Seq,
                                    &"x or y or z",
                                ));
                            }
                        };

                        Ok(TypeIdContent(type_id))
                    }
                }

                deserializer.deserialize_tuple(2, TypeIdContentVisitor)
            }
        }

        struct TypeIdVisitor;
        impl<'de> de::Visitor<'de> for TypeIdVisitor {
            type Value = TypeId;

            fn expecting(
                &self,
                formatter: &mut fmt::Formatter,
            ) -> fmt::Result {
                formatter.write_str("a bcbc type id")
            }

            fn visit_map<A>(
                self,
                mut map: A,
            ) -> Result<Self::Value, A::Error>
            where
                A: de::MapAccess<'de>,
            {
                visitor_expect_tag(&mut map, Tag::TypeId, &self)?;

                let key = map.next_key::<raw::Key<TypeIdKey>>()?;
                if key.is_none() {
                    Err(de::Error::invalid_type(
                        de::Unexpected::Map,
                        &self,
                    ))
                } else {
                    map.next_value::<TypeIdContent>().map(|it| it.0)
                }
            }
        }

        deserializer.deserialize_struct(
            raw::TOKEN,
            &[TagKey::KEY, TypeIdKey::KEY],
            TypeIdVisitor,
        )
    }
}
