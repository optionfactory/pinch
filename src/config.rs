use serde::Deserialize;
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct DockerConfig {
    pub network: String,
    pub ip: String,
}

#[derive(Debug, Deserialize)]
pub struct RawAppConfig {
    pub title: Option<String>,
    pub vars: Option<HashMap<String, String>>,
    pub processes: Vec<RawProcessConfig>,
    pub logs_max_size: Option<usize>,
    pub auto_start: Option<bool>,
    pub auto_restart: Option<bool>,
    pub grace_period: Option<u64>,
    pub shell: Option<bool>,    
    pub docker_network: Option<String>, 
}

#[derive(Debug, Deserialize)]
pub struct RawProcessConfig {
    pub title: String,
    pub cmd: String,
    pub cwd: Option<String>,
    pub auto_start: Option<bool>,
    pub auto_restart: Option<bool>,
    pub grace_period: Option<u64>,
    pub shell: Option<bool>,    
    pub docker_network: Option<String>,
    pub docker_ip: Option<String>,
}

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub title: String,
    pub processes: Vec<ProcessConfig>,
    pub logs_max_size: Option<usize>,
}

#[derive(Debug, Clone)]
pub struct ProcessConfig {
    pub title: String,
    pub cmd: Vec<String>,
    pub cwd: Option<PathBuf>,
    pub auto_start: bool,
    pub auto_restart: bool,
    pub grace_period: u64,
}

impl RawAppConfig {
    pub fn prepare(self) -> AppConfig {
        let current_pwd = std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .to_string_lossy()
            .into_owned();

        let title = self.title.unwrap_or_else(|| "Process Supervisor".to_string());

        let prepared_processes = self.processes.into_iter().map(|mut raw| {
            raw.cmd = raw.cmd.replace("${PWD}", &current_pwd);
            if let Some(cwd) = raw.cwd.as_mut() {
                *cwd = cwd.replace("${PWD}", &current_pwd);
            }
            if let Some(ref vars_map) = self.vars {
                for (key, value) in vars_map {
                    let pattern = format!("${{{}}}", key);
                    raw.cmd = raw.cmd.replace(&pattern, value);
                    if let Some(cwd) = raw.cwd.as_mut() {
                        *cwd = cwd.replace(&pattern, value);
                    }
                }
            }

            let resolved_docker = match raw.docker_ip {
                Some(ip) => {
                    let network = raw.docker_network.clone()
                        .or_else(|| self.docker_network.clone())
                        .expect(&format!("Process '{}' has a docker_ip, but no docker_network was found locally or globally!", raw.title));
                    
                    Some(DockerConfig { network, ip })
                },
                None => None,
            };

            let use_shell = raw.shell.or(self.shell).unwrap_or(false);

            let final_cmd = if let Some(docker) = &resolved_docker {
                let sanitized_title: String = raw.title
                    .chars()
                    .filter(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_')
                    .collect();
                let container_name = format!("pinch-net-{}", sanitized_title.to_lowercase());

                let safe_name = shlex::try_quote(&container_name).expect("Null bytes in container name");
                let safe_net = shlex::try_quote(&docker.network).expect("Null bytes in network name");
                let safe_ip = shlex::try_quote(&docker.ip).expect("Null bytes in IP address");

                let inner_cmd = if use_shell {
                    format!("bash -c {}", shlex::try_quote(&raw.cmd).expect("Null bytes in command"))
                } else {
                    let tokens = shlex::split(&raw.cmd).unwrap_or_else(|| {
                        panic!("Failed to parse command (check for mismatched quotes): {}", raw.cmd)
                    });
                    shlex::try_join(tokens.iter().map(|s| s.as_str())).expect("Null bytes in command tokens")
                };

            let bash_script = format!(
                "NAME={name}; \
                NET={net}; \
                IP={ip}; \
                USER_NAME=$(logname 2>/dev/null || echo $USER); \
                if ! sudo --non-interactive /usr/bin/nsenter --version >/dev/null 2>&1; then \
                    echo -e '\\033[31m:: Error: Sudo access missing or requires password. ::\\033[0m'; \
                    echo '    edit sudoers or run:'; \
                    echo \"    echo \\\"$USER_NAME ALL=(ALL) NOPASSWD: /usr/bin/nsenter\\\" | sudo tee /etc/sudoers.d/${{USER_NAME}}_nsenter\"; \
                    exit 1; \
                fi; \
                echo -e \"\\033[32m:: Preparing Docker network holder ($NAME) ::\\033[0m\"; \
                docker rm -f \"$NAME\" >/dev/null 2>&1; \
                docker run -d --rm --name \"$NAME\" --network \"$NET\" --ip \"$IP\" optionfactory/sloth:226; \
                trap 'docker rm -f \"$NAME\" >/dev/null 2>&1' EXIT INT TERM; \
                PID=$(docker inspect -f '{{{{.State.Pid}}}}' \"$NAME\"); \
                echo -e '\\033[32m:: Entering namespace ::\\033[0m'; \
                sudo --non-interactive nsenter --net=/proc/$PID/ns/net {cmd}",
                name = safe_name,
                net = safe_net,
                ip = safe_ip,
                cmd = inner_cmd
            );

                vec!["bash".to_string(), "-c".to_string(), bash_script]
            } else if use_shell {
                vec!["bash".to_string(), "-c".to_string(), raw.cmd.clone()]
            } else {
                shlex::split(&raw.cmd).unwrap_or_else(|| {
                    panic!("Failed to parse command (check for mismatched quotes): {}", raw.cmd)
                })
            };

            ProcessConfig {
                title: raw.title,
                cmd: final_cmd,
                cwd: raw.cwd.map(PathBuf::from),
                auto_start: raw.auto_start.or(self.auto_start).unwrap_or(true),
                auto_restart: raw.auto_restart.or(self.auto_restart).unwrap_or(true),
                grace_period: raw.grace_period.or(self.grace_period).unwrap_or(3000),
            }
        }).collect();

        AppConfig { 
            title, 
            processes: prepared_processes, 
            logs_max_size: self.logs_max_size 
        }
    }
}