// Uncomment this block to pass the first stage
use std::{
    collections::HashMap,
    io::{BufRead, BufReader, Error, Write},
    net::{TcpListener, TcpStream},
    sync::{Arc, Mutex},
    thread::{self},
    time::{SystemTime, UNIX_EPOCH},
};

#[derive(Debug)]
struct Request {
    parameter_count: i32,
    parameters: Vec<String>,
}
#[derive(Debug)]
struct Value {
    data: String,
    created_at: Option<u128>,
    expiry: Option<u64>,
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
    let response;
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
    redis_cache: &Arc<Mutex<HashMap<String, Value>>>,
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
    thread_shared_redis_cache: Arc<Mutex<HashMap<String, Value>>>,
) {
    println!("accepted new connection");

    loop {
        let request = Request::new(&mut stream);
        if let Some(req) = request {
            println!("Request is {:?}", req);
            match req.parameters[0].to_lowercase().as_str() {
                "echo" => {
                    let content = &req.parameters[1];
                    let response = make_response(content, ResponseType::BulkString);
                    let _ = stream.write(response.as_bytes());
                }
                "set" => {
                    let mut map = thread_shared_redis_cache.lock().unwrap();
                    let current_time = SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap()
                        .as_millis();
                    let mut val = Value {
                        data: req.parameters[2].clone(),
                        created_at: Some(current_time),
                        expiry: None,
                    };

                    if req.parameter_count > 3 {
                        let command = req.parameters[3].to_lowercase();
                        println!("command is:{}", { command.clone() });
                        match command.as_str() {
                            "px" => {
                                let ms: u64 = req.parameters[4].parse().unwrap_or_else(|_| 0);
                                val.expiry = Some(ms);
                                println!("ms is {}", ms);
                            }
                            _ => {}
                        }
                    }

                    map.insert(req.parameters[1].clone(), val);
                    let response = make_response(&String::from("OK"), ResponseType::SimpleString);
                    let _ = stream.write(response.as_bytes());
                }
                "get" => {
                    let mut map = thread_shared_redis_cache.lock().unwrap();
                    let null_string = String::from("");
                    let mut default_val = Value {
                        data: null_string.clone(),
                        created_at: None,
                        expiry: None,
                    };
                    let content = map
                        .get_mut(&req.parameters[1])
                        .unwrap_or_else(|| &mut default_val);
                    let response;

                    let current_time = SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap()
                        .as_millis();
                    let is_key_expired = current_time
                        - content.created_at.unwrap_or_else(|| current_time)
                        > content.expiry.unwrap_or_else(|| 0) as u128;

                    if is_key_expired {
                        map.remove(&req.parameters[1]);
                        response = make_response(&null_string, ResponseType::NullBulkString);
                    } else {
                        if content.data.as_str() == null_string.as_str() {
                            response = make_response(&null_string, ResponseType::NullBulkString);
                        } else {
                            response = make_response(&content.data, ResponseType::BulkString);
                        }
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

// Another thread in loop to check and remove the expired data (inefficient)

// fn check_and_remove_expired_data(redis_cache: &Arc<Mutex<HashMap<String, Value>>>) {
//     let thread_shared_redis_cache = Arc::clone(redis_cache);
//     thread::spawn(move || loop {
//         let mut map = thread_shared_redis_cache.lock().unwrap();
//         map.retain(|key, val| {
//             let current_time = SystemTime::now()
//                 .duration_since(UNIX_EPOCH)
//                 .unwrap()
//                 .as_millis();
//             let is_key_expired = current_time - (val.created_at.unwrap_or_else(|| current_time))
//                 > val.expiry.unwrap_or_else(|| 0) as u128;
//             println!("key:{}, expired: {}", key, is_key_expired);
//             return !is_key_expired;
//         });

//         thread::sleep(Duration::from_millis(1));
//     });
// }

fn main() {
    // You can use print statements as follows for debugging, they'll be visible when running tests.
    println!("Logs from your program will appear here!");

    // Uncomment this block to pass the first stage

    let listener = TcpListener::bind("127.0.0.1:6379").unwrap();
    let redis_cache: Arc<Mutex<HashMap<String, Value>>> = Arc::new(Mutex::new(HashMap::new()));
    //let ARedisCache = Arc::new(RedisCache);
    // check_and_remove_expired_data(&redis_cache);

    for stream in listener.incoming() {
        handle_connection_helper(stream, &redis_cache);
        println!("redis cache outside {:?}", redis_cache.lock().unwrap())
    }
}
