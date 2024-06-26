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

#[derive(Debug)]
pub struct ReplicationInfo {
    replica_info: String,
    role: String,
}

#[derive(Debug)]
pub struct SharedData {
    replication_info: ReplicationInfo,
    redis_cache: HashMap<String, Value>,
}

fn handle_connection_helper(
    stream: Result<TcpStream, Error>,
    thread_shared_data: &Arc<Mutex<SharedData>>,
) {
    let thread_shared_data = Arc::clone(thread_shared_data);
    match stream {
        Ok(mut _stream) => {
            thread::spawn(move || handle_connection(_stream, thread_shared_data));
            // handle_connection(_stream);
        }
        Err(e) => {
            println!("error: {}", e);
        }
    }
}

fn handle_connection(mut stream: TcpStream, thread_shared_data: Arc<Mutex<SharedData>>) {
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
                    let response = handle_set(req, &thread_shared_data);
                    let _ = stream.write(response.as_bytes());
                }
                "get" => {
                    let response = handle_get(req, &thread_shared_data);
                    let _ = stream.write(response.as_bytes());
                }
                "info" => {
                    let response = handle_info(req, &thread_shared_data);
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

fn parse_args(args: Vec<String>) -> (i32, ReplicationInfo) {
    println!("Args are: {:?}", args);
    let mut port = 6379;
    let mut role = "master";
    let mut replica_info = String::from("");

    for i in 1..args.len() {
        let option = &args[i];

        if option == &String::from("--port") {
            let port_str = &args[i + 1];
            port = port_str
                .parse()
                .unwrap_or_else(|_| panic!("Invalid port specified."));
        } else if option == &String::from("--replicaof") {
            role = "slave";
            if let Some(replica_of) = args.get(i + 1) {
                replica_info = replica_of.clone()
            }
        }
    }
    // if args.len() >= 3 {
    //     let option = &args[1];

    //     if option == &String::from("--port") {
    //         let port_str = &args[2];
    //         port = port_str
    //             .parse()
    //             .unwrap_or_else(|_| panic!("Invalid port specified."));
    //     }
    // }
    let replication_info = ReplicationInfo {
        role: role.to_string(),
        replica_info,
    };
    return (port, replication_info);
}

fn main() {
    // You can use print statements as follows for debugging, they'll be visible when running tests.
    println!("Logs from your program will appear here!");

    let args: Vec<String> = env::args().collect();

    let (port, replication_info) = parse_args(args);

    let addr = String::from("127.0.0.1:") + &port.to_string();

    if replication_info.role == "slave" {
        if replication_info.replica_info != "" {
            let mut host_port = replication_info.replica_info.split_ascii_whitespace();
            let host = host_port.next().unwrap_or_else(|| "can't unwrap host");
            let port = host_port.next().unwrap_or_else(|| "can't unwrap port");

            let stream = TcpStream::connect(host.to_owned() + ":" + port);
            match stream {
                Ok(mut _stream) => {
                    let _ = _stream.write(b"*1\r\n$4\r\nPING\r\n");
                }
                Err(e) => {
                    panic!("can't connect to master, error: {:?}", e);
                }
            }
        }
    }

    let listener = TcpListener::bind(addr).unwrap();
    let thread_shared_data: Arc<Mutex<SharedData>> = Arc::new(Mutex::new(SharedData {
        replication_info,
        redis_cache: HashMap::new(),
    }));
    //let ARedisCache = Arc::new(RedisCache);
    // check_and_remove_expired_data(&redis_cache);

    for stream in listener.incoming() {
        handle_connection_helper(stream, &thread_shared_data);
        println!(
            "redis cache outside {:?}",
            thread_shared_data.lock().unwrap().redis_cache
        )
    }
}
