use std::{
    collections::HashMap,
    env,
    error::Error,
    sync::Arc,
    time::{Duration, Instant},
};

use anyhow::Context;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    sync::RwLock,
};

mod errors;
mod resp;

type Storage = Arc<RwLock<HashMap<String, ValueEntry>>>;
struct ValueEntry {
    value: String,
    expiry: Option<Instant>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    if env::var("RUST_LOG").is_err() {
        unsafe {
            env::set_var("RUST_LOG", "debug");
        }
    }
    env_logger::init();
    let storage: Storage = Arc::new(RwLock::new(HashMap::new()));

    let storage_cleaner = storage.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(5));
        loop {
            interval.tick().await;
            let mut storage = storage_cleaner.write().await;
            storage.retain(|key, entry|{
                entry.expiry.map(|expiry| {
                    log::debug!("Retain key: {key}");
                    return expiry > Instant::now();
                }).unwrap_or(true)
            });
        }
    });

    let listener = TcpListener::bind("0.0.0.0:6379").await.context("Bind 0.0.0.0:6379 failed")?;

    loop {
        let (socket, _) = listener.accept().await.context("Accept socket failed")?;
        let storage = storage.clone();
        tokio::spawn(async move { handle_socket(socket, storage).await });
    }
}

async fn handle_socket(mut socket: TcpStream, storage: Storage) {
    let mut buf = [0; 1024];
    let len = match socket.read(&mut buf).await {
        Ok(0) => return,
        Ok(n) => n,
        Err(e) => {
            log::error!("Failed to read from socket; err: {:?}", e);
            return;
        }
    };

    let commands = match resp::parse_resp(&buf[..len]) {
        Ok(cmds) => cmds,
        Err(e) => {
            let _ = socket.write_all(b"-ERR Invalid command\r\n").await;
            log::error!("Failed to parse command; err: {:?}", e);
            return;
        }
    };

    match commands[0].to_uppercase().as_str() {
        "PING" => {
            let _ = socket.write_all(b"+PONG\r\n").await;
        }
        "ECHO" if commands.len() > 1 => {
            let response = format!("+{:#?}\r\n", &commands[1..]);
            let _ = socket.write_all(response.as_bytes()).await;
        }
        "SET" if commands.len() >= 3 => {
            let mut storage = storage.write().await;
            let value = ValueEntry {
                value: commands[2].clone(),
                expiry: None,
            };
            storage.insert(commands[1].clone(), value);
            let _ = socket.write_all(b"+OK\r\n").await;
        }
        "GET" if commands.len() > 1 => {
            let storage = storage.read().await;
            match storage.get(&commands[1]) {
                Some(value) => {
                    let response = format!("+{}\r\n", value.value);
                    let _ = socket.write_all(response.as_bytes()).await;
                    log::debug!("Return {response}");
                }
                None => {
                    let _ = socket.write_all(b"$-1\r\n").await;
                    log::debug!("Return -1");
                }
            }
        }
        "DEL" if commands.len() > 1 => {
            let mut storage = storage.write().await;
            let count = storage.remove(&commands[1]).is_some() as i64;
            let response = format!(":{}\r\n", count);
            let _ = socket.write_all(response.as_bytes()).await;
        }
        "EXPIRE" if commands.len() >= 3 => {
            let key = &commands[1];
            let seconds: u64 = match commands[2].parse() {
                Ok(n) => n,
                Err(_) => {
                    let _ = socket.write_all(b"-ERR Invalid integer\r\n").await;
                    return;
                }
            };
            let mut storage = storage.write().await;
            if let Some(entry) = storage.get_mut(key) {
                entry.expiry = Some(Instant::now() + Duration::from_secs(seconds));
                let _ = socket.write_all(b":1\r\n").await;
            } else {
                let _ = socket.write_all(b":0\r\n").await;
            }
        }
        "TTL" if commands.len() > 1 => {
            let key = &commands[1];
            let storage = storage.read().await;
            match storage.get(key) {
                Some(ValueEntry {
                    expiry: Some(expiry),
                    ..
                }) => {
                    let remaining =
                        expiry.saturating_duration_since(Instant::now()).as_secs() as i64;
                    let response = format!(":{}\r\n", remaining);
                    let _ = socket.write_all(response.as_bytes()).await;
                }
                Some(ValueEntry { expiry: None, .. }) => {
                    // -1: permanent
                    let _ = socket.write_all(b":-1\r\n").await;
                }
                None => {
                    // -2: key not exist
                    let _ = socket.write_all(b":-2\r\n").await;
                }
            }
        }
        _ => {
            let _ = socket.write_all(b"-ERR Unknown command\r\n").await;
        }
    }
}

mod test {
    #[test]
    fn feature() {}
}
