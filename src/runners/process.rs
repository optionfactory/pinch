use crate::config::RunMode;
use crate::runners::{BuildOutput, BuildResult, RunBuilder, RunContext, parse_command_string};
use crate::vars::apply_vars;

impl RunBuilder for crate::config::ProcessRunConfig {
    fn build_command(&self, ctx: &RunContext) -> BuildResult {
        let expanded_cmd = apply_vars(self.cmd.trim(), ctx.vars, true);
        let cmd_vec = parse_command_string(&expanded_cmd, self.bash, ctx.name)?;
        Ok(BuildOutput {
            cmd: cmd_vec,
            run_mode: if ctx.background { RunMode::Spawn } else { RunMode::Exec },
        })
    }
}
