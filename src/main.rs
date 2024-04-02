use std::{
    io::Write,
    net::{Ipv4Addr, SocketAddr, SocketAddrV4, TcpListener, TcpStream},
};

fn handle_ping(stream: &mut TcpStream) {
    stream.write_all(b"+PONG\r\n").unwrap();
}

fn main() {
    // You can use print statements as follows for debugging, they'll be visible when running tests.
    println!("Logs from your program will appear here!");
    let addr = SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), 6379));
    let listener = TcpListener::bind(addr).unwrap();
    for streamr in listener.incoming() {
        match streamr {
            Ok(mut stream) => {
                println!("New connection!");
                handle_ping(&mut stream);
            }
            Err(e) => println!("Error: {}", e),
        }
    }
}
