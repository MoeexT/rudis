use std::{ops::Range, sync::Arc};

use async_trait::async_trait;
use rudis_macros::register;

use crate::object::redis_object::{ObjectType, RedisValue};
use crate::{
    command::{CommandExecutor, error::CommandError, registry::CommandResult},
    context::Context,
    resp::RespValue,
};

#[register("GETRANGE")]
struct GetRangeCommand {
    key: String,
    start: i64,
    end: i64,
}

/// Get an index range of a string, end character will be included.
///
/// See: `redis.git/src/t_string.c:getrangeCommand(client *c)`
fn get_range(len: usize, start: i64, end: i64) -> Option<Range<usize>> {
    if len == 0 || start > len as i64 || (start < 0 && end < 0 && start > end) {
        return None;
    }
    let to_pos = |index: i64| -> i64 { if index < 0 { len as i64 + index } else { index } };
    let start = to_pos(start).clamp(0, len as i64 - 1) as usize;
    let end = to_pos(end).clamp(0, len as i64 - 1) as usize;
    if start <= end {
        Some(start..end + 1)
    } else {
        None
    }
}

#[async_trait]
impl CommandExecutor for GetRangeCommand {
    async fn execute(self, ctx: Arc<Context>) -> CommandResult {
        let db = ctx.db.clone();
        let db = db.read().await;
        log::debug!("Getting range for {}", &self.key);
        if let Some(o) = db.get(&self.key) {
            if &o.header.obj_type() != &ObjectType::String {
                log::error!("`GETRANGE` operation on {} value", &o.header.obj_type());
                return Err(CommandError::WrongType);
            }
            let bytes = match o.ptr {
                RedisValue::Int(i) => i.to_be_bytes().to_vec(),
                RedisValue::EmbStr(es) => es.into(),
                RedisValue::Raw(raw) => raw.into(),
                _ => Vec::new(),
            };
            if let Some(range) = get_range(bytes.len(), self.start, self.end) {
                Ok(RespValue::BulkString(Some(bytes[range].to_vec())))
            } else {
                Ok(RespValue::BulkString(None))
            }
        } else {
            Ok(RespValue::BulkString(None))
        }
    }
}

#[cfg(test)]
mod test {
    use crate::command::string::getrange::get_range;
    use std::ops::Range;

    #[test]
    fn test_get_range_on_start_is_positive_then_ok() {
        assert_eq!(get_range(4, 0, 0), Some(0..1));
        assert_eq!(get_range(4, 0, 1), Some(0..2));
        assert_eq!(get_range(4, 0, 4), Some(0..4));
        assert_eq!(get_range(4, 0, 5), Some(0..4));
        assert_eq!(get_range(4, 0, -1), Some(0..4));
        assert_eq!(get_range(4, 0, -2), Some(0..3));
        assert_eq!(get_range(4, 0, -5), Some(0..1));
    }

    #[test]
    fn test_get_range_on_start_is_negative_then_ok() {
        assert_eq!(get_range(4, -4, 0), Some(0..1));
        assert_eq!(get_range(4, -4, 1), Some(0..2));
        assert_eq!(get_range(4, -4, 4), Some(0..4));
        assert_eq!(get_range(4, -4, 5), Some(0..4));
        assert_eq!(get_range(4, -4, -1), Some(0..4));
        assert_eq!(get_range(4, -4, -2), Some(0..3));
    }

    #[test]
    fn test_get_range_on_start_is_very_small_then_ok() {
        assert_eq!(get_range(4, -5, 0), Some(0..1));
        assert_eq!(get_range(4, -5, 1), Some(0..2));
        assert_eq!(get_range(4, -5, 4), Some(0..4));
        assert_eq!(get_range(4, -5, 5), Some(0..4));
        assert_eq!(get_range(4, -5, -1), Some(0..4));
        assert_eq!(get_range(4, -5, -2), Some(0..3));
        assert_eq!(get_range(4, -5, -5), Some(0..1));
    }

    #[test]
    fn test_get_range_of_bytes_on_index_out_of_range() {
        assert_eq!(get_range(4, 2, 1), None::<Range<usize>>);
        assert_eq!(get_range(4, 5, 6), None::<Range<usize>>);
        assert_eq!(get_range(4, -4, -5), None::<Range<usize>>);
    }
}
