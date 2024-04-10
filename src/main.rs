use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};
use std::sync::Arc;

use tokio::io::BufReader;
use tokio::net::{TcpListener, TcpStream};

mod cli;
mod cmd;
mod resp;
mod store;
use cli::CliArgs;
use cmd::Command;
use resp::RespHandler;
use store::Db;

use crate::cmd::command::RespCache;
use clap::Parser;

async fn handle_conn(cache: Arc<Db>, stream: TcpStream) -> Result<(), Box<dyn std::error::Error>> {
    let mut handler = RespHandler::new(BufReader::new(stream));
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
    let args = CliArgs::parse();
    let addr = SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), args.port));
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
