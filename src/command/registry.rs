use std::{
    collections::HashMap,
    pin::Pin,
    sync::{Arc, Mutex},
};

use crate::{command::error::CommandError, context::Context, resp::RespValue};

use once_cell::sync::Lazy;
use tokio::sync::RwLock;

/// redis command handler
pub type CommandHandler = fn(ctx: Arc<Context>, Vec<RespValue>) -> CommandFuture;

pub type RegisterFuture = Pin<Box<dyn Future<Output = ()> + Send>>;

/// redis command handler result
pub type CommandFuture = Pin<Box<dyn Future<Output = Result<RespValue, CommandError>> + Send>>;

/// global redis command registry
pub static COMMAND_REGISTRY: Lazy<RwLock<HashMap<&'static str, CommandHandler>>> =
    Lazy::new(|| RwLock::new(HashMap::new()));

pub static PENDING_REGISTRATIONS: Lazy<Mutex<Vec<RegisterFuture>>> =
    Lazy::new(|| Mutex::new(Vec::new()));

/// register a redis command to the registry
pub async fn register_command(name: &'static str, handler: CommandHandler) {
    let mut map = COMMAND_REGISTRY.write().await;
    map.insert(name, handler);
}

pub fn en_register_queue(fut: RegisterFuture) {
    let mut pending = PENDING_REGISTRATIONS.lock().unwrap();
    pending.push(fut);
    log::debug!("Pushed register future.")
}

/// Pool all registration futures, register all redis commands to COMMAND_REGISTRY
pub async fn do_register() {
    let futures = {
        let mut locked = PENDING_REGISTRATIONS.lock().unwrap();
        std::mem::take(&mut *locked)
    };
    for f in futures {
        f.await;
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
                $crate::command::registry::en_register_queue(Box::pin(async {
                    $crate::command::registry::register_command($cmd_name, $handler).await;
                }));
            }
        }
    };
}
