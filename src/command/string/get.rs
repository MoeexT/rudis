use std::sync::Arc;

use async_trait::async_trait;
use rudis_macros::redis_command;

use crate::{
    command::{CommandExecutor, error::CommandError, registry::CommandResult},
    context::Context,
    resp::RespValue,
};

#[derive(Debug, PartialEq, Eq)]
struct GetCommand {
    key: String,
}

impl TryFrom<Vec<RespValue>> for GetCommand {
    type Error = CommandError;

    fn try_from(values: Vec<RespValue>) -> Result<Self, CommandError> {
        let [key]: [RespValue; 1] = values
            .try_into()
            .map_err(|_| CommandError::InvalidArgumentNumber("get".to_string()))?;

        match key {
            RespValue::BulkString(Some(k)) => Ok(GetCommand {
                key: String::from_utf8(k)?,
            }),
            _ => Err(CommandError::InvalidCommandFormat("get".to_string())),
        }
    }
}

#[async_trait]
impl CommandExecutor for GetCommand {
    async fn execute(self, ctx: Arc<Context>) -> CommandResult {
        let db = ctx.db.clone();
        let db = db.read().await;
        log::debug!("[string] ctx {} get {}", ctx.id, &self.key);
        if let Some(o) = db.get(&self.key) {
            log::debug!("value get: {}", &self.key);
            Ok(o.into())
        } else {
            Ok(RespValue::Null)
        }
    }
}

#[redis_command("GET")]
pub async fn get_command(ctx: Arc<Context>, args: Vec<RespValue>) -> CommandResult {
    let cmd: GetCommand = args.try_into()?;
    cmd.execute(ctx).await
}

#[cfg(test)]
mod test {
    #[cfg(test)]
    use crate::{command::string::get::GetCommand, resp::RespValue};

    #[test]
    fn test_try_from_resp_to_set_ok() {
        let value = vec![RespValue::BulkString(Some("key".as_bytes().to_vec()))];
        let result: GetCommand = value.try_into().unwrap();
        assert_eq!(
            result,
            GetCommand {
                key: "key".to_string(),
            }
        )
    }
}
