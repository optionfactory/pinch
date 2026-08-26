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

/// Replaces every `{{ name }}` placeholder in `text` with the corresponding value.
///
/// Substitution is purely textual and happens *before* any command parsing, so a
/// value is spliced verbatim into the surrounding string (like a Makefile
/// variable or an unquoted `$VAR` in a shell). Consequences for `cmd` strings:
/// - a value containing spaces yields several argv tokens (`flags: "-c 10"`);
///   wrap the placeholder in quotes in the template (`"{{ dir }}"`) to keep it
///   as a single token;
/// - quotes inside a value are interpreted by the command parser, exactly as if
///   they had been written inline.
///
/// Unknown placeholders and unterminated `{{` are left untouched.
pub fn apply_vars(text: &str, vars: &HashMap<String, String>) -> String {
    let mut result = String::new();
    let mut rest = text;
    while let Some(start) = rest.find("{{") {
        result.push_str(&rest[..start]);
        let after_open = &rest[start + 2..];
        if let Some(end) = after_open.find("}}") {
            let key = after_open[..end].trim();
            if let Some(val) = vars.get(key) {
                result.push_str(val);
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
        assert_eq!(apply_vars("hello {{name}}!", &v), "hello acme!");
    }

    #[test]
    fn leaves_unknown_var_verbatim() {
        let v = vars(&[("name", "acme")]);
        assert_eq!(apply_vars("hi {{missing}}", &v), "hi {{missing}}");
    }

    #[test]
    fn trims_whitespace_around_key() {
        let v = vars(&[("name", "acme")]);
        assert_eq!(apply_vars("hi {{ name }}", &v), "hi acme");
    }

    #[test]
    fn substitutes_multiple_vars_in_one_string() {
        let v = vars(&[("a", "1"), ("b", "2")]);
        assert_eq!(apply_vars("{{a}}-{{b}}", &v), "1-2");
    }

    #[test]
    fn passes_through_text_without_placeholders() {
        let v = vars(&[("a", "1")]);
        assert_eq!(apply_vars("plain text", &v), "plain text");
    }

    #[test]
    fn substitution_is_textual_and_never_adds_quotes() {
        let v = vars(&[("dir", "/path with spaces"), ("q", r#"a"b"#)]);
        assert_eq!(apply_vars("cd {{dir}}", &v), "cd /path with spaces");
        assert_eq!(apply_vars(r#"cd "{{dir}}""#, &v), r#"cd "/path with spaces""#);
        assert_eq!(apply_vars("echo {{q}}", &v), r#"echo a"b"#);
    }

    #[test]
    fn leaves_unclosed_placeholder_verbatim() {
        let v = vars(&[("a", "1")]);
        assert_eq!(apply_vars("oops {{a", &v), "oops {{a");
    }
}
