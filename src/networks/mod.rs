use crate::config::{DockerNetworkConfig, PinchManifest};
use crate::vars::apply_vars;
use std::collections::HashMap;

pub fn build_docker_network_command(
    net_key: &str,
    config: &DockerNetworkConfig,
    vars: &HashMap<String, String>,
) -> Vec<String> {
    let net_name = apply_vars(net_key, vars);
    let mut cmd_args = vec!["docker".to_string(), "network".to_string(), "create".to_string()];
    cmd_args.push("-o".to_string());
    cmd_args.push(format!("com.docker.network.bridge.name={}", net_name));
    cmd_args.push("--subnet".to_string());
    cmd_args.push(apply_vars(config.subnet(), vars));
    cmd_args.push("-d".to_string());
    cmd_args.push("bridge".to_string());
    if let Some(args_list) = config.args() {
        for arg in args_list {
            cmd_args.push(apply_vars(arg, vars));
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
    let net_name = apply_vars(net_key, vars);
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

/// Selects the networks to act on: all of them, or only `target` if given.
/// Errors when `target` names a network that is not defined.
pub fn select_networks<'a>(
    manifest: &'a PinchManifest,
    target: Option<&str>,
) -> Result<Vec<(&'a String, &'a DockerNetworkConfig)>, String> {
    let networks = manifest.docker_networks.as_ref();
    match target {
        Some(t) => {
            let config = networks
                .and_then(|nets| nets.get_key_value(t))
                .ok_or_else(|| format!("Network '{}' not found in configuration", t))?;
            Ok(vec![config])
        }
        None => Ok(networks.map(|nets| nets.iter().collect()).unwrap_or_default()),
    }
}

pub fn create_networks(
    manifest: &PinchManifest,
    cli_vars: &HashMap<String, String>,
    target: Option<&str>,
) -> Result<(), String> {
    let context_vars = manifest.resolve_vars(cli_vars);
    for (name, config) in select_networks(manifest, target)? {
        ensure_docker_network(name, config, &context_vars)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn manifest(yaml: &str) -> PinchManifest {
        serde_saphyr::from_str(yaml).expect("valid manifest")
    }

    #[test]
    fn unknown_target_is_an_error_even_when_networks_are_defined() {
        let m = manifest("schema_version: 1\nname: x\ndocker_networks:\n  hi: 172.18.23.0/24\n");
        let err = select_networks(&m, Some("nope")).unwrap_err();
        assert_eq!(err, "Network 'nope' not found in configuration");
    }

    #[test]
    fn unknown_target_is_an_error_when_no_networks_are_defined() {
        let m = manifest("schema_version: 1\nname: x\n");
        assert!(select_networks(&m, Some("nope")).is_err());
    }

    #[test]
    fn known_target_selects_only_that_network() {
        let m = manifest("schema_version: 1\nname: x\ndocker_networks:\n  a: 10.0.0.0/24\n  b: 10.0.1.0/24\n");
        let sel = select_networks(&m, Some("b")).unwrap();
        assert_eq!(sel.len(), 1);
        assert_eq!(sel[0].0, "b");
        assert_eq!(sel[0].1.subnet(), "10.0.1.0/24");
    }

    #[test]
    fn no_target_selects_all_or_none() {
        let m = manifest("schema_version: 1\nname: x\ndocker_networks:\n  a: 10.0.0.0/24\n  b: 10.0.1.0/24\n");
        assert_eq!(select_networks(&m, None).unwrap().len(), 2);
        let m = manifest("schema_version: 1\nname: x\n");
        assert!(select_networks(&m, None).unwrap().is_empty());
    }
}
