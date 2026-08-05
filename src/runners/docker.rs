use crate::config::RunMode;
use crate::runners::{BuildOutput, BuildResult, RunBuilder, RunContext};
use crate::vars::apply_vars;

impl RunBuilder for crate::config::DockerRunConfig {
    fn build_command(&self, ctx: &RunContext) -> BuildResult {
        let mode_flag = if ctx.background { "-d" } else { "-ti" };
        let mut cmd_str = format!("docker run {} --rm", mode_flag);
        if let Some(opts) = &self.opts {
            cmd_str.push(' ');
            cmd_str.push_str(opts.trim());
        }
        cmd_str.push(' ');
        cmd_str.push_str(self.image.trim());
        if let Some(args) = &self.args {
            cmd_str.push(' ');
            cmd_str.push_str(args.trim());
        }
        let expanded_cmd = apply_vars(&cmd_str, ctx.vars, false);
        let tokens = shlex::split(&expanded_cmd)
            .ok_or_else(|| format!("Failed to parse docker run command for '{}'", ctx.name))?;

        Ok(BuildOutput {
            cmd: tokens,
            run_mode: RunMode::Exec,
        })
    }
}
