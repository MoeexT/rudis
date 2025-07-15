use std::sync::Arc;

use async_trait::async_trait;

use crate::{
    command::{error::CommandError, registry::CommandFuture, CommandExecutor},
    context::Context,
    register_redis_command,
    resp::RespValue,
};

#[derive(Debug, PartialEq, Eq)]
pub struct GetCommand {
    pub key: String,
}

impl TryFrom<Vec<RespValue>> for GetCommand {
    type Error = CommandError;

    fn try_from(mut value: Vec<RespValue>) -> Result<Self, CommandError> {
        if value.len() != 1 {
            return Err(CommandError::InvalidArguments(
                "get".to_string(),
                value.into_iter().map(|v| v.into()).collect(),
            ));
        }

        let k = value.pop().unwrap();
        match k {
            RespValue::BulkString(Some(k)) => Ok(GetCommand {
                key: String::from_utf8(k)?,
            }),
            _ => Err(CommandError::InvalidArguments("set".to_string(), vec![])),
        }
    }
}

#[async_trait]
impl CommandExecutor for GetCommand {
    async fn execute(self, ctx: Arc<Context>) -> Result<RespValue, CommandError> {
        let db = ctx.db.clone();
        let db = db.write().await;
        log::debug!(
            "[string] ctx {} get {}",
            ctx.id,
            &self.key
        );
        if let Some(o) = db.get(&self.key) {
            log::debug!("value get: {}", &self.key);
            Ok(o.into())
        } else {
            Ok(RespValue::Null)
        }
    }
}

pub fn get_command(ctx: Arc<Context>, args: Vec<RespValue>) -> CommandFuture {
    Box::pin(async move {
        let cmd: GetCommand = args.try_into()?;
        cmd.execute(ctx).await
    })
}

register_redis_command!("GET", get_command);

mod test {
    #[cfg(test)]
    use crate::{command::string::get::GetCommand, resp::RespValue};

    #[test]
    fn test_try_from_resp_to_set_ok() {
        let value = vec![
            RespValue::BulkString(Some("key".as_bytes().to_vec())),
        ];
        let result: GetCommand = value.try_into().unwrap();
        assert_eq!(
            result,
            GetCommand {
                key: "key".to_string(),
            }
        )
    }
}
