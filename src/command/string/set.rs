use std::sync::Arc;

use async_trait::async_trait;
use rudis_macros::register;

use crate::object::redis_object::RedisObject;
use crate::{
    command::{CommandExecutor, registry::CommandResult},
    context::Context,
    resp::RespValue,
};

#[derive(PartialEq, Eq)]
#[register("SET")]
struct SetCommand {
    key: String,
    value: Vec<u8>,
}

#[async_trait]
impl CommandExecutor for SetCommand {
    async fn execute(self, ctx: Arc<Context>) -> CommandResult {
        let db = ctx.db.clone();
        let db = db.write().await;
        log::debug!(
            "[string] ctx {} set {{{}: {:?}}}",
            ctx.id,
            &self.key,
            &self.value[..self.value.len().min(16)]
        );
        db.set(self.key, RedisObject::new_string(self.value), None);
        log::debug!("value set");
        Ok(RespValue::Boolean(true))
    }
}

#[cfg(test)]
mod test {
    #[cfg(test)]
    use crate::{command::string::set::SetCommand, resp::RespValue};

    #[test]
    fn test_try_from_resp_to_set_ok() {
        let value = vec![
            RespValue::BulkString(Some("set".as_bytes().to_vec())),
            RespValue::BulkString(Some("key".as_bytes().to_vec())),
        ];
        let result: SetCommand = value.try_into().unwrap();
        assert_eq!(
            result,
            SetCommand {
                key: "set".to_string(),
                value: "key".as_bytes().to_vec(),
            }
        )
    }
}
