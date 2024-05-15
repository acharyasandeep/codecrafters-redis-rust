// Uncomment this block to pass the first stage
use std::{
    collections::HashMap,
    io::{BufRead, BufReader, Error, Write},
    net::{TcpListener, TcpStream},
    ptr::null,
    sync::{Arc, Mutex},
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

enum ResponseType {
    SimpleString,
    BulkString,
    NullBulkString,
}

impl ResponseType {
    fn as_str(&self) -> &'static str {
        match self {
            ResponseType::SimpleString => "+",
            ResponseType::BulkString => "$",
            ResponseType::NullBulkString => "$",
        }
    }
}

fn make_response(content: &String, content_type: ResponseType) -> String {
    let mut response = String::new();
    match content_type {
        ResponseType::BulkString => {
            let content_length = content.len();
            response = format!(
                "{}{}\r\n{}\r\n",
                content_type.as_str(),
                content_length,
                content
            );
        }
        ResponseType::NullBulkString => {
            response = format!("{}{}\r\n{}\r\n", content_type.as_str(), -1, content);
        }
        ResponseType::SimpleString => {
            response = format!("{}{}\r\n", content_type.as_str(), content);
        }
    }

    response
}

fn handle_connection_helper(
    stream: Result<TcpStream, Error>,
    redis_cache: &Arc<Mutex<HashMap<String, String>>>,
) {
    let thread_shared_redis_cache = Arc::clone(redis_cache);
    match stream {
        Ok(mut _stream) => {
            thread::spawn(move || handle_connection(_stream, thread_shared_redis_cache));
            // handle_connection(_stream);
        }
        Err(e) => {
            println!("error: {}", e);
        }
    }
}

fn handle_connection(
    mut stream: TcpStream,
    thread_shared_redis_cache: Arc<Mutex<HashMap<String, String>>>,
) {
    println!("accepted new connection");

    loop {
        let request = Request::new(&mut stream);
        if let Some(req) = request {
            println!("Request is {:?}", req);
            match req.parameters[0].as_str() {
                "ECHO" => {
                    let content = &req.parameters[1];
                    let response = make_response(content, ResponseType::BulkString);
                    let _ = stream.write(response.as_bytes());
                }
                "SET" => {
                    let mut map = thread_shared_redis_cache.lock().unwrap();
                    map.insert(req.parameters[1].clone(), req.parameters[2].clone());
                    let response = make_response(&String::from("OK"), ResponseType::SimpleString);
                    let _ = stream.write(response.as_bytes());
                }
                "GET" => {
                    let map = thread_shared_redis_cache.lock().unwrap();
                    let null_string = String::from("");
                    let content = map.get(&req.parameters[1]).unwrap_or_else(|| &null_string);
                    let response;
                    if content.as_str() == null_string.as_str() {
                        response = make_response(&null_string, ResponseType::NullBulkString);
                    } else {
                        response = make_response(&content, ResponseType::BulkString);
                    }
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
    let redis_cache: Arc<Mutex<HashMap<String, String>>> = Arc::new(Mutex::new(HashMap::new()));
    //let ARedisCache = Arc::new(RedisCache);

    for stream in listener.incoming() {
        handle_connection_helper(stream, &redis_cache);
        println!("redis cache outside {:?}", redis_cache.lock().unwrap())
    }
}
