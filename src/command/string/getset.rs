use std::sync::Arc;

use async_trait::async_trait;

use crate::{
    command::{CommandExecutor, error::CommandError, registry::CommandResult},
    config::get_server_config,
    context::Context,
    object::redis_object::RedisObject,
    register_redis_command,
    resp::RespValue,
};

#[derive(Debug)]
struct GetSetCommand {
    key: String,
    value: Vec<u8>,
}

impl TryFrom<Vec<RespValue>> for GetSetCommand {
    type Error = CommandError;

    fn try_from(values: Vec<RespValue>) -> Result<Self, Self::Error> {
        let [key, value]: [RespValue; 2] = values
            .try_into()
            .map_err(|_| CommandError::InvalidArgumentNumber("getset".to_string()))?;

        match (key, value) {
            (RespValue::BulkString(Some(key)), RespValue::BulkString(Some(value))) => {
                let config = get_server_config();
                if value.len() > config.string_max_length {
                    return Err(CommandError::SuperHugeString(
                        value.len(),
                        "getset".to_string(),
                    ));
                }
                Ok(GetSetCommand {
                    key: String::from_utf8(key)?,
                    value,
                })
            }
            _ => Err(CommandError::InvalidArgumentFormat("getset".to_string())),
        }
    }
}

#[async_trait]
impl CommandExecutor for GetSetCommand {
    async fn execute(self, ctx: Arc<Context>) -> CommandResult {
        let db = ctx.db.clone();
        let db = db.write().await;
        if let Some(origin_val) = db.get(&self.key) {
            db.set(self.key, RedisObject::new_string(self.value), None);
            Ok(origin_val.into())
        } else {
            db.set(self.key, RedisObject::new_string(self.value), None);
            Ok(RespValue::Null)
        }
    }
}

pub async fn getset_command(ctx: Arc<Context>, args: Vec<RespValue>) -> CommandResult {
    let cmd: GetSetCommand = args.try_into()?;
    cmd.execute(ctx).await
}

register_redis_command!("GETSET", getset_command);
