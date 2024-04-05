use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};
use std::sync::Arc;

use tokio::io::BufReader;
use tokio::net::{TcpListener, TcpStream};

mod cmd;
mod resp;
mod store;
use cmd::Command;
use resp::RespHandler;
use store::Db;

use crate::cmd::command::RespCache;

async fn handle_conn(cache: Arc<Db>, stream: TcpStream) -> Result<(), Box<dyn std::error::Error>> {
    let mut handler = RespHandler::new(BufReader::new(stream));
    println!("Starting read loop");
    loop {
        let resp = handler.decode().await?;
        match resp {
            Some(res) => {
                let rc = RespCache::new(cache.clone(), res);
                let cmd: Command = match rc.try_into() {
                    Ok(cmd) => cmd,
                    Err(e) => {
                        eprintln!("Error: {:?}", e);
                        continue;
                    }
                };
                let response = cmd.execute().await?;
                handler.write(response).await?;
            }
            None => return Ok(()),
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // You can use print statements as follows for debugging, they'll be visible when running tests.
    println!("Logs from your program will appear here!");
    let addr = SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), 6379));
    let listener = TcpListener::bind(addr).await?;
    println!("Listening on {}:{}", addr.ip(), addr.port());
    let cache = Arc::new(Db::new());
    loop {
        let cache_clone = Arc::clone(&cache);
        let (stream, _) = listener.accept().await?;
        tokio::spawn(async move {
            handle_conn(cache_clone, stream).await.unwrap();
        });
    }
}
