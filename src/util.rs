pub fn to_pascal_case(val: &str) -> String {
    let mut pascal = String::new();
    let mut capitalize = true;
    for ch in val.chars() {
        if ch == '_' {
            capitalize = true;
        } else if capitalize {
            pascal.push(ch.to_ascii_uppercase());
            capitalize = false;
        } else {
            pascal.push(ch);
        }
    }
    pascal
}

pub fn to_snake_case(val: &str) -> String {
    let mut snake = String::new();
    for (i, c) in val.chars().enumerate() {
        if c.is_uppercase() {
            if i != 0 {
                snake.push('_');
            }
            snake.push(c.to_ascii_lowercase());
        } else {
            snake.push(c);
        }
    }
    snake
}

pub fn strip_raw_r(val: &str) -> &str {
    val.strip_prefix("r#").unwrap_or(val)
}

pub fn strip_quote_mark(val: &str) -> Option<&str> {
    val.strip_prefix('"')?.strip_suffix('"')
}
