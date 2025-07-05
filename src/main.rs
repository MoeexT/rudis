use anyhow::{Context, Result};
use tokio::io::AsyncWriteExt;
use std::convert::TryInto;
use std::{env, sync::Arc};
use tokio::{
    io::{BufReader, BufWriter},
    net::{TcpListener, TcpStream},
    sync::RwLock,
};

use crate::{
    command::{Command, CommandExecutor},
    storage::Database,
};

mod command;
mod context;
mod errors;
mod resp;
mod storage;

#[tokio::main]
async fn main() -> Result<()> {
    if env::var("RUST_LOG").is_err() {
        unsafe {
            env::set_var("RUST_LOG", "trace");
        }
    }
    env_logger::init();
    let db = Arc::new(RwLock::new(Database::new(0)));
    log::debug!("database created");
    let address = "0.0.0.0:6379";
    let listener = TcpListener::bind(&address)
        .await
        .context(format!("Bind {} failed", &address))?;
    log::info!("redis is listening on {}", address);

    loop {
        let (socket, addr) = listener.accept().await.context("Accept socket failed")?;
        let client_id = 0usize;
        let db = db.clone();
        let ctx = context::Context::new(client_id, db);
        log::debug!("received connection from: {}, id: {}", &addr, client_id);
        tokio::spawn(async move { handle_socket(socket, ctx).await });
    }
}

async fn handle_socket(socket: TcpStream, ctx: context::Context) -> Result<()> {
    let (reader, writer) = socket.into_split();
    let mut reader = BufReader::new(reader);
    let mut writer = BufWriter::new(writer);

    loop {
        let cmd: Command = resp::parse_resp(&mut reader).await?.try_into()?;
        let result = cmd.execute(&ctx).await?;
        log::debug!("ctx {} execute result: {:?}", ctx.id, &result);
        result.write_to(&mut writer).await?;
        writer.flush().await?;
    }
}

mod test {
    #[test]
    fn feature() {}
}
