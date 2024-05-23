use std::io::prelude::*;
use std::net::TcpListener;
use std::net::TcpStream;

use in_memory_cache::InMemoryCache;

mod in_memory_cache;

fn main() {
    let listener = TcpListener::bind("127.0.0.1:3000").unwrap();

    for stream in listener.incoming() {
        let stream = stream.unwrap();
        handle_connection(stream);
    }
}

fn handle_connection(mut stream: TcpStream) {
    let mut buffer = [0; 1024];
    stream.read_exact(&mut buffer).unwrap();

    let get_request_line = buffer
        .split(|&b| b == b'\r' || b == b'\n')
        .find(|&line| line.starts_with(b"GET"));

    if let Some(get_request_line) = get_request_line {
        if let Ok(get_request_str) = std::str::from_utf8(get_request_line) {
            let path = get_request_str.split_whitespace().nth(1).unwrap_or("/");

            let mut cache = InMemoryCache::new();

            let value = cache.resolve(path, format!("Unicorn {path}").as_str(), 10);
            let response = format!("HTTP/1.1 200 OK\r\n\r\n{}", value.unwrap());
            stream.write_all(response.as_bytes()).unwrap();
            stream.flush().unwrap();
        }
    }
}
