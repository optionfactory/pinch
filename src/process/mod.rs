pub mod panes;
pub mod ptys;

use crate::config::ProcessConfig;

pub fn build_std_command(cfg: &ProcessConfig) -> Result<std::process::Command, String> {
    if cfg.cmd.is_empty() {
        return Err(format!("Process command missing for: {}", cfg.title));
    }
    let mut cmd = std::process::Command::new(&cfg.cmd[0]);
    if cfg.cmd.len() > 1 {
        cmd.args(&cfg.cmd[1..]);
    }
    if let Some(ref cwd) = cfg.cwd {
        cmd.current_dir(cwd);
    }
    Ok(cmd)
}
