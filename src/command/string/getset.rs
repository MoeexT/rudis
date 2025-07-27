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

#[cfg(test)]
mod test {
    use std::sync::Arc;

    use mockall::mock;
    use tokio::sync::RwLock;

    use crate::{context::Context, object::redis_object::RedisObject};

    mock! {
        pub Database {
            async fn get(&self, key: &str) -> Option<RedisObject>;
            async fn set(&self, key: String, value: RedisObject, ttl: Option<std::time::Duration>);
        }
    }

    #[tokio::test]
    async fn test_execute_getset_command_on_origin_value_not_exist() {
        let mut mock_db = MockDatabase::new();
        let db= Arc::new(RwLock::new(mock_db));
        let ctx = Arc::new(Context::new(0, db));

        let key = "key".to_string();
        let old_value = RedisObject::new_string("old string".as_bytes().to_vec());
        let new_value = RedisObject::new_string("new string".as_bytes().to_vec());
    }
}
