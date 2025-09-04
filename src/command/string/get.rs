use std::sync::Arc;

use async_trait::async_trait;
use rudis_macros::register;

use crate::{
    command::{CommandExecutor, registry::CommandResult},
    context::Context,
    resp::RespValue,
};

#[derive(PartialEq, Eq)]
#[register("GET")]
struct GetCommand {
    key: String,
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
