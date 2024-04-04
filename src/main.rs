use std::collections::HashMap;
use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};
use std::sync::Arc;

use tokio::io::BufReader;
use tokio::net::{TcpListener, TcpStream};

mod resp;
use resp::{RespDT, RespHandler};
use tokio::sync::Mutex;

pub trait CmdHandler {
    fn handle_cmd(&mut self, cmd: &str, args: Vec<&str>) -> RespDT;
}

type Cache = Arc<Mutex<HashMap<String, String>>>;

async fn cache_set(cache: Cache, key: String, val: String) {
    cache.lock().await.insert(key, val);
}

async fn cache_get(cache: Cache, key: String) -> Option<String> {
    cache.lock().await.get(&key).cloned()
}

async fn handle_conn(cache: Cache, stream: TcpStream) -> Result<(), Box<dyn std::error::Error>> {
    let mut handler = RespHandler::new(BufReader::new(stream));
    println!("Starting read loop");
    loop {
        let resp = handler.decode().await?;
        match resp {
            Some(res) => {
                let (cmd, args) = res.extract_resp().unwrap();
                let response = match cmd.to_lowercase().as_str() {
                    "ping" => RespDT::SimpleString("PONG".to_string()),
                    "echo" => {
                        RespDT::SimpleString(args.first().unwrap().extrac_bulk_str().unwrap())
                    }
                    "set" => {
                        if args.len() == 2 {
                            let key = args.first().unwrap().extrac_bulk_str().unwrap();
                            let val = args.last().unwrap().extrac_bulk_str().unwrap();
                            cache_set(cache.clone(), key, val).await;
                            RespDT::SimpleString("OK".to_string())
                        } else {
                            return Err("Invalid number of arguments".into());
                        }
                    }
                    "get" => {
                        if args.len() == 1 {
                            let key = args.first().unwrap().extrac_bulk_str().unwrap();
                            match cache_get(cache.clone(), key).await {
                                Some(val) => RespDT::SimpleString(val.to_string()),
                                None => RespDT::Null,
                            }
                        } else {
                            return Err("Invalid number of arguments".into());
                        }
                    }
                    c => RespDT::SimpleString(format!("Cannot Handle command:{}", c)),
                };
                handler.write(response.encode_raw()).await?;
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
    let cache = Arc::new(Mutex::new(HashMap::<String, String>::new()));
    loop {
        let cache_clone = Arc::clone(&cache);
        let (stream, _) = listener.accept().await?;
        tokio::spawn(async move {
            handle_conn(cache_clone, stream).await.unwrap();
        });
    }
}
