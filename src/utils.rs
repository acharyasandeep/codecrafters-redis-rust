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
