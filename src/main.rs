pub mod handlers;
pub mod request;
pub mod response;
use request::Request;

use handlers::{handle_echo, handle_get, handle_set};
use std::{
    collections::HashMap,
    io::{Error, Write},
    net::{TcpListener, TcpStream},
    sync::{Arc, Mutex},
    thread::{self},
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
    let redis_cache: Arc<Mutex<HashMap<String, Value>>> = Arc::new(Mutex::new(HashMap::new()));
    //let ARedisCache = Arc::new(RedisCache);
    // check_and_remove_expired_data(&redis_cache);

    for stream in listener.incoming() {
        handle_connection_helper(stream, &redis_cache);
        println!("redis cache outside {:?}", redis_cache.lock().unwrap())
    }
}
