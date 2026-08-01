use crate::config::{DockerNetworkConfig, PinchManifest};
use crate::vars::apply_vars;
use std::collections::HashMap;

pub fn build_docker_network_command(
    net_key: &str,
    config: &DockerNetworkConfig,
    vars: &HashMap<String, String>,
) -> Vec<String> {
    let net_name = apply_vars(net_key, vars, false);
    let mut cmd_args = vec!["docker".to_string(), "network".to_string(), "create".to_string()];
    cmd_args.push("-o".to_string());
    cmd_args.push(format!("com.docker.network.bridge.name={}", net_name));
    cmd_args.push("--subnet".to_string());
    cmd_args.push(apply_vars(config.subnet(), vars, false));
    cmd_args.push("-d".to_string());
    cmd_args.push("bridge".to_string());
    if let Some(args_list) = config.args() {
        for arg in args_list {
            cmd_args.push(apply_vars(arg, vars, false));
        }
    }
    cmd_args.push(net_name);
    cmd_args
}

pub fn ensure_docker_network(
    net_key: &str,
    config: &DockerNetworkConfig,
    vars: &HashMap<String, String>,
) -> Result<(), String> {
    use std::process::Command;
    let net_name = apply_vars(net_key, vars, false);
    let status = Command::new("docker")
        .args(["network", "inspect", &net_name])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map_err(|e| format!("Failed to inspect docker network '{}': {}", net_name, e))?;
    if !status.success() {
        let mut cmd = Command::new("docker");
        let cmd_args = build_docker_network_command(net_key, config, vars);
        cmd.args(&cmd_args[1..]);
        let create_status = cmd
            .status()
            .map_err(|e| format!("Failed to create docker network '{}': {}", net_name, e))?;
        if !create_status.success() {
            return Err(format!("Docker failed to create network: {}", net_name));
        }
    }
    Ok(())
}

pub fn create_networks(
    manifest: &PinchManifest,
    cli_vars: &HashMap<String, String>,
    target: Option<&str>,
) -> Result<(), String> {
    let context_vars = manifest.resolve_vars(cli_vars);
    if let Some(networks) = &manifest.docker_networks {
        for (name, config) in networks {
            if let Some(t) = target {
                if name != t {
                    continue;
                }
            }
            ensure_docker_network(name, config, &context_vars)?;
        }
    } else if let Some(t) = target {
        return Err(format!("Network '{}' not found in configuration", t));
    }
    Ok(())
}
