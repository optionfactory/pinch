use crate::cli::{OutputFormat, format_command_args, render_list, render_map, render_object, render_single};
use crate::config::PinchManifest;
use crate::networks::build_docker_network_command;
use crate::vars::apply_vars;
use std::collections::{BTreeMap, BTreeSet, HashMap};

pub fn list_processes(manifest: &PinchManifest, format: Option<OutputFormat>) -> Result<(), String> {
    let names: Vec<String> = manifest
        .processes
        .as_deref()
        .unwrap_or(&[])
        .iter()
        .map(|p| p.name.clone())
        .collect();
    render_list(&names, format)
}

pub fn show_vars(
    manifest: &PinchManifest,
    cli_vars: &HashMap<String, String>,
    target: Option<&str>,
    format: Option<OutputFormat>,
) -> Result<(), String> {
    let context_vars = manifest.resolve_vars(cli_vars);
    if let Some(name) = target {
        let val = context_vars
            .get(name)
            .ok_or_else(|| format!("Variable '{}' not found in configuration", name))?;
        render_single(val, format)?;
    } else {
        let sorted: BTreeMap<&String, &String> = context_vars.iter().collect();
        render_map(&sorted, format)?;
    }
    Ok(())
}

pub fn show_config(
    manifest: &PinchManifest,
    cli_vars: &HashMap<String, String>,
    format: Option<OutputFormat>,
) -> Result<(), String> {
    let config = manifest.prepare(cli_vars.clone(), false)?;
    render_object(&config, format, OutputFormat::Yaml)
}

pub fn show_processes(
    manifest: &PinchManifest,
    cli_vars: &HashMap<String, String>,
    target: Option<&str>,
    format: Option<OutputFormat>,
) -> Result<(), String> {
    let config = manifest.prepare(cli_vars.clone(), false)?;
    if let Some(name) = target {
        let proc = config
            .processes
            .iter()
            .find(|p| p.name == name)
            .ok_or_else(|| format!("Process with name '{}' not found in configuration", name))?;
        let cmd_str = proc.cmd.join(" ");
        render_single(&cmd_str, format)?;
    } else {
        let sorted: BTreeMap<String, String> = config
            .processes
            .iter()
            .map(|p| (p.name.clone(), p.cmd.join(" ")))
            .collect();
        render_map(&sorted, format)?;
    }
    Ok(())
}

pub fn list_networks(manifest: &PinchManifest, format: Option<OutputFormat>) -> Result<(), String> {
    let net_names: Vec<String> = manifest
        .docker_networks
        .as_ref()
        .map(|nets| nets.keys().cloned().collect())
        .unwrap_or_default();
    render_list(&net_names, format)
}

pub fn show_networks(
    manifest: &PinchManifest,
    cli_vars: &HashMap<String, String>,
    target: Option<&str>,
    format: Option<OutputFormat>,
) -> Result<(), String> {
    let context_vars = manifest.resolve_vars(cli_vars);
    let networks = manifest.docker_networks.as_ref();

    if let Some(name) = target {
        let config = networks
            .and_then(|nets| nets.get(name))
            .ok_or_else(|| format!("Network '{}' not found in configuration", name))?;
        let cmd_args = build_docker_network_command(name, config, &context_vars);
        render_single(&format_command_args(cmd_args), format)?;
    } else {
        let mut sorted = BTreeMap::new();
        if let Some(nets) = networks {
            for (name, config) in nets {
                let cmd_args = build_docker_network_command(name, config, &context_vars);
                sorted.insert(name, format_command_args(cmd_args));
            }
        }
        render_map(&sorted, format)?;
    }
    Ok(())
}

fn used_containers(
    manifest: &PinchManifest,
    context_vars: &HashMap<String, String>,
) -> Result<BTreeSet<String>, String> {
    let mut images = BTreeSet::new();
    if let Some(processes) = &manifest.processes {
        for proc in processes {
            if let crate::config::RunManifest::Detailed(crate::config::RunKind::Docker(ref docker_cfg)) = proc.run {
                let resolved_image = apply_vars(&docker_cfg.image, context_vars);
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
