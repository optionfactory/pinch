use crate::config::RunMode;
use crate::runners::{BuildOutput, BuildResult, RunBuilder, RunContext, parse_command_string};
use crate::vars::apply_vars;

impl RunBuilder for crate::config::DockerIntrudeRunConfig {
    fn build_command(&self, ctx: &RunContext) -> BuildResult {
        let network = self
            .network
            .as_ref()
            .or(ctx.default_docker_network)
            .ok_or_else(|| {
                format!(
                    "Process '{}' has run type 'docker-intrude', but no 'network' is specified and there isn't exactly one network defined in 'docker_networks' to default to.",
                    ctx.name
                )
            })?;
        let _net_config = ctx.defined_networks.get(network).ok_or_else(|| {
            format!(
                "Process '{}' references docker network '{}' which is not defined in the root 'docker_networks' map.",
                ctx.name, network
            )
        })?;

        let sanitized_name: String = ctx
            .name
            .replace('_', "-")
            .chars()
            .filter(|c| c.is_ascii_alphanumeric() || *c == '-')
            .collect();
        let container_name = format!("pinch-ns-{}", sanitized_name.to_lowercase());
        let actual_net_name = apply_vars(network, ctx.vars, false);
        let actual_ip = apply_vars(&self.ip, ctx.vars, false);

        let mut cmd_vec = vec![
            "docker-intrude".to_string(),
            "--name".to_string(),
            container_name,
            "--net".to_string(),
            actual_net_name,
            "--ip".to_string(),
            actual_ip,
            "--".to_string(),
        ];
        let expanded_cmd = apply_vars(self.cmd.trim(), ctx.vars, true);
        let tokens = parse_command_string(&expanded_cmd, self.shell.or(ctx.global_shell), ctx.name)?;
        cmd_vec.extend(tokens);

        Ok(BuildOutput {
            cmd: cmd_vec,
            run_mode: if ctx.background { RunMode::Spawn } else { RunMode::Exec },
        })
    }
}
