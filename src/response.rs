pub enum ResponseType {
    SimpleString,
    BulkString,
    NullBulkString,
}

impl ResponseType {
    pub fn as_str(&self) -> &'static str {
        match self {
            ResponseType::SimpleString => "+",
            ResponseType::BulkString => "$",
            ResponseType::NullBulkString => "$",
        }
    }
}

pub fn make_response(content: &String, content_type: ResponseType) -> String {
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
            response = format!("{}{}\r\n", content_type.as_str(), -1);
        }
        ResponseType::SimpleString => {
            response = format!("{}{}\r\n", content_type.as_str(), content);
        }
    }
    response
}
