use std::collections::{BTreeMap, HashMap};

use crate::config::{ContainerEngine, DockerMountConfig, DockerRunConfig, RunMode};
use crate::runners::{BuildOutput, BuildResult, RunBuilder, RunContext};
use crate::vars::apply_vars;

fn expand(value: &str, vars: &HashMap<String, String>) -> String {
    apply_vars(value, vars, false).trim().to_string()
}

fn render_mount(mount: &DockerMountConfig, vars: &HashMap<String, String>) -> Option<String> {
    let source = expand(&mount.source, vars);
    if source.is_empty() {
        return None;
    }
    let target = mount
        .target
        .as_deref()
        .map(|t| expand(t, vars))
        .filter(|t| !t.is_empty())
        .unwrap_or_else(|| source.clone());
    let mount_type = mount
        .mount_type
        .as_deref()
        .map(str::trim)
        .filter(|t| !t.is_empty())
        .unwrap_or("bind");
    let mut flag = format!("type={mount_type},source={source},target={target}");
    if mount.readonly.unwrap_or(true) {
        flag.push_str(",readonly");
    }
    let extras: Vec<String> = mount
        .opts
        .as_deref()
        .map(|o| expand(o, vars))
        .unwrap_or_default()
        .split(',')
        .map(str::trim)
        .filter(|p| !p.is_empty())
        .map(str::to_string)
        .collect();
    if !extras.is_empty() {
        flag.push(',');
        flag.push_str(&extras.join(","));
    }
    Some(flag)
}

impl DockerRunConfig {
    fn engine_binary(&self) -> &'static str {
        match self.engine {
            Some(ContainerEngine::Podman) => "podman",
            _ => "docker",
        }
    }
}

fn shlex_tokens(value: &str, vars: &HashMap<String, String>, name: &str) -> Result<Vec<String>, String> {
    shlex::split(&expand(value, vars))
        .filter(|tokens| !tokens.is_empty())
        .ok_or_else(|| format!("Failed to parse docker run flags for '{}': {}", name, value))
}

impl RunBuilder for DockerRunConfig {
    fn build_command(&self, ctx: &RunContext) -> BuildResult {
        // Structured fields are passed as literal argv tokens (no shell involved),
        // so values containing spaces are preserved verbatim. Only the free-form
        // `opts` and `args` strings are shell-parsed via shlex.
        let mut tokens: Vec<String> = vec![
            self.engine_binary().to_string(),
            "run".to_string(),
            if ctx.background { "-d" } else { "-ti" }.to_string(),
            "--rm".to_string(),
        ];
        // Mirrors bundle: the enclosing process name doubles as the container name,
        // unless overridden explicitly.
        let container_name = self
            .name
            .as_deref()
            .map(|n| expand(n, ctx.vars))
            .filter(|n| !n.is_empty())
            .unwrap_or_else(|| ctx.name.to_string());
        tokens.push("--name".to_string());
        tokens.push(container_name);
        // Same defaulting as 'docker-intrude': fall back to the single network
        // defined in 'docker_networks' when no explicit network is configured.
        let network = self.network.as_deref().or(ctx.default_docker_network.map(String::as_str));
        if let Some(network) = network {
            let expanded = expand(network, ctx.vars);
            if !expanded.is_empty() {
                tokens.push("--network".to_string());
                tokens.push(expanded);
            }
        }
        if let Some(ip) = &self.ip {
            let expanded = expand(ip, ctx.vars);
            if !expanded.is_empty() {
                tokens.push("--ip".to_string());
                tokens.push(expanded);
            }
        }
        if let Some(env) = &self.env {
            for (key, value) in env.iter().collect::<BTreeMap<_, _>>() {
                tokens.push("--env".to_string());
                tokens.push(format!("{}={}", key, expand(value, ctx.vars)));
            }
        }
        for p in self.publish.iter().flatten() {
            let expanded = expand(p, ctx.vars);
            if !expanded.is_empty() {
                tokens.push("-p".to_string());
                tokens.push(expanded);
            }
        }
        for mount in self.mounts.iter().flatten() {
            if let Some(flag) = render_mount(mount, ctx.vars) {
                tokens.push("--mount".to_string());
                tokens.push(flag);
            }
        }
        for v in self.volumes.iter().flatten() {
            let expanded = expand(v, ctx.vars);
            if !expanded.is_empty() {
                tokens.push("--volume".to_string());
                tokens.push(expanded);
            }
        }
        if let Some(opts) = &self.opts {
            tokens.extend(shlex_tokens(opts, ctx.vars, ctx.name)?);
        }
        tokens.push(expand(&self.image, ctx.vars));
        if let Some(args) = &self.args {
            tokens.extend(shlex_tokens(args, ctx.vars, ctx.name)?);
        }

        Ok(BuildOutput {
            cmd: tokens,
            run_mode: RunMode::Exec,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx(vars: &HashMap<String, String>) -> RunContext<'_> {
        static EMPTY_NETWORKS: std::sync::OnceLock<HashMap<String, crate::config::DockerNetworkConfig>> =
            std::sync::OnceLock::new();
        RunContext {
            name: "test",
            vars,
            global_shell: None,
            default_docker_network: None,
            defined_networks: EMPTY_NETWORKS.get_or_init(HashMap::new),
            background: false,
        }
    }

    #[test]
    fn defaults_to_single_defined_docker_network() {
        let vars = HashMap::new();
        let default_network = "hi".to_string();
        let networks = HashMap::new();
        let context = RunContext {
            name: "test",
            vars: &vars,
            global_shell: None,
            default_docker_network: Some(&default_network),
            defined_networks: &networks,
            background: false,
        };
        let cfg = docker_cfg(r#"{ "image": "i" }"#);
        let out = cfg.build_command(&context).unwrap().cmd;
        assert_eq!(out, vec!["docker", "run", "-ti", "--rm", "--name", "test", "--network", "hi", "i"]);

        // An explicit network takes precedence over the default.
        let cfg = docker_cfg(r#"{ "image": "i", "network": "other" }"#);
        let out = cfg.build_command(&context).unwrap().cmd;
        assert_eq!(
            out,
            vec!["docker", "run", "-ti", "--rm", "--name", "test", "--network", "other", "i"]
        );
    }

    fn docker_cfg(json: &str) -> DockerRunConfig {
        serde_json::from_str(json).expect("valid docker run config")
    }

    #[test]
    fn builds_minimal_docker_run() {
        let vars = HashMap::new();
        let cfg = docker_cfg(r#"{ "image": "nginx:alpine" }"#);
        let out = cfg.build_command(&ctx(&vars)).unwrap();
        assert_eq!(
            out.cmd,
            vec!["docker", "run", "-ti", "--rm", "--name", "test", "nginx:alpine"]
        );
    }

    #[test]
    fn uses_explicit_container_name_when_provided() {
        let vars = HashMap::new();
        let cfg = docker_cfg(r#"{ "image": "i", "name": "hi-nginx" }"#);
        let out = cfg.build_command(&ctx(&vars)).unwrap().cmd;
        assert_eq!(&out[4..6], &["--name", "hi-nginx"]);
    }

    #[test]
    fn renders_all_flags_in_bundle_order() {
        let vars = HashMap::new();
        let cfg = docker_cfg(
            r#"{
                "image": "nginx:alpine",
                "name": "hi-nginx",
                "network": "hi",
                "ip": "172.18.23.10",
                "env": { "TZ": "Europe/Rome", "A": "1" },
                "publish": ["0.0.0.0:8888:8888", ""],
                "mounts": [
                    { "source": "/opt/conf/nginx.conf", "target": "/etc/nginx/nginx.conf" },
                    { "type": "volume", "source": "data", "target": "/data", "readonly": false, "opts": "ro-copy" }
                ],
                "volumes": ["mydata:/var/lib/app", ""],
                "opts": "--log-driver=none",
                "args": "nginx -g 'daemon off;'"
            }"#,
        );
        let out = cfg.build_command(&ctx(&vars)).unwrap();
        assert_eq!(
            out.cmd,
            vec![
                "docker",
                "run",
                "-ti",
                "--rm",
                "--name",
                "hi-nginx",
                "--network",
                "hi",
                "--ip",
                "172.18.23.10",
                "--env",
                "A=1",
                "--env",
                "TZ=Europe/Rome",
                "-p",
                "0.0.0.0:8888:8888",
                "--mount",
                "type=bind,source=/opt/conf/nginx.conf,target=/etc/nginx/nginx.conf,readonly",
                "--mount",
                "type=volume,source=data,target=/data,ro-copy",
                "--volume",
                "mydata:/var/lib/app",
                "--log-driver=none",
                "nginx:alpine",
                "nginx",
                "-g",
                "daemon off;"
            ]
        );
    }

    #[test]
    fn uses_podman_engine() {
        let vars = HashMap::new();
        let cfg = docker_cfg(r#"{ "engine": "podman", "image": "registry.example.com/my-app:2" }"#);
        let out = cfg.build_command(&ctx(&vars)).unwrap();
        assert_eq!(out.cmd.first().unwrap(), "podman");
    }

    #[test]
    fn expands_vars_and_drops_empty_flags() {
        let mut vars = HashMap::new();
        vars.insert("net".to_string(), String::new());
        vars.insert("img".to_string(), "nginx:alpine".to_string());
        let cfg = docker_cfg(r#"{ "image": "{{img}}", "network": "{{net}}", "ip": " {{net}} " }"#);
        let out = cfg.build_command(&ctx(&vars)).unwrap().cmd;
        assert_eq!(
            out,
            vec!["docker", "run", "-ti", "--rm", "--name", "test", "nginx:alpine"]
        );
    }

    #[test]
    fn mount_target_defaults_to_source() {
        let vars: HashMap<String, String> = HashMap::new();
        let m = serde_json::from_str::<DockerMountConfig>(r#"{ "source": "/data" }"#).unwrap();
        assert_eq!(
            render_mount(&m, &vars).unwrap(),
            "type=bind,source=/data,target=/data,readonly"
        );
    }

    #[test]
    fn network_flags_render_in_declaration_order() {
        let vars: HashMap<String, String> = HashMap::new();
        let cfg = docker_cfg(r#"{ "image": "i", "ip": "10.0.0.2", "network": "net1" }"#);
        let out = cfg.build_command(&ctx(&vars)).unwrap().cmd;
        assert_eq!(
            out,
            vec![
                "docker",
                "run",
                "-ti",
                "--rm",
                "--name",
                "test",
                "--network",
                "net1",
                "--ip",
                "10.0.0.2",
                "i"
            ]
        );
    }

    #[test]
    fn preserves_spaces_in_structured_field_values() {
        let mut vars = HashMap::new();
        vars.insert("msg".to_string(), "hello world".to_string());
        vars.insert("conf".to_string(), "/opt/my app/nginx.conf".to_string());
        let cfg = docker_cfg(
            r#"{
                "image": "i",
                "env": { "MSG": "{{msg}}" },
                "mounts": [ { "source": "{{conf}}" } ],
                "volumes": [ "{{msg}}:/data" ]
            }"#,
        );
        let out = cfg.build_command(&ctx(&vars)).unwrap().cmd;
        assert!(out.contains(&"MSG=hello world".to_string()));
        assert!(
            out.contains(&"type=bind,source=/opt/my app/nginx.conf,target=/opt/my app/nginx.conf,readonly".to_string())
        );
        assert!(out.contains(&"hello world:/data".to_string()));
    }

    #[test]
    fn shell_quotes_in_opts_and_args_are_parsed() {
        let vars: HashMap<String, String> = HashMap::new();
        let cfg = docker_cfg(r#"{ "image": "i", "opts": "--name 'hi nginx'", "args": "nginx -g 'daemon off;'" }"#);
        let out = cfg.build_command(&ctx(&vars)).unwrap().cmd;
        assert!(out.contains(&"hi nginx".to_string()));
        assert_eq!(&out[out.len() - 3..], &["nginx", "-g", "daemon off;"]);
    }
}
