use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};

use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
};

const PONG: &[u8] = b"+PONG\r\n";

async fn handle_ping(
    buffer: &mut [u8; 1 << 10],
    stream: &mut TcpStream,
) -> Result<(), Box<dyn std::error::Error>> {
    loop {
        match stream.read(buffer).await {
            Ok(0) => return Ok(()),
            Ok(n) => {
                println!("Recieved {} bytes", n);
                stream.write_all(PONG).await?;
                stream.flush().await?;
            }
            Err(e) => eprintln!("Error reading from socket: {}", e),
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
    let mut buffer = [0; 1 << 10];
    loop {
        let (mut stream, _) = listener.accept().await?;
        tokio::spawn(async move {
            handle_ping(&mut buffer, &mut stream).await.unwrap();
        });
    }
}
