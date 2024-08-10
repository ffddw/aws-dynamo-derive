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

pub fn strip_quote_mark(val: &str) -> Option<&str> {
    val.strip_prefix('"')?.strip_suffix('"')
}