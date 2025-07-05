use crate::{
    command::{error::CommandError, list::ListCommand, string::StringCommand}, context::Context, resp::RespValue
};
use anyhow::Result;
use async_trait::async_trait;

pub mod error;
pub mod list;
pub mod string;

#[async_trait]
pub trait CommandExecutor {
    async fn execute(self, ctx: &Context) -> Result<RespValue>;
}

#[derive(Debug)]
pub enum Command {
    String(StringCommand),
    List(ListCommand),
}

/// Turns `RespValue` to `Command`
impl TryFrom<RespValue> for Command {
    type Error = CommandError;

    fn try_from(value: RespValue) -> Result<Self, Self::Error> {
        let args = match value {
            RespValue::Array(Some(arr)) => arr,
            _ => return Err(CommandError::InvalidCommand("Expectd RESP Array".into())),
        };

        let mut iter = args.into_iter();
        let name = match iter.next() {
            Some(RespValue::BulkString(Some(bytes))) => String::from_utf8(bytes)?,
            _ => return Err(CommandError::InvalidCommand("Missing command name".into())),
        };

        let name_upper = name.to_ascii_uppercase();
        let string_args: Vec<String> = iter
            .filter_map(|val| {
                if let RespValue::BulkString(Some(bytes)) = val {
                    String::from_utf8(bytes).ok()
                } else {
                    None
                }
            })
            .collect();

        match name_upper.as_str() {
            "GET" => Ok(Command::String(StringCommand::try_from((
                name_upper,
                string_args,
            ))?)),
            "SET" => Ok(Command::String(StringCommand::try_from((name_upper, string_args))?)),
            _ => Err(CommandError::InvalidCommand(name_upper.into())),
        }
    }
}

#[async_trait]
impl CommandExecutor for Command {
    async fn execute(self, ctx: &Context) -> Result<RespValue> {
        match self {
            Command::String(string) => string.execute(ctx).await,
            Command::List(list) => list.execute(ctx).await,
        }
    }
}
