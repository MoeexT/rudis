use std::{env, sync::Arc};

use anyhow::{Context, Result};
use tokio::{
    io::{AsyncWriteExt, BufReader, BufWriter},
    net::{TcpListener, TcpStream},
    sync::RwLock,
};

use rudis::{
    command::{Command, CommandExecutor, registry::do_register},
    config::init_config,
    context, resp,
    storage::database::Database,
};

#[tokio::main]
async fn main() -> Result<()> {
    init_config();
    if env::var("RUST_LOG").is_err() {
        unsafe {
            env::set_var("RUST_LOG", "trace");
        }
    }
    env_logger::init();

    // register redis commands
    do_register().await;

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
        let ctx = Arc::new(context::Context::new(client_id, db));
        log::debug!("received connection from: {}, id: {}", &addr, client_id);
        tokio::spawn(async move { handle_socket(socket, ctx).await });
    }
}

async fn handle_socket(socket: TcpStream, context: Arc<context::Context>) -> Result<()> {
    let (reader, writer) = socket.into_split();
    let mut reader = BufReader::new(reader);
    let mut writer = BufWriter::new(writer);

    loop {
        let ctx = context.clone();
        let cid = ctx.id;
        let resp = resp::parse_resp(&mut reader).await?;
        let command = match Command::parse(resp).await {
            Ok(cmd) => cmd,
            Err(e) => {
                log::error!("ctx {} parse command error: {:?}", cid, e);
                let err: resp::RespValue = resp::RespValue::Error(e.to_string());
                err.write_to(&mut writer).await?;
                writer.flush().await?;
                continue;
            }
        };
        let result = command.execute(ctx).await?;
        log::debug!("ctx {} execute result: {:?}", cid, &result);
        result.write_to(&mut writer).await?;
        writer.flush().await?;
    }
}
