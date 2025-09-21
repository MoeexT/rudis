use std::sync::Arc;

use async_trait::async_trait;
use rudis_macros::Command;

use crate::{
    command::{CommandExecutor, registry::CommandResult},
    protocol::Frame,
};

#[derive(PartialEq, Eq, Command, Debug)]
struct Ping;

#[async_trait]
impl CommandExecutor for Ping {
    async fn execute(self, _ctx: Arc<crate::context::Context>) -> CommandResult {
        Ok(Frame::SimpleString("PONG".to_string()))
    }
}
