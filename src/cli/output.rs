use crate::config::OutputFormat;
use serde::Serialize;
use std::collections::BTreeMap;

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
