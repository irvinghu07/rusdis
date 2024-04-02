use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4, TcpListener};

fn main() {
    // You can use print statements as follows for debugging, they'll be visible when running tests.
    println!("Logs from your program will appear here!");
    let addr = SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), 6379));
    let listener = TcpListener::bind(addr).unwrap();
    for stream in listener.incoming() {
        match stream {
            Ok(_stream) => println!("New connection!"),
            Err(e) => println!("Error: {}", e),
        }
    }
}
