pub fn to_pascal_case(s: &str) -> String {
    let mut result = String::new();
    let mut capitalize = true;
    for c in s.chars() {
        if c == '_' {
            capitalize = true;
        } else if capitalize {
            result.push(c.to_ascii_uppercase());
            capitalize = false;
        } else {
            result.push(c);
        }
    }
    result
}

pub fn get_type_name<T: ?Sized + 'static>() -> String {
    let full_type_name = std::any::type_name::<T>();
    match full_type_name.rsplit_once("::") {
        Some((_path, type_name)) => type_name.to_string(),
        None => full_type_name.to_string(), // Handle cases without a path
    }
}
