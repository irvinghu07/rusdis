use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};

use tokio::io::BufReader;
use tokio::net::{TcpListener, TcpStream};

mod resp;
use resp::{RespDT, RespParser};

async fn handle_conn(stream: TcpStream) -> Result<(), Box<dyn std::error::Error>> {
    let mut parser = RespParser::new(BufReader::new(stream));
    let resp: RespDT = parser.decode().await?;
    println!("Got resp: {:?}", resp);
    // println!("Starting read loop");
    // loop {
    //     let resp = parser.decode().await?;
    //     println!("Got resp: {:?}", resp);
    // }
    unimplemented!();
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
