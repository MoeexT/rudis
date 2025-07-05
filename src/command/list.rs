use anyhow::Result;
use async_trait::async_trait;

use crate::{command::CommandExecutor, context::Context, resp::RespValue};

#[derive(Debug)]
pub enum ListCommand {
    LPUSH(String, Vec<String>),
    RPUSH(String, Vec<String>),
}



#[async_trait]
impl CommandExecutor for ListCommand {
    async fn execute(self, ctx: &Context) -> Result<RespValue> {
        todo!()
    }
}
