use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};

use tokio::io::BufReader;
use tokio::net::{TcpListener, TcpStream};

mod resp;
use resp::{RespDT, RespHandler};

async fn handle_conn(stream: TcpStream) -> Result<(), Box<dyn std::error::Error>> {
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
    loop {
        let (stream, _) = listener.accept().await?;
        tokio::spawn(async move {
            handle_conn(stream).await.unwrap();
        });
    }
}
