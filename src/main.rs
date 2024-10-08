pub mod handlers;
pub mod request;
pub mod response;
pub mod utils;
use request::Request;

use handlers::{handle_echo, handle_get, handle_info, handle_psync, handle_replconf, handle_set};
use std::{
    collections::HashMap,
    env,
    io::{BufReader, BufWriter, Error, Read, Write},
    net::{TcpListener, TcpStream, ToSocketAddrs},
    sync::{Arc, Mutex},
    thread::{self, sleep},
    time::Duration,
};

pub enum ResponseEnum {
    OK,
    PONG,
}

impl ResponseEnum {
    pub fn as_string(&self) -> String {
        match self {
            ResponseEnum::OK => String::from("+OK\r\n"),
            ResponseEnum::PONG => String::from("+PONG\r\n"),
        }
    }
}

pub enum RequestEnum {
    PING,
    REPLCONF1,
    REPLCONF2,
    PSYNC,
}

impl RequestEnum {
    pub fn as_string(&self) -> String {
        match self {
            RequestEnum::PING => String::from("*1\r\n$4\r\nPING\r\n"),
            RequestEnum::REPLCONF1 => {
                String::from("*3\r\n$8\r\nREPLCONF\r\n$14\r\nlistening-port\r\n$4\r\n{}\r\n")
            }
            RequestEnum::REPLCONF2 => {
                String::from("*3\r\n$8\r\nREPLCONF\r\n$4\r\ncapa\r\n$6\r\npsync2\r\n")
            }
            RequestEnum::PSYNC => String::from("*3\r\n$5\r\nPSYNC\r\n$1\r\n?\r\n$2\r\n-1\r\n"),
        }
    }
}

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
    master_replid: String,
    master_repl_offset: i32,
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
                "replconf" => {
                    let response = handle_replconf(req);
                    let _ = stream.write(response.as_bytes());
                }
                "psync" => {
                    let response = handle_psync(req, &thread_shared_data);
                    let _ = stream.write(response.as_bytes());

                    //Second data won't be read so, a simple hack here. *
                    sleep(Duration::from_secs(1));
                    //
                    let empty_rdb = get_empty_rdb();
                    let content = [
                        format!("${}\r\n", empty_rdb.len()).as_bytes(),
                        empty_rdb.as_slice(),
                    ]
                    .concat();
                    let _ = stream.write(&content);
                }
                _ => {
                    let _ = stream.write(ResponseEnum::PONG.as_string().as_bytes());
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
    let master_replid = "8371b4fb1155b71f4a04d3e1bc3e18c4a990aeeb";
    let master_repl_offset = 0;
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
        master_replid: master_replid.to_string(),
        master_repl_offset: master_repl_offset,
    };
    return (port, replication_info);
}

fn do_handshake(_stream: TcpStream, port: &str) {
    _stream
        .set_read_timeout(Some(Duration::from_secs(1)))
        .unwrap();
    let mut reader = BufReader::new(&_stream);
    let mut writer = BufWriter::new(&_stream);
    let _ = writer.write(RequestEnum::PING.as_string().as_bytes());
    writer.flush().unwrap();

    let mut buf = String::from("");

    let _ = reader.read_to_string(&mut buf);
    println!("Got response {:?}", buf);
    if buf == ResponseEnum::PONG.as_string() {
        let replconf_first_str =
            str::replace(RequestEnum::REPLCONF1.as_string().as_str(), "{}", port);
        let _ = writer.write(replconf_first_str.as_bytes());
        writer.flush().unwrap();
        buf.clear();
        let _ = reader.read_to_string(&mut buf);
        println!("Got response {:?}", buf);
        if buf == ResponseEnum::OK.as_string() {
            let _ = writer.write(RequestEnum::REPLCONF2.as_string().as_bytes());
            writer.flush().unwrap();
            buf.clear();
            let _ = reader.read_to_string(&mut buf);
            println!("Got response {:?}", buf);
            if buf == ResponseEnum::OK.as_string() {
                let _ = writer.write(RequestEnum::PSYNC.as_string().as_bytes());
                writer.flush().unwrap();
                buf.clear();
                let _ = reader.read_to_string(&mut buf);
                println!("Got response {:?}", buf);
                buf.clear();
                writer.flush().unwrap();
                let mut buf = vec![];
                let _ = reader.read_to_end(&mut buf);

                println!("Got response {:?}", buf);
            }
        }
    }
}

fn get_empty_rdb() -> Vec<u8> {
    let hex_empty_rdb = "524544495330303131fa0972656469732d76657205372e322e30fa0a72656469732d62697473c040fa056374696d65c26d08bc65fa08757365642d6d656dc2b0c41000fa08616f662d62617365c000fff06e3bfec0ff5aa2";
    let empty_file_payload = hex::decode(hex_empty_rdb).map_err(|decoding_err| {
        std::io::Error::new(std::io::ErrorKind::InvalidData, decoding_err.to_string())
    });
    empty_file_payload.unwrap()
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
            let master_host = host_port.next().unwrap_or_else(|| "can't unwrap host");
            let master_port = host_port.next().unwrap_or_else(|| "can't unwrap port");

            let sock_addr = (master_host.to_owned() + ":" + master_port)
                .to_socket_addrs()
                .unwrap()
                .filter(|addr| match addr.ip() {
                    std::net::IpAddr::V4(_) => true,
                    _ => false,
                })
                .next();
            let sock_addr_resolved = sock_addr.unwrap();
            println!("{}", sock_addr_resolved);

            let stream = TcpStream::connect_timeout(&sock_addr_resolved, Duration::from_secs(30));

            match stream {
                Ok(mut _stream) => {
                    do_handshake(_stream, port.to_string().as_str());
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
