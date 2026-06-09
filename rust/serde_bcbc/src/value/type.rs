use std::fmt;

use serde::{de, ser::SerializeStruct as _};

use crate::{
    raw::{self, RawKey, Tag, TagKey},
    value::{
        EMPTY_TUPLE, EmptyTuple, type_id::TypeId, visitor_expect_tag,
    },
};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub enum Type {
    #[default]
    Unknown,
    Uint,
    Int,
    Bool,
    Uints,
    Bytes,
    String,
    //
    Tuple(Box<[Type]>),
    List(Box<Type>),
    Option(Box<Type>),
    //
    Alias(TypeId, Box<[Type]>),
    Enum(TypeId),
    Choice(TypeId, Box<[Type]>),
    Struct(TypeId, Box<[Type]>),
    //
    Type,
    TypeId,
}

impl Type {
    fn to_tag(&self) -> u8 {
        match self {
            Self::Unknown => b'0',
            Self::Uint => b'u',
            Self::Int => b'i',
            Self::Bool => b'f',
            Self::Uints => b'n',
            Self::Bytes => b'b',
            Self::String => b's',
            //
            Self::Tuple(..) => b'p',
            Self::List(..) => b'l',
            Self::Option(..) => b'o',
            //
            Self::Alias(..) => b'a',
            Self::Enum(..) => b'e',
            Self::Choice(..) => b'c',
            Self::Struct(..) => b'r',
            //
            Self::Type => b't',
            Self::TypeId => b'd',
        }
    }
}

struct TypeKey;
impl raw::RawKey for TypeKey {
    const KEY: &str = "$type";
}

impl serde::Serialize for Type {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut s = serializer.serialize_struct(raw::TOKEN, 2)?;

        s.serialize_field(TagKey::KEY, &Tag::Type)?;

        let tag = self.to_tag();
        match self {
            Type::Unknown
            | Type::Uint
            | Type::Int
            | Type::Bool
            | Type::Uints
            | Type::Bytes
            | Type::String
            | Type::Type
            | Type::TypeId => {
                s.serialize_field(TypeKey::KEY, &(tag, EMPTY_TUPLE))?;
            }
            Type::Tuple(items) => {
                s.serialize_field(TypeKey::KEY, &(tag, items))?
            }
            Type::List(ty) | Type::Option(ty) => {
                s.serialize_field(TypeKey::KEY, &(tag, ty))?
            }
            Type::Enum(type_id) => {
                s.serialize_field(TypeKey::KEY, &(tag, type_id))?
            }
            Type::Alias(type_id, items)
            | Type::Choice(type_id, items)
            | Type::Struct(type_id, items) => {
                s.serialize_field(TypeKey::KEY, &(tag, (type_id, items)))?
            }
        }

        s.end()
    }
}

impl<'de> serde::Deserialize<'de> for Type {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct TypeContentVisitor;
        impl<'de> de::Visitor<'de> for TypeContentVisitor {
            type Value = TypeContent;

            fn expecting(
                &self,
                formatter: &mut fmt::Formatter,
            ) -> fmt::Result {
                formatter.write_str("bcbc type content")
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

                let r#type = match tag {
                    b'0' | b'u' | b'i' | b'f' | b'n' | b'b' | b's'
                    | b't' | b'd' => {
                        seq.next_element::<EmptyTuple>()?.ok_or(
                            de::Error::invalid_value(
                                de::Unexpected::Seq,
                                &"empty tuple",
                            ),
                        )?;

                        match tag {
                            b'0' => Type::Unknown,
                            b'u' => Type::Uint,
                            b'i' => Type::Int,
                            b'f' => Type::Bool,
                            b'n' => Type::Uints,
                            b'b' => Type::Bytes,
                            b's' => Type::String,
                            b't' => Type::Type,
                            b'd' => Type::TypeId,
                            _ => unreachable!(),
                        }
                    }
                    b'p' => {
                        let items = seq
                            .next_element::<Box<[Type]>>()?
                            .ok_or(de::Error::invalid_value(
                                de::Unexpected::Seq,
                                &"types of tuple",
                            ))?;

                        Type::Tuple(items)
                    }
                    b'l' | b'o' => {
                        let ty = seq
                            .next_element::<Type>()?
                            .ok_or(de::Error::invalid_value(
                                de::Unexpected::Seq,
                                &"types of list/option",
                            ))?
                            .into();

                        match tag {
                            b'l' => Type::List(ty),
                            b'o' => Type::Option(ty),
                            _ => unreachable!(),
                        }
                    }
                    b'e' => {
                        let type_id = seq
                            .next_element::<TypeId>()?
                            .ok_or(de::Error::invalid_value(
                                de::Unexpected::Seq,
                                &"typeid of enum",
                            ))?;

                        Type::Enum(type_id)
                    }
                    b'a' | b'c' | b'r' => {
                        let (type_id, items) = seq
                            .next_element::<(TypeId, Box<[Type]>)>()?
                            .ok_or(de::Error::invalid_value(
                                de::Unexpected::Seq,
                                &"type id and items \
                                of alias/choice/struct",
                            ))?;

                        match tag {
                            b'a' => Type::Alias(type_id, items),
                            b'c' => Type::Choice(type_id, items),
                            b'r' => Type::Struct(type_id, items),
                            _ => unreachable!(),
                        }
                    }
                    _ => {
                        return Err(de::Error::invalid_value(
                            de::Unexpected::Seq,
                            &"type tag",
                        ));
                    }
                };

                Ok(TypeContent(r#type))
            }
        }

        struct TypeContent(Type);
        impl<'de> serde::Deserialize<'de> for TypeContent {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: de::Deserializer<'de>,
            {
                deserializer.deserialize_tuple(2, TypeContentVisitor)
            }
        }

        struct TypeVisitor;
        impl<'de> de::Visitor<'de> for TypeVisitor {
            type Value = Type;

            fn expecting(
                &self,
                formatter: &mut fmt::Formatter,
            ) -> fmt::Result {
                formatter.write_str("a bcbc type")
            }

            fn visit_map<A>(
                self,
                mut map: A,
            ) -> Result<Self::Value, A::Error>
            where
                A: de::MapAccess<'de>,
            {
                visitor_expect_tag(&mut map, Tag::Type, &self)?;

                let key = map.next_key::<raw::Key<TypeKey>>()?;
                if key.is_none() {
                    Err(de::Error::invalid_type(
                        de::Unexpected::Map,
                        &self,
                    ))
                } else {
                    map.next_value::<TypeContent>().map(|it| it.0)
                }
            }
        }

        deserializer.deserialize_struct(
            raw::TOKEN,
            &[TagKey::KEY, TypeKey::KEY],
            TypeVisitor,
        )
    }
}
