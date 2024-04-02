use std::{
    io::{Read, Write},
    net::{Ipv4Addr, SocketAddr, SocketAddrV4, TcpListener, TcpStream},
};

fn handle_ping(stream: &mut TcpStream, buffer: &mut [u8; 1 << 10]) {
    loop {
        let sz: usize = stream.read(buffer).unwrap();
        if sz == 0 {
            println!("Client disconnected");
            return;
        }
        println!("Recieved {} bytes", sz);
        stream.write_all(b"+PONG\r\n").unwrap();
        stream.flush().unwrap();
    }
}

fn main() {
    // You can use print statements as follows for debugging, they'll be visible when running tests.
    println!("Logs from your program will appear here!");
    let addr = SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), 6379));
    let listener = TcpListener::bind(addr).unwrap();
    println!("Listening on {}:{}", addr.ip(), addr.port());
    let mut read_buffer = [0; 1 << 10];
    for streamr in listener.incoming() {
        match streamr {
            Ok(mut stream) => {
                println!("New connection!");
                handle_ping(&mut stream, &mut read_buffer);
            }
            Err(e) => println!("Error: {}", e),
        }
    }
}
