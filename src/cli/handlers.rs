use crate::cli::{format_command_args, render_map, render_single};
use crate::config::{OutputFormat, PinchManifest};
use crate::networks::build_docker_network_command;
use crate::vars::apply_vars;
use std::collections::{BTreeMap, BTreeSet, HashMap};

pub fn show_vars(
    manifest: &PinchManifest,
    cli_vars: &HashMap<String, String>,
    target: Option<&str>,
    format: Option<OutputFormat>,
) -> Result<(), String> {
    let context_vars = manifest.resolve_vars(cli_vars);

    if let Some(name) = target {
        let Some(val) = context_vars.get(name) else {
            return Err(format!("Variable '{}' not found in configuration", name));
        };
        render_single(val, format)?;
    } else {
        let sorted: BTreeMap<&String, &String> = context_vars.iter().collect();
        render_map(&sorted, format)?;
    }
    Ok(())
}

pub fn show_project(manifest: &PinchManifest, format: Option<OutputFormat>) -> Result<(), String> {
    let project = &manifest.project;
    match format.unwrap_or(OutputFormat::Yaml) {
        OutputFormat::Yaml => {
            let yaml_str = serde_yaml::to_string(project).map_err(|e| format!("Failed to serialize to YAML: {}", e))?;
            print!("{}", yaml_str);
        }
        OutputFormat::Json => {
            let json_str =
                serde_json::to_string_pretty(project).map_err(|e| format!("Failed to serialize to JSON: {}", e))?;
            println!("{}", json_str);
        }
        OutputFormat::Raw | OutputFormat::Properties => {
            println!("{:#?}", project);
        }
    }
    Ok(())
}

pub fn show_config(
    manifest: &PinchManifest,
    cli_vars: &HashMap<String, String>,
    format: Option<OutputFormat>,
) -> Result<(), String> {
    let config = manifest.prepare(cli_vars.clone(), false)?;
    match format.unwrap_or(OutputFormat::Yaml) {
        OutputFormat::Yaml => {
            let yaml_str = serde_yaml::to_string(&config).map_err(|e| format!("Failed to serialize to YAML: {}", e))?;
            print!("{}", yaml_str);
        }
        OutputFormat::Json => {
            let json_str =
                serde_json::to_string_pretty(&config).map_err(|e| format!("Failed to serialize to JSON: {}", e))?;
            println!("{}", json_str);
        }
        OutputFormat::Raw | OutputFormat::Properties => {
            println!("{:#?}", config);
        }
    }
    Ok(())
}

pub fn show_processes(
    manifest: &PinchManifest,
    cli_vars: &HashMap<String, String>,
    target: Option<&str>,
    format: Option<OutputFormat>,
) -> Result<(), String> {
    let config = manifest.prepare(cli_vars.clone(), false)?;

    if let Some(title) = target {
        let Some(proc) = config.processes.iter().find(|p| p.title == title) else {
            return Err(format!("Process with title '{}' not found in configuration", title));
        };
        let cmd_str = proc.cmd.join(" ");
        render_single(&cmd_str, format)?;
    } else {
        let mut sorted: BTreeMap<String, String> = BTreeMap::new();
        for p in &config.processes {
            sorted.insert(p.title.clone(), p.cmd.join(" "));
        }
        render_map(&sorted, format)?;
    }
    Ok(())
}

pub fn list_networks(manifest: &PinchManifest) {
    if let Some(networks) = &manifest.docker_networks {
        for name in networks.keys() {
            println!("  - {}", name);
        }
    } else {
        println!("  (No networks defined)");
    }
}

pub fn show_networks(
    manifest: &PinchManifest,
    cli_vars: &HashMap<String, String>,
    target: Option<&str>,
    format: Option<OutputFormat>,
) -> Result<(), String> {
    let context_vars = manifest.resolve_vars(cli_vars);

    if let Some(networks) = &manifest.docker_networks {
        if let Some(name) = target {
            let Some(config) = networks.get(name) else {
                return Err(format!("Network '{}' not found in configuration", name));
            };
            let cmd_args = build_docker_network_command(name, config, &context_vars);
            let cmd_str = format_command_args(cmd_args);
            render_single(&cmd_str, format)?;
        } else {
            let mut sorted: BTreeMap<&String, String> = BTreeMap::new();
            for (name, config) in networks {
                let cmd_args = build_docker_network_command(name, config, &context_vars);
                sorted.insert(name, format_command_args(cmd_args));
            }
            render_map(&sorted, format)?;
        }
        Ok(())
    } else if let Some(t) = target {
        Err(format!("Network '{}' not found in configuration", t))
    } else {
        println!("(No Docker networks defined)");
        Ok(())
    }
}

pub fn list_images(
    manifest: &PinchManifest,
    cli_vars: &HashMap<String, String>,
    format: Option<OutputFormat>,
) -> Result<(), String> {
    let context_vars = manifest.resolve_vars(cli_vars);
    let mut images = BTreeSet::new();

    if let Some(processes) = &manifest.processes {
        for proc in processes {
            if let crate::config::RunManifest::Detailed(crate::config::RunKind::Docker(ref docker_cfg)) = proc.run {
                let resolved_image = apply_vars(&docker_cfg.image, &context_vars, false);
                images.insert(resolved_image);
            }
        }
    }

    let chosen_format = format.unwrap_or(OutputFormat::Raw);
    match chosen_format {
        OutputFormat::Raw | OutputFormat::Properties => {
            for img in &images {
                println!("{}", img);
            }
        }
        OutputFormat::Yaml => {
            let yaml_str = serde_yaml::to_string(&images).map_err(|e| format!("Failed to serialize to YAML: {}", e))?;
            print!("{}", yaml_str);
        }
        OutputFormat::Json => {
            let json_str =
                serde_json::to_string_pretty(&images).map_err(|e| format!("Failed to serialize to JSON: {}", e))?;
            println!("{}", json_str);
        }
    }
    Ok(())
}
