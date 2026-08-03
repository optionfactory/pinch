use crate::config::OutputFormat;
use serde::Serialize;
use std::{collections::BTreeMap, fmt::Debug};

pub fn format_command_args(args: Vec<String>) -> String {
    args.into_iter()
        .map(|arg| {
            shlex::try_quote(&arg)
                .unwrap_or(std::borrow::Cow::Borrowed(&arg))
                .to_string()
        })
        .collect::<Vec<String>>()
        .join(" ")
}

pub fn render_single<T: Serialize + std::fmt::Display>(value: &T, format: Option<OutputFormat>) -> Result<(), String> {
    match format.unwrap_or(OutputFormat::Raw) {
        OutputFormat::Raw | OutputFormat::Properties => println!("{}", value),
        OutputFormat::Yaml => {
            let yaml_str = serde_yaml::to_string(value).map_err(|e| format!("Failed to serialize to YAML: {}", e))?;
            print!("{}", yaml_str);
        }
        OutputFormat::Json => {
            let json_str =
                serde_json::to_string_pretty(value).map_err(|e| format!("Failed to serialize to JSON: {}", e))?;
            println!("{}", json_str);
        }
    }
    Ok(())
}

pub fn render_list<T: Serialize + std::fmt::Display>(list: &[T], format: Option<OutputFormat>) -> Result<(), String> {
    match format.unwrap_or(OutputFormat::Raw) {
        OutputFormat::Raw | OutputFormat::Properties => {
            for item in list {
                println!("{}", item);
            }
        }
        OutputFormat::Yaml => {
            let yaml_str = serde_yaml::to_string(list).map_err(|e| format!("Failed to serialize to YAML: {}", e))?;
            print!("{}", yaml_str);
        }
        OutputFormat::Json => {
            let json_str =
                serde_json::to_string_pretty(list).map_err(|e| format!("Failed to serialize to JSON: {}", e))?;
            println!("{}", json_str);
        }
    }
    Ok(())
}

pub fn render_map<K, V>(map: &BTreeMap<K, V>, format: Option<OutputFormat>) -> Result<(), String>
where
    K: std::fmt::Display + Serialize,
    V: std::fmt::Display + Serialize,
{
    match format.unwrap_or(OutputFormat::Yaml) {
        OutputFormat::Yaml => {
            let yaml_str = serde_yaml::to_string(map).map_err(|e| format!("Failed to serialize to YAML: {}", e))?;
            print!("{}", yaml_str);
        }
        OutputFormat::Raw => {
            for (key, val) in map {
                println!("{}\t{}", key, val);
            }
        }
        OutputFormat::Properties => {
            for (key, val) in map {
                println!("{}={}", key, val);
            }
        }
        OutputFormat::Json => {
            let json_str =
                serde_json::to_string_pretty(map).map_err(|e| format!("Failed to serialize to JSON: {}", e))?;
            println!("{}", json_str);
        }
    }
    Ok(())
}

pub fn render_object<T: Serialize + Debug>(
    value: &T,
    format: Option<OutputFormat>,
    default_format: OutputFormat,
) -> Result<(), String> {
    let fmt = format.unwrap_or(default_format);
    match fmt {
        OutputFormat::Yaml => {
            let yaml_str = serde_yaml::to_string(value).map_err(|e| format!("Failed to serialize to YAML: {}", e))?;
            print!("{}", yaml_str);
        }
        OutputFormat::Json => {
            let json_str =
                serde_json::to_string_pretty(value).map_err(|e| format!("Failed to serialize to JSON: {}", e))?;
            println!("{}", json_str);
        }
        OutputFormat::Raw => {
            // Outputs the raw Rust Debug representation
            println!("{:#?}", value);
        }
        OutputFormat::Properties => {
            let json_val =
                serde_json::to_value(value).map_err(|e| format!("Failed to convert object to Value: {}", e))?;
            let mut props = BTreeMap::new();
            flatten_json_value("", &json_val, &mut props);
            for (k, v) in props {
                println!("{}={}", k, v);
            }
        }
    }
    Ok(())
}

fn flatten_json_value(prefix: &str, value: &serde_json::Value, out: &mut BTreeMap<String, String>) {
    match value {
        serde_json::Value::Object(map) => {
            for (k, v) in map {
                let new_prefix = if prefix.is_empty() {
                    k.clone()
                } else {
                    format!("{}.{}", prefix, k)
                };
                flatten_json_value(&new_prefix, v, out);
            }
        }
        serde_json::Value::Array(arr) => {
            if arr.iter().all(|v| v.is_string() || v.is_number() || v.is_boolean()) {
                let joined = arr
                    .iter()
                    .map(|v| match v {
                        serde_json::Value::String(s) => s.clone(),
                        _ => v.to_string(),
                    })
                    .collect::<Vec<_>>()
                    .join(",");
                out.insert(prefix.to_string(), joined);
            } else {
                for (i, v) in arr.iter().enumerate() {
                    let new_prefix = format!("{}.{}", prefix, i);
                    flatten_json_value(&new_prefix, v, out);
                }
            }
        }
        serde_json::Value::String(s) => {
            out.insert(prefix.to_string(), s.clone());
        }
        serde_json::Value::Number(n) => {
            out.insert(prefix.to_string(), n.to_string());
        }
        serde_json::Value::Bool(b) => {
            out.insert(prefix.to_string(), b.to_string());
        }
        serde_json::Value::Null => {}
    }
}
