use std::{
    collections::HashMap,
    pin::Pin,
    sync::{Arc, Mutex},
};

use once_cell::sync::Lazy;
use tokio::sync::RwLock;

use crate::{command::error::CommandError, context::Context, resp::RespValue};

/// redis command handler
pub type CommandHandler = fn(ctx: Arc<Context>, Vec<RespValue>) -> CommandFuture;

/// redis command handler result
pub type CommandFuture = Pin<Box<dyn Future<Output = Result<RespValue, CommandError>> + Send>>;

/// global redis command registry
pub static COMMAND_REGISTRY: Lazy<RwLock<HashMap<String, CommandHandler>>> =
    Lazy::new(|| RwLock::new(HashMap::new()));

pub static PENDING_REGISTRATIONS: Lazy<Mutex<Vec<(String, CommandHandler)>>> =
    Lazy::new(|| Mutex::new(Vec::new()));

pub fn en_register_queue(cmd: &str, handler: CommandHandler) {
    let mut pending = PENDING_REGISTRATIONS.lock().unwrap();
    pending.push((cmd.to_ascii_uppercase(), handler));
    log::debug!("Pushed handler to register queue.")
}

/// Register all redis commands to COMMAND_REGISTRY
pub async fn do_register() {
    let futures = {
        let mut locked = PENDING_REGISTRATIONS.lock().unwrap();
        std::mem::take(&mut *locked)
    };
    let mut map = COMMAND_REGISTRY.write().await;
    for (cmd, handler) in futures {
        map.insert(cmd, handler);
    }
    log::debug!("All redis commands are registered.")
}

/// Automatically register the redis command handler during the module init phrase
#[macro_export]
macro_rules! register_redis_command {
    ($cmd_name:literal, $handler:path) => {
        ::paste::item! {
            #[ctor::ctor]
            fn [<__register_command_ $cmd_name:lower>]() {
                fn wrapper(ctx: Arc<Context>, args: Vec<RespValue>) -> CommandFuture {
                Box::pin($handler(ctx, args))
                }
                $crate::command::registry::en_register_queue($cmd_name, wrapper);
            }
        }
    };
}
