use std::{
    io::{BufRead, BufReader},
    net::TcpStream,
};

#[derive(Debug)]
pub struct Request {
    pub parameter_count: i32,
    pub parameters: Vec<String>,
}

impl Request {
    pub fn new(mut stream: &mut TcpStream) -> Option<Request> {
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
