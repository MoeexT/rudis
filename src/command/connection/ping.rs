use std::sync::Arc;

use async_trait::async_trait;
use rudis_macros::register;

use crate::{
    command::{CommandExecutor, registry::CommandResult},
    resp::RespValue,
};

#[register("PING")]
struct PingCommand;

#[async_trait]
impl CommandExecutor for PingCommand {
    async fn execute(self, _ctx: Arc<crate::context::Context>) -> CommandResult {
        Ok(RespValue::SimpleString("PONG".to_string()))
    }
}
