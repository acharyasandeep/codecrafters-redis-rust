use crate::request::Request;
use crate::response::{make_response, ResponseType};
use crate::Value;
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    time::{SystemTime, UNIX_EPOCH},
};

pub fn handle_echo(req: Request) -> String {
    let content = &req.parameters[1];

    make_response(content, ResponseType::BulkString)
}

pub fn handle_set(
    req: Request,
    thread_shared_redis_cache: &Arc<Mutex<HashMap<String, Value>>>,
) -> String {
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
    make_response(&String::from("OK"), ResponseType::SimpleString)
}

pub fn handle_get(
    req: Request,
    thread_shared_redis_cache: &Arc<Mutex<HashMap<String, Value>>>,
) -> String {
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
    let mut is_key_expired = false;
    if let Some(expiry) = content.expiry {
        is_key_expired =
            current_time - content.created_at.unwrap_or_else(|| current_time) > expiry as u128;
    }

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
    response
}
