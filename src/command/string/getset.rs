use std::sync::Arc;

use async_trait::async_trait;
use rudis_macros::command;

use crate::{
    command::{CommandExecutor, registry::CommandResult},
    context::Context,
    object::redis_object::RedisObject,
    protocol::Frame,
};

#[command("GETSET")]
struct GetSetCommand {
    key: String,
    value: Vec<u8>,
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
            Ok(Frame::Null)
        }
    }
}
