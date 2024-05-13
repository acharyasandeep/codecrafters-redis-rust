// Uncomment this block to pass the first stage
use std::{
    io::{BufReader, Error, Read, Write},
    net::{TcpListener, TcpStream},
    thread,
};

fn handle_connection_helper(stream: Result<TcpStream, Error>) {
    match stream {
        Ok(mut _stream) => {
            // thread::spawn(|| handle_connection(_stream));
            handle_connection(_stream);
        }
        Err(e) => {
            println!("error: {}", e);
        }
    }
}

fn handle_connection(mut stream: TcpStream) {
    println!("accepted new connection");
    let mut buffer = vec![0; 1024];

    loop {
        let read_count = stream.read(&mut buffer).unwrap();
        if read_count == 0 {
            break;
        }
        let _ = stream.write(b"+PONG\r\n");
    }
    // println!("Request is {:?}", req);

    // let _ = stream.write(b"+PONG\r\n");
}

fn main() {
    // You can use print statements as follows for debugging, they'll be visible when running tests.
    println!("Logs from your program will appear here!");

    // Uncomment this block to pass the first stage

    let listener = TcpListener::bind("127.0.0.1:6379").unwrap();

    for stream in listener.incoming() {
        handle_connection_helper(stream);
    }
}
