use std::sync::Arc;

use async_trait::async_trait;
use rudis_macros::Command;

use crate::{
    command::{CommandExecutor, registry::CommandResult},
    context::Context,
    protocol::Frame,
};

#[derive(PartialEq, Eq, Debug, Command)]
#[command]
struct Get {
    key: String,
}

#[async_trait]
impl CommandExecutor for Get {
    async fn execute(self, ctx: Arc<Context>) -> CommandResult {
        let db = ctx.db.clone();
        let db = db.read().await;
        log::debug!("[string] ctx {} get {}", ctx.id, &self.key);
        if let Some(o) = db.get(&self.key) {
            log::debug!("value get: {}", &self.key);
            Ok(o.into())
        } else {
            Ok(Frame::Null)
        }
    }
}

#[cfg(test)]
mod test {
    use crate::command::parser::Parser;
    #[cfg(test)]
    use crate::{command::string::get::Get, protocol::Frame};

    #[test]
    fn test_try_from_frame_to_set_ok() {
        let value = vec![Frame::BulkString(Some("key".as_bytes().to_vec()))];
        let value = Parser::new(Frame::Array(Some(value))).unwrap();
        let result: Get = value.try_into().unwrap();
        assert_eq!(
            result,
            Get {
                key: "key".to_string(),
            }
        )
    }
}
