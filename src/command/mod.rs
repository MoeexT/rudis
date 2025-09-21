use std::{fmt::Debug, sync::Arc};

use async_trait::async_trait;

use crate::{
    command::{
        error::CommandError,
        parser::Parser,
        registry::{COMMAND_REGISTRY, CommandHandler},
    },
    context::Context,
    protocol::Frame,
};

pub mod connection;
pub mod error;
pub mod parser;
pub mod registry;
pub mod string;
mod option;

#[async_trait]
pub trait CommandExecutor: Send + Sync {
    async fn execute(self, ctx: Arc<Context>) -> Result<Frame, CommandError>;
}

#[derive(Debug)]
pub struct Command {
    handler: CommandHandler,
    parser: Parser,
}

impl Command {
    pub async fn parse(value: Frame) -> Result<Command, CommandError> {
        let mut parser = Parser::new(value)?;
        let name_upper = parser.next::<String>()?.to_ascii_uppercase();
        let reg = COMMAND_REGISTRY.read().await;
        if let Some(&handler) = reg.get(name_upper.as_str()) {
            Ok(Command { handler, parser })
        } else {
            Err(CommandError::InvalidCommand(name_upper.into()))
        }
    }
}

#[async_trait]
impl CommandExecutor for Command {
    async fn execute(self, ctx: Arc<Context>) -> Result<Frame, CommandError> {
        match (self.handler)(ctx, self.parser).await {
            Ok(result) => Ok(result),
            Err(error) => Ok(Frame::Error(error.to_string())),
        }
    }
}
