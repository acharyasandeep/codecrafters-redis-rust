// Uncomment this block to pass the first stage
use std::{
    io::{BufReader, Read, Write},
    net::TcpListener,
};

fn main() {
    // You can use print statements as follows for debugging, they'll be visible when running tests.
    println!("Logs from your program will appear here!");

    // Uncomment this block to pass the first stage

    let listener = TcpListener::bind("127.0.0.1:6379").unwrap();

    for stream in listener.incoming() {
        match stream {
            Ok(mut _stream) => {
                println!("accepted new connection");
                let mut buf_reader = BufReader::new(&_stream);
                let mut buffer = vec![0; 100];
                // let mut req = String::new();
                // let _ = buf_reader.read_line(&mut req);
                let _ = buf_reader.read(&mut buffer);
                println!(
                    "Request is {:?}",
                    String::from_utf8_lossy(&buffer).to_string()
                );
                // println!("Request is {:?}", req);

                let _ = _stream.write_all(b"+PONG\r\n");
            }
            Err(e) => {
                println!("error: {}", e);
            }
        }
    }
}
