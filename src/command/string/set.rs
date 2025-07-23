use std::sync::Arc;

use async_trait::async_trait;

use crate::object::redis_object::RedisObject;
use crate::{
    command::{CommandExecutor, error::CommandError, registry::CommandResult},
    config::get_server_config,
    context::Context,
    register_redis_command,
    resp::RespValue,
};

#[derive(Debug, PartialEq, Eq)]
struct SetCommand {
    key: String,
    value: Vec<u8>,
}

impl TryFrom<Vec<RespValue>> for SetCommand {
    type Error = CommandError;

    fn try_from(values: Vec<RespValue>) -> Result<Self, CommandError> {
        let [key, value]: [RespValue; 2] = values
            .try_into()
            .map_err(|_| CommandError::InvalidArgumentNumber("set".to_string()))?;

        match (key, value) {
            (RespValue::BulkString(Some(k)), RespValue::BulkString(Some(v))) => {
                let config = get_server_config();
                if v.len() > config.string_max_length {
                    return Err(CommandError::SuperHugeString(
                        v.len(),
                        String::from_utf8_lossy(&k).into_owned(),
                    ));
                }
                Ok(SetCommand {
                    key: String::from_utf8(k)?,
                    value: v,
                })
            }
            _ => Err(CommandError::InvalidCommandFormat("set".to_string())),
        }
    }
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

pub async fn set_command(ctx: Arc<Context>, args: Vec<RespValue>) -> CommandResult {
    let cmd: SetCommand = args.try_into()?;
    cmd.execute(ctx).await
}

register_redis_command!("SET", set_command);

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
