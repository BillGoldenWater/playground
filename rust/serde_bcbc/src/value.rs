use serde::de;

use crate::raw::{self, Tag, TagKey};

pub mod generics;
pub mod list_items;
pub mod r#type;
pub mod type_id;
pub mod uints;

pub(crate) type EmptyTuple = [u8; 0];
pub(crate) const EMPTY_TUPLE: EmptyTuple = [];

fn visitor_expect_tag<'de, A: de::MapAccess<'de>>(
    map: &mut A,
    expected_tag: Tag,
    exp: &dyn de::Expected,
) -> Result<(), A::Error> {
    let key = map.next_key::<raw::Key<TagKey>>()?;

    let tag = if key.is_none() {
        return Err(de::Error::invalid_type(de::Unexpected::Map, exp));
    } else {
        map.next_value::<Tag>()?
    };

    if tag != expected_tag {
        return Err(de::Error::invalid_type(
            de::Unexpected::Other(tag.to_str()),
            &expected_tag.to_str(),
        ));
    }

    Ok(())
}
