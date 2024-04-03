use std::{
    io::BufReader,
    net::{Ipv4Addr, SocketAddr, SocketAddrV4},
};

use resp::resp_parser::{encode_raw, RespParser};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
};

use crate::resp::RespDT;

mod resp;

async fn handle_ping(
    buffer: &mut [u8; 1 << 10],
    stream: &mut TcpStream,
) -> Result<(), Box<dyn std::error::Error>> {
    loop {
        match stream.read(buffer).await {
            Ok(0) => return Ok(()),
            Ok(n) => {
                let mut parser = RespParser::new(BufReader::new(&buffer[..n]));
                let a = parser.decode()?;
                match a {
                    RespDT::Array(a) => {
                        if a.len() == 1 {
                            let inline_cmd = a.get(0).unwrap();
                            match inline_cmd {
                                RespDT::Bulk(cmd) => {
                                    if cmd.to_lowercase().eq("ping") {
                                        let pong = encode_raw(&resp::RespDT::SimpleString(
                                            "PONG".to_string(),
                                        ));
                                        stream.write_all(&pong).await?;
                                        stream.flush().await?;
                                    } else {
                                        eprintln!("Error input: {:?}", a);
                                        return Ok(());
                                    }
                                }
                                _ => {
                                    eprintln!("Error input: {:?}", a);
                                    return Ok(());
                                }
                            }
                        } else if a.len() == 2 {
                            let responsive_cmd = a.get(0).unwrap();
                            let callback = a.get(1).unwrap();
                            match responsive_cmd {
                                RespDT::Bulk(cmd) => {
                                    if cmd.to_lowercase().eq("echo") {
                                        match callback {
                                            RespDT::Bulk(cb) => {
                                                let echo =
                                                    encode_raw(&resp::RespDT::Bulk(cb.clone()));
                                                stream.write_all(&echo).await?;
                                                stream.flush().await?;
                                            }
                                            _ => {
                                                eprintln!("Error input: {:?}", a);
                                                return Ok(());
                                            }
                                        }
                                    } else {
                                        eprintln!("Error input: {:?}", a);
                                        return Ok(());
                                    }
                                }
                                _ => {
                                    eprintln!("Error input: {:?}", a);
                                    return Ok(());
                                }
                            }
                        } else {
                            eprintln!("Error input: {:?}", a);
                            return Ok(());
                        }
                    }
                    _ => {
                        eprintln!("Error input: {:?}", a);
                        return Ok(());
                    }
                }
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
