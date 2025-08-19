pub mod handlers;
pub mod request;
pub mod response;
pub mod utils;
use request::Request;

use handlers::{handle_echo, handle_get, handle_info, handle_psync, handle_replconf, handle_set};
use std::{
    collections::HashMap,
    env,
    io::{BufRead, BufReader, BufWriter, Error, Read, Write},
    net::{TcpListener, TcpStream, ToSocketAddrs},
    sync::{Arc, Mutex},
    thread::{self},
};
use utils::{get_empty_rdb, sync_to_replica};

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

#[derive(Debug, Clone)]
pub struct ReplicationInfo {
    replica_info: String,
    role: String,
    master_replid: String,
    master_repl_offset: i32,
}

#[derive(Debug)]
pub struct SharedData {
    replica_connections: Vec<TcpStream>,
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
    loop {
        let request = Request::new(&mut stream);
        println!("Request received: {:?}", request);
        if let Some(req) = request {
            if req.parameter_count == 0 {
                continue;
            }
            match req.parameters[0].to_lowercase().as_str() {
                "echo" => {
                    let response = handle_echo(req);
                    let _ = stream.write(response.as_bytes());
                }
                "set" => {
                    let response = handle_set(req.clone(), &thread_shared_data);
                    let shared_data = thread_shared_data.lock().unwrap();
                    if shared_data.replication_info.role == "master" {
                        let _ = stream.write(response.as_bytes());
                        if let Err(e) = sync_to_replica(shared_data, req.clone()) {
                            eprintln!("Error syncing to replica: {}", e);
                        };
                    }
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

                    let empty_rdb = get_empty_rdb();
                    let content = [
                        format!("${}\r\n", empty_rdb.len()).as_bytes(),
                        empty_rdb.as_slice(),
                    ]
                    .concat();
                    let _ = stream.write(&content);

                    thread_shared_data
                        .lock()
                        .unwrap()
                        .replica_connections
                        .push(stream.try_clone().unwrap());
                    println!(
                        "Replica connections: {:?}",
                        thread_shared_data.lock().unwrap().replica_connections
                    );
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
    let replication_info = ReplicationInfo {
        role: role.to_string(),
        replica_info,
        master_replid: master_replid.to_string(),
        master_repl_offset: master_repl_offset,
    };
    return (port, replication_info);
}

fn do_handshake(_stream: &TcpStream, port: &str) {
    let mut reader = BufReader::new(_stream);
    let mut writer = BufWriter::new(_stream);
    let _ = writer.write(RequestEnum::PING.as_string().as_bytes());
    writer.flush().unwrap();

    let mut buf = String::from("");

    let _ = reader.read_line(&mut buf);
    println!("Got response {:?}", buf);
    if buf == ResponseEnum::PONG.as_string() {
        let replconf_first_str =
            str::replace(RequestEnum::REPLCONF1.as_string().as_str(), "{}", port);
        let _ = writer.write(replconf_first_str.as_bytes());
        writer.flush().unwrap();
        buf.clear();
        let _ = reader.read_line(&mut buf);
        println!("Got response {:?}", buf);
        if buf == ResponseEnum::OK.as_string() {
            let _ = writer.write(RequestEnum::REPLCONF2.as_string().as_bytes());
            writer.flush().unwrap();
            buf.clear();
            let _ = reader.read_line(&mut buf);
            println!("Got response {:?}", buf);
            if buf == ResponseEnum::OK.as_string() {
                let _ = writer.write(RequestEnum::PSYNC.as_string().as_bytes());
                writer.flush().unwrap();
                buf.clear();
                let _ = reader.read_line(&mut buf);
                println!("Got response {:?}", buf);
                buf.clear();
                writer.flush().unwrap();
                let _ = reader.read_line(&mut buf);
                let buff_trimmed = buf.trim()[1..].to_string();
                let length = buff_trimmed.parse::<usize>().unwrap_or(0);
                let mut binary_rdb_file = vec![0; length];
                let _ = reader.read_exact(&mut binary_rdb_file);

                println!("Got response {:?}{:?}", buf, binary_rdb_file);
            }
        }
    }
}

fn main() {
    // You can use print statements as follows for debugging, they'll be visible when running tests.
    println!("Logs from your program will appear here!");

    let args: Vec<String> = env::args().collect();

    let (port, replication_info) = parse_args(args);

    let addr = String::from("127.0.0.1:") + &port.to_string();

    let thread_shared_data: Arc<Mutex<SharedData>> = Arc::new(Mutex::new(SharedData {
        replica_connections: Vec::new(),
        replication_info: replication_info.clone(),
        redis_cache: HashMap::new(),
    }));

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

            let slave_stream = TcpStream::connect(&sock_addr_resolved);

            match slave_stream {
                Ok(ref _stream) => {
                    do_handshake(&_stream, port.to_string().as_str());
                }
                Err(e) => {
                    panic!("can't connect to master, error: {:?}", e);
                }
            }
            let thread_shared_data_clone = Arc::clone(&thread_shared_data);

            handle_connection_helper(slave_stream, &thread_shared_data_clone);
        }
    }

    let listener = TcpListener::bind(addr).unwrap();

    for stream in listener.incoming() {
        handle_connection_helper(stream, &thread_shared_data);
    }
}
