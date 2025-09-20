use std::sync::Arc;

use async_trait::async_trait;
use rudis_macros::command;

use crate::{
    command::{CommandExecutor, registry::CommandResult},
    protocol::Frame,
};

#[command("PING")]
struct PingCommand;

#[async_trait]
impl CommandExecutor for PingCommand {
    async fn execute(self, _ctx: Arc<crate::context::Context>) -> CommandResult {
        Ok(Frame::SimpleString("PONG".to_string()))
    }
}
