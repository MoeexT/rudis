use std::sync::Arc;

use async_trait::async_trait;
use rudis_macros::Command;

use crate::command::option::Expiration;
use crate::object::redis_object::RedisObject;
use crate::{
    command::{CommandExecutor, registry::CommandResult},
    context::Context,
    protocol::Frame,
};

#[derive(PartialEq, Eq, Command, Debug)]
#[command("SET")]
struct SetCommand {
    key: String,
    value: Vec<u8>,

    #[arg(aliases = ["EX", "PX", "EXAT", "PXAT"])]
    expiration: Option<Expiration>,

    // boolean flags
    #[arg(flag = "NX")]
    nx: bool,

    #[arg(flag = "XX")]
    xx: bool,

    #[arg(flag = "KEEPTTL")]
    keepttl: bool,

    #[arg(flag = "GET")]
    get: bool,
}

#[async_trait]
impl CommandExecutor for SetCommand {
    async fn execute(self, ctx: Arc<Context>) -> CommandResult {
        log::debug!("{:?}", &self);
        let db = ctx.db.clone();
        let db = db.write().await;
        log::debug!(
            "[string] ctx {} set {{{}: {:?}}}",
            ctx.id,
            &self.key,
            &self.value[..self.value.len().min(16)]
        );
        db.set(self.key, RedisObject::new_string(self.value), None);
        log::debug!("value set");
        Ok(Frame::Boolean(true))
    }
}

#[cfg(test)]
mod test {
    use crate::command::parser::Parser;
    #[cfg(test)]
    use crate::{command::string::set::SetCommand, protocol::Frame};

    #[test]
    fn test_try_from_frame_to_set_ok() {
        let value = vec![
            Frame::BulkString(Some("set".as_bytes().to_vec())),
            Frame::BulkString(Some("key".as_bytes().to_vec())),
        ];
        let value = Parser::new(Frame::Array(Some(value))).unwrap();
        let result: SetCommand = value.try_into().unwrap();
        assert_eq!(
            result,
            SetCommand {
                key: "set".to_string(),
                value: "key".as_bytes().to_vec(),
                expiration: None,
                nx: false,
                xx: false,
                keepttl: false,
                get: false,
            }
        )
    }
}
