use crate::cli::{format_command_args, render_list, render_map, render_single};
use crate::config::{OutputFormat, PinchManifest, ProjectManifest};
use crate::networks::build_docker_network_command;
use crate::vars::apply_vars;
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet, HashMap};

pub fn list_processes(manifest: &PinchManifest, format: Option<OutputFormat>) -> Result<(), String> {
    let mut titles = Vec::new();
    if let Some(processes) = &manifest.processes {
        for proc in processes {
            titles.push(proc.title.clone());
        }
    }
    render_list(&titles, format)
}

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
        OutputFormat::Properties | OutputFormat::Raw => {
            let mut map = BTreeMap::new();
            map.insert("name", project.name.clone());
            if let Some(t) = &project.project_type {
                map.insert("type", format!("{:?}", t).to_lowercase());
            }
            if let Some(e) = &project.exposure {
                map.insert("exposure", format!("{:?}", e).to_lowercase());
            }
            if let Some(l) = &project.lifecycle {
                map.insert("lifecycle", format!("{:?}", l).to_lowercase());
            }
            if let Some(a) = &project.auth {
                map.insert("auth", format!("{:?}", a).to_lowercase());
            }
            render_map(&map, format)?;
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
            let yaml_str = serde_yaml::to_string(&config).map_err(|e| format!("Failed to serialize to YAML: {}", e))?;
            print!("{}", yaml_str);
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

pub fn list_networks(manifest: &PinchManifest, format: Option<OutputFormat>) -> Result<(), String> {
    let mut net_names = Vec::new();
    if let Some(networks) = &manifest.docker_networks {
        for name in networks.keys() {
            net_names.push(name.clone());
        }
    }
    render_list(&net_names, format)
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
        let empty_map: BTreeMap<String, String> = BTreeMap::new();
        render_map(&empty_map, format)?;
        Ok(())
    }
}

fn used_containers(manifest: &PinchManifest, context_vars: &HashMap<String, String>) -> Result<BTreeSet<String>, String> {
    let mut images = BTreeSet::new();
    if let Some(processes) = &manifest.processes {
        for proc in processes {
            if let crate::config::RunManifest::Detailed(crate::config::RunKind::Docker(ref docker_cfg)) = proc.run {
                let resolved_image = apply_vars(&docker_cfg.image, context_vars, false);
                images.insert(resolved_image);
            }
        }
    }
    Ok(images)
}

pub fn list_images(
    manifest: &PinchManifest,
    cli_vars: &HashMap<String, String>,
    format: Option<OutputFormat>,
) -> Result<(), String> {
    let context_vars = manifest.resolve_vars(cli_vars);
    let containers = used_containers(manifest, &context_vars)?;
    let images: Vec<String> = containers.into_iter().collect();
    render_list(&images, format)
}

#[derive(Debug, Serialize)]
pub struct AuditReport<'a> {
    pub project: &'a ProjectManifest,
    pub containers: Vec<String>,
}

pub fn show_audit(
    manifest: &PinchManifest,
    cli_vars: &HashMap<String, String>,
    format: Option<OutputFormat>,
) -> Result<(), String> {
    let context_vars = manifest.resolve_vars(cli_vars);
    let containers = used_containers(manifest, &context_vars)?;
    let report = AuditReport {
        project: &manifest.project,
        containers: containers.into_iter().collect(),
    };
    match format.unwrap_or(OutputFormat::Json) {
        OutputFormat::Json => {
            let json_str = serde_json::to_string_pretty(&report)
                .map_err(|e| format!("Failed to serialize audit report to JSON: {}", e))?;
            println!("{}", json_str);
        }
        OutputFormat::Yaml => {
            let yaml_str = serde_yaml::to_string(&report)
                .map_err(|e| format!("Failed to serialize audit report to YAML: {}", e))?;
            print!("{}", yaml_str);
        }
        OutputFormat::Raw | OutputFormat::Properties => {
            let json_str = serde_json::to_string_pretty(&report)
                .map_err(|e| format!("Failed to serialize audit report to JSON: {}", e))?;
            println!("{}", json_str);
        }
    }
    Ok(())
}