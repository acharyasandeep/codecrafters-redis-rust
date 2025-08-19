use std::{io::Write, sync::MutexGuard};

use crate::{request::Request, SharedData};

pub fn hex_to_string(hex_code: &str) -> String {
    let mut hex_decoded_string = String::from("");
    let chars: Vec<char> = hex_code.chars().collect();

    let mut iter = chars.iter();

    while let Some(first_char) = iter.next() {
        let second = iter.next(); // Get the next character, if it exists

        if let Some(second_char) = second {
            let pair = format!("{}{}", first_char, second_char);
            let dec_ascii = u8::from_str_radix(pair.as_str(), 16);
            match dec_ascii {
                Ok(num) => {
                    if num >= 32 && num <= 126 {
                        let character = num as char;
                        hex_decoded_string += format!("{}", character as char).as_str();
                    } else {
                        hex_decoded_string += ".";
                    }
                }
                Err(_e) => {
                    hex_decoded_string += ".";
                }
            }
        }
    }

    hex_decoded_string
}

pub fn make_request(req: Request) -> String {
    let mut request_string = String::new();
    request_string.push_str(format!("*{}\r\n", req.parameter_count).as_str());

    for param in req.parameters {
        request_string.push_str(format!("${}\r\n", param.len()).as_str());
        request_string.push_str(format!("{}\r\n", param).as_str());
    }

    request_string
}

pub fn sync_to_replica(
    shared_data: MutexGuard<'_, SharedData>,
    content: Request,
) -> Result<(), String> {
    let mut errors = Vec::new();

    for mut replica in &shared_data.replica_connections {
        println!("Syncing to replica: {:?}", replica);
        let request_string = make_request(content.clone());

        if let Err(e) = replica.write_all(request_string.as_bytes()) {
            println!("Write error to replica: {:?}", e);
            errors.push(e.to_string());
            continue; // Skip to next replica
        }

        if let Err(e) = replica.flush() {
            println!("Flush error to replica: {:?}", e);
            errors.push(e.to_string());
            continue; // Skip to next replica
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(format!("Errors during sync: {:?}", errors))
    }
}

pub fn get_empty_rdb() -> Vec<u8> {
    let hex_empty_rdb = "524544495330303131fa0972656469732d76657205372e322e30fa0a72656469732d62697473c040fa056374696d65c26d08bc65fa08757365642d6d656dc2b0c41000fa08616f662d62617365c000fff06e3bfec0ff5aa2";
    let empty_file_payload = hex::decode(hex_empty_rdb).map_err(|decoding_err| {
        std::io::Error::new(std::io::ErrorKind::InvalidData, decoding_err.to_string())
    });
    empty_file_payload.unwrap()
}
