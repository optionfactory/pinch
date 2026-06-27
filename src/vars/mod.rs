use std::collections::HashMap;

pub fn builtin_vars() -> HashMap<String, String> {
    let mut builtins = HashMap::new();
    if let Ok(current_pwd) = std::env::current_dir() {
        builtins.insert("pwd".to_string(), current_pwd.to_string_lossy().into_owned());
    }
    if let Ok(user) = std::env::var("USER").or_else(|_| std::env::var("USERNAME")) {
        builtins.insert("user".to_string(), user);
    }
    if let Ok(home) = std::env::var("HOME").or_else(|_| std::env::var("USERPROFILE")) {
        builtins.insert("home".to_string(), home);
    }
    builtins
}

pub fn apply_vars(text: &str, vars: &HashMap<String, String>, quote_spaces: bool) -> String {
    let mut result = String::new();
    let mut rest = text;
    while let Some(start) = rest.find("{{") {
        result.push_str(&rest[..start]);
        let after_open = &rest[start + 2..];
        if let Some(end) = after_open.find("}}") {
            let key = after_open[..end].trim();
            if let Some(val) = vars.get(key) {
                if quote_spaces && val.contains(' ') {
                    result.push('"');
                    result.push_str(val);
                    result.push('"');
                } else {
                    result.push_str(val);
                }
            } else {
                result.push_str("{{");
                result.push_str(&after_open[..end + 2]);
            }
            rest = &after_open[end + 2..];
        } else {
            result.push_str("{{");
            rest = after_open;
            break;
        }
    }
    result.push_str(rest);
    result
}
