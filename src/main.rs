// Uncomment this block to pass the first stage
use std::{
    io::{BufRead, BufReader, Error, Write},
    net::{TcpListener, TcpStream},
    thread,
};

#[derive(Debug)]
struct Request {
    parameter_count: i32,
    parameters: Vec<String>,
}

impl Request {
    fn new(mut stream: &mut TcpStream) -> Option<Request> {
        // let mut buffer = vec![0; 1024];
        // let read_count = stream.read(&mut buffer).unwrap();
        let mut buf_reader = BufReader::new(&mut stream);
        let mut first_line = String::new();
        let read_count = buf_reader.read_line(&mut first_line).unwrap_or_else(|_| 0);
        if read_count == 0 {
            return None;
        }

        let parameter_count: i32 = first_line.trim()[1..].parse().unwrap();

        let mut parameters = vec![];

        for _ in 0..parameter_count {
            let mut param_length_line = String::new();
            let _ = buf_reader.read_line(&mut param_length_line);
            let _: i32 = param_length_line.trim()[1..].parse().unwrap();
            let mut parameter = String::new();
            let _ = buf_reader.read_line(&mut parameter);
            parameters.push(parameter.trim().to_string());
        }
        Some(Request {
            parameter_count,
            parameters,
        })
    }
}

fn make_response(content: &String) -> String {
    let content_length = content.len();
    let response = format!("${}\r\n{}\r\n", content_length, content);
    response
}

fn handle_connection_helper(stream: Result<TcpStream, Error>) {
    match stream {
        Ok(mut _stream) => {
            thread::spawn(|| handle_connection(_stream));
            // handle_connection(_stream);
        }
        Err(e) => {
            println!("error: {}", e);
        }
    }
}

fn handle_connection(mut stream: TcpStream) {
    println!("accepted new connection");

    loop {
        let request = Request::new(&mut stream);
        if let Some(req) = request {
            println!("Request is {:?}", req);
            match req.parameters[0].as_str() {
                "ECHO" => {
                    let content = &req.parameters[1];
                    let response = make_response(content);
                    let _ = stream.write(response.as_bytes());
                }
                _ => {
                    let _ = stream.write(b"+PONG\r\n");
                }
            }
        } else {
            break;
        }
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
