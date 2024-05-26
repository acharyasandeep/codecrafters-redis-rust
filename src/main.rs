pub mod handlers;
pub mod request;
pub mod response;
use request::Request;

use handlers::{handle_echo, handle_get, handle_info, handle_set};
use std::{
    collections::HashMap,
    env,
    io::{Error, Write},
    net::{TcpListener, TcpStream},
    sync::{Arc, Mutex},
    thread,
};

#[derive(Debug)]
pub struct Value {
    data: String,
    created_at: Option<u128>,
    expiry: Option<u64>,
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
                    let response = handle_echo(req);
                    let _ = stream.write(response.as_bytes());
                }
                "set" => {
                    let response = handle_set(req, &thread_shared_redis_cache);
                    let _ = stream.write(response.as_bytes());
                }
                "get" => {
                    let response = handle_get(req, &thread_shared_redis_cache);
                    let _ = stream.write(response.as_bytes());
                }
                "info" => {
                    let response = handle_info(req);
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

fn parse_args(args: Vec<String>) -> i32 {
    println!("Args are: {:?}", args);
    if args.len() >= 3 {
        let option = &args[1];

        if option == &String::from("--port") {
            let port = &args[2];
            let port_i32: i32 = port
                .parse()
                .unwrap_or_else(|_| panic!("Invalid port specified."));
            return port_i32;
        }
    }
    return 6379;
}

fn main() {
    // You can use print statements as follows for debugging, they'll be visible when running tests.
    println!("Logs from your program will appear here!");

    let args: Vec<String> = env::args().collect();

    let port = parse_args(args);

    let addr = String::from("127.0.0.1:") + &port.to_string();
    println!("Address is: {}", addr);

    let listener = TcpListener::bind(addr).unwrap();
    let redis_cache: Arc<Mutex<HashMap<String, Value>>> = Arc::new(Mutex::new(HashMap::new()));
    //let ARedisCache = Arc::new(RedisCache);
    // check_and_remove_expired_data(&redis_cache);

    for stream in listener.incoming() {
        handle_connection_helper(stream, &redis_cache);
        println!("redis cache outside {:?}", redis_cache.lock().unwrap())
    }
}
