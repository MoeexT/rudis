use crate::{
    command::{error::CommandError, CommandExecutor}, context::Context, resp::RespValue, storage::Object
};
use anyhow::Result;
use async_trait::async_trait;

///
/// redis string
///
#[derive(Debug)]
pub enum StringCommand {
    Get(String),
    Set(String, String),
}

impl TryFrom<(String, Vec<String>)> for StringCommand {
    type Error = CommandError;

    fn try_from((cmd, mut args): (String, Vec<String>)) -> Result<Self, Self::Error> {
        match cmd.as_str() {
            "GET" if args.len() == 1 => Ok(Self::Get(args.remove(0))),
            "SET" if args.len() == 2 => Ok(Self::Set(args.remove(0), args.remove(0))),
            _ => Err(CommandError::InvalidArguments(cmd.into(), args.to_owned())),
        }
    }
}


#[async_trait]
impl CommandExecutor for StringCommand {
    async fn execute(self, ctx: &Context) -> Result<RespValue> {
        match self {
            StringCommand::Get(key) => {
                let db = ctx.db.clone();
                let db = db.read().await;
                if let Some(val) = db.get(&key) {
                    log::info!("[string] ctx {} get `{}` `{:?}`", ctx.id, &key, &val);
                    match val {
                        Object::String(val) => Ok(RespValue::SimpleString(String::from(val))),
                        Object::List => Err(CommandError::InvalidCommand(String::from("WRONGTYPE Operation against a key holding the wrong kind of value")).into()),
                    }
                } else {
                    log::info!("[string] ctx {} get `{}` `null`", ctx.id, &key);
                    Ok(RespValue::Null)
                }
            },
            StringCommand::Set(key, value) => {
                let db = ctx.db.clone();
                let db = db.write().await;
                log::info!("[string] ctx {} set {{{}: {}}}", ctx.id, &key, &value);
                db.set(key, Object::String(value), None);
                log::debug!("value set");
                Ok(RespValue::Boolean(true))
            },
        }
    }
}
