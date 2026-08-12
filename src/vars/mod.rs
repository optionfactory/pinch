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

#[cfg(test)]
mod tests {
    use super::*;

    fn vars(pairs: &[(&str, &str)]) -> HashMap<String, String> {
        pairs.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect()
    }

    #[test]
    fn substitutes_known_var() {
        let v = vars(&[("name", "acme")]);
        assert_eq!(apply_vars("hello {{name}}!", &v, false), "hello acme!");
    }

    #[test]
    fn leaves_unknown_var_verbatim() {
        let v = vars(&[("name", "acme")]);
        assert_eq!(apply_vars("hi {{missing}}", &v, false), "hi {{missing}}");
    }

    #[test]
    fn trims_whitespace_around_key() {
        let v = vars(&[("name", "acme")]);
        assert_eq!(apply_vars("hi {{ name }}", &v, false), "hi acme");
    }

    #[test]
    fn substitutes_multiple_vars_in_one_string() {
        let v = vars(&[("a", "1"), ("b", "2")]);
        assert_eq!(apply_vars("{{a}}-{{b}}", &v, false), "1-2");
    }

    #[test]
    fn passes_through_text_without_placeholders() {
        let v = vars(&[("a", "1")]);
        assert_eq!(apply_vars("plain text", &v, false), "plain text");
    }

    #[test]
    fn quotes_values_containing_spaces_when_requested() {
        let v = vars(&[("dir", "/path with spaces")]);
        assert_eq!(apply_vars("cd {{dir}}", &v, true), r#"cd "/path with spaces""#);
    }

    #[test]
    fn does_not_quote_when_flag_disabled() {
        let v = vars(&[("dir", "/path with spaces")]);
        assert_eq!(apply_vars("cd {{dir}}", &v, false), "cd /path with spaces");
    }

    #[test]
    fn leaves_unclosed_placeholder_verbatim() {
        let v = vars(&[("a", "1")]);
        assert_eq!(apply_vars("oops {{a", &v, false), "oops {{a");
    }
}
