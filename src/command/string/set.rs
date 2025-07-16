use std::sync::Arc;

use async_trait::async_trait;

use crate::{
    command::{error::CommandError, registry::CommandFuture, CommandExecutor},
    context::Context,
    register_redis_command,
    resp::RespValue, storage::object::redis_object::RedisObject,
};

#[derive(Debug, PartialEq, Eq)]
pub struct SetCommand {
    pub key: String,
    pub value: Vec<u8>,
}

impl TryFrom<Vec<RespValue>> for SetCommand {
    type Error = CommandError;

    fn try_from(mut value: Vec<RespValue>) -> Result<Self, CommandError> {
        if value.len() != 2 {
            return Err(CommandError::InvalidArguments(
                "set".to_string(),
                value.into_iter().map(|v| v.into()).collect(),
            ));
        }

        let (v, k) = (value.pop().unwrap(), value.pop().unwrap());
        match (k, v) {
            (RespValue::BulkString(Some(k)), RespValue::BulkString(Some(v))) => Ok(SetCommand {
                key: String::from_utf8(k)?,
                value: v,
            }),
            _ => Err(CommandError::InvalidArguments("set".to_string(), vec![])),
        }
    }
}

#[async_trait]
impl CommandExecutor for SetCommand {
    async fn execute(self, ctx: Arc<Context>) -> Result<RespValue, CommandError> {
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

pub async fn set_command(ctx: Arc<Context>, args: Vec<RespValue>) -> Result<RespValue, CommandError> {
    let cmd: SetCommand = args.try_into()?;
    cmd.execute(ctx).await
}

register_redis_command!("SET", set_command);

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
