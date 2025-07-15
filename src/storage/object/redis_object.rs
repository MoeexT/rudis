use std::collections::{HashMap, HashSet};

use modular_bitfield::{Specifier, bitfield, prelude::B24};

use crate::{resp::RespValue, storage::object::encoding::sds::{self, EmbStr, Raw}};

#[derive(Specifier, Debug, PartialEq, Eq, Clone)]
#[bits = 8]
pub enum ObjectType {
    String,
    List,
    Hash,
    Set,
    Zset,
}

#[bitfield(bytes = 8)]
#[derive(Specifier, Debug, PartialEq, Eq, Clone)]
pub struct ObjectHeader {
    #[bits = 8]
    pub obj_type: ObjectType,

    #[bits = 24]
    pub lru: B24,

    #[bits = 32]
    pub ref_count: u32,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum RedisValue {
    Int(i64),
    EmbStr(EmbStr),
    Raw(Raw),
    HashTable(HashMap<String, String>),
    LinkedList(Vec<Box<RedisObject>>),
    ZipList,
    IntSet(HashSet<i64>),
    SkipList,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct RedisObject {
    header: ObjectHeader,
    ptr: RedisValue,
}

impl RedisObject {
    pub fn new_string(buf: Vec<u8>) -> Self {
        if buf.len() <= sds::EMB_LEN {
            Self {
                header: ObjectHeader::new().with_obj_type(ObjectType::String),
                ptr: RedisValue::EmbStr(buf.into()),
            }
        } else {
            Self {
                header: ObjectHeader::new().with_obj_type(ObjectType::String),
                ptr: RedisValue::Raw(buf.into()),
            }
        }
    }
}

impl Into<RespValue> for RedisObject {
    fn into(self) -> RespValue {
        match self.header.obj_type() {
            ObjectType::String => {
                match self.ptr {
                    RedisValue::Int(i) => RespValue::BulkString(Some(i.to_string().as_bytes().to_vec())),
                    RedisValue::EmbStr(emb_str) => RespValue::BulkString(Some(emb_str.into())),
                    RedisValue::Raw(raw) => RespValue::BulkString(Some(raw.into())),
                    _ => RespValue::Null,
                }
            },
            ObjectType::List => RespValue::Error("Not Implemented".to_string()),
            ObjectType::Hash => RespValue::Error("Not Implemented".to_string()),
            ObjectType::Set => RespValue::Error("Not Implemented".to_string()),
            ObjectType::Zset => RespValue::Error("Not Implemented".to_string()),
        }
    }
}


mod test {
    #[cfg(test)]
    use crate::storage::object::{
        encoding::sds::{EmbStr, Raw},
        redis_object::{ObjectHeader, ObjectType, RedisObject, RedisValue},
    };
    #[cfg(test)]
    use std::mem;

    #[test]
    fn ensure_static_size() {
        dbg!(mem::size_of::<EmbStr>());
        dbg!(mem::size_of::<Raw>());
        dbg!(mem::size_of::<ObjectType>());
        dbg!(mem::size_of::<ObjectHeader>());
        dbg!(mem::size_of::<RedisValue>());
        dbg!(mem::size_of::<RedisObject>());
        assert_eq!(mem::size_of::<ObjectType>(), 1);
        assert_eq!(mem::size_of::<ObjectHeader>(), 8);
        assert_eq!(mem::size_of::<RedisValue>(), 56);
        assert_eq!(mem::size_of::<RedisObject>(), 64);
    }

    #[test]
    fn test_new_string_object_on_buf_len_less_than_emblen() {
        let obj = RedisObject::new_string("string".as_bytes().to_vec());

        assert_eq!(obj.header.obj_type(), ObjectType::String);
        assert!(matches!(obj.ptr, RedisValue::EmbStr(_)));
    }

    #[test]
    fn test_new_string_object_on_buf_len_more_than_emblen() {
        let obj = RedisObject::new_string("string".repeat(20).as_bytes().to_vec());

        assert_eq!(obj.header.obj_type(), ObjectType::String);
        assert!(matches!(obj.ptr, RedisValue::Raw(_)));
    }
}
