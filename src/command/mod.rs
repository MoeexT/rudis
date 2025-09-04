use std::{fmt::Debug, sync::Arc};

use async_trait::async_trait;

use crate::{
    command::{
        error::CommandError,
        registry::{COMMAND_REGISTRY, CommandHandler},
    },
    context::Context,
    resp::RespValue,
};

pub mod error;
pub mod registry;
pub mod string;
pub mod connection;

#[async_trait]
pub trait CommandExecutor: Send + Sync + Debug {
    async fn execute(self, ctx: Arc<Context>) -> Result<RespValue, CommandError>;
}

#[derive(Debug)]
pub struct Command {
    handler: CommandHandler,
    args: Vec<RespValue>,
}

impl Command {
    pub async fn parse(value: RespValue) -> Result<Command, CommandError> {
        let args = match value {
            RespValue::Array(Some(arr)) => arr,
            _ => return Err(CommandError::InvalidCommand("Expected RESP Array".into())),
        };

        let mut iter = args.into_iter();
        let name = match iter.next() {
            Some(RespValue::BulkString(Some(bytes))) => String::from_utf8(bytes)?,
            _ => return Err(CommandError::InvalidCommand("Missing command name".into())),
        };

        let name_upper = name.to_ascii_uppercase();
        let string_args: Vec<RespValue> = iter.collect();
        let reg = COMMAND_REGISTRY.read().await;
        if let Some(&handler) = reg.get(name_upper.as_str()) {
            Ok(Command {
                handler,
                args: string_args,
            })
        } else {
            Err(CommandError::InvalidCommand(name_upper.into()))
        }
    }
}

#[async_trait]
impl CommandExecutor for Command {
    async fn execute(self, ctx: Arc<Context>) -> Result<RespValue, CommandError> {
        match (self.handler)(ctx, self.args).await {
            Ok(result) => Ok(result),
            Err(error) => Ok(RespValue::Error(error.to_string())),
        }
    }
}
