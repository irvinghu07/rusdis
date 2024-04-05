use std::collections::HashMap;
use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};
use std::sync::Arc;

use tokio::io::BufReader;
use tokio::net::{TcpListener, TcpStream};

mod cmd;
mod resp;
use cmd::Command;
use resp::RespHandler;
use tokio::sync::Mutex;

type Cache = Arc<Mutex<HashMap<String, String>>>;


async fn handle_conn(cache: Cache, stream: TcpStream) -> Result<(), Box<dyn std::error::Error>> {
    let mut handler = RespHandler::new(BufReader::new(stream));
    println!("Starting read loop");
    loop {
        let resp = handler.decode().await?;
        match resp {
            Some(res) => {
                let cmd: Command = match res.try_into() {
                    Ok(cmd) => cmd,
                    Err(e) => {
                        eprintln!("Error: {:?}", e);
                        continue;
                    }
                };
                let response = cmd.execute(Arc::clone(&cache)).await;
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
    let cache : Cache = Arc::new(Mutex::new(HashMap::<String, String>::new()));
    loop {
        let (stream, _) = listener.accept().await?;
        let cache_clone = Arc::clone(&cache);
        tokio::spawn(async move {
            handle_conn(cache_clone, stream).await.unwrap();
        });
    }
}
