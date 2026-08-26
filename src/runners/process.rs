use crate::config::RunMode;
use crate::runners::{BuildOutput, BuildResult, RunBuilder, RunContext, parse_command_string};
use crate::vars::apply_vars;

impl RunBuilder for crate::config::ProcessRunConfig {
    fn build_command(&self, ctx: &RunContext) -> BuildResult {
        let expanded_cmd = apply_vars(self.cmd.trim(), ctx.vars);
        let cmd_vec = parse_command_string(&expanded_cmd, self.shell.or(ctx.global_shell), ctx.name)?;
        Ok(BuildOutput {
            cmd: cmd_vec,
            run_mode: if ctx.background { RunMode::Spawn } else { RunMode::Exec },
            container: None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{ProcessRunConfig, RunKind, RunManifest, WrappingShell};
    use std::collections::HashMap;

    fn vars(pairs: &[(&str, &str)]) -> HashMap<String, String> {
        pairs.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect()
    }

    fn build(cmd: &str, shell: Option<WrappingShell>, vars: &HashMap<String, String>) -> Vec<String> {
        let networks = HashMap::new();
        let ctx = RunContext {
            name: "test",
            vars,
            global_shell: None,
            default_docker_network: None,
            defined_networks: &networks,
            background: false,
        };
        let run = RunManifest::Detailed(RunKind::Process(ProcessRunConfig {
            shell,
            cmd: cmd.to_string(),
        }));
        run.build_command(&ctx).expect("command builds").cmd
    }

    #[test]
    fn var_with_spaces_expands_to_several_argv_tokens() {
        // README example: `pinch -o flags:"-c 4"` must yield separate flags.
        let v = vars(&[("target", "8.8.8.8"), ("flags", "-c 10")]);
        assert_eq!(build("ping {{ target }} {{ flags }}", None, &v), vec!["ping", "8.8.8.8", "-c", "10"]);
    }

    #[test]
    fn quoting_the_placeholder_keeps_value_as_one_token() {
        let v = vars(&[("dir", "/path with spaces")]);
        assert_eq!(build(r#"ls "{{ dir }}""#, None, &v), vec!["ls", "/path with spaces"]);
        assert_eq!(build("ls '{{ dir }}'", None, &v), vec!["ls", "/path with spaces"]);
    }

    #[test]
    fn quotes_inside_var_value_are_parsed_like_inline_quotes() {
        // Mirrors the `spring_debug_opts` var used by real manifests: the single
        // quotes must be consumed by the parser, not passed to the JVM.
        let v = vars(&[(
            "spring_debug_opts",
            "-Dspring-boot.run.jvmArguments='-agentlib:jdwp=transport=dt_socket,server=y,suspend=n,address=*:8787'",
        )]);
        assert_eq!(
            build("mvn spring-boot:run {{ spring_debug_opts }}", None, &v),
            vec![
                "mvn",
                "spring-boot:run",
                "-Dspring-boot.run.jvmArguments=-agentlib:jdwp=transport=dt_socket,server=y,suspend=n,address=*:8787",
            ]
        );
    }

    #[test]
    fn var_containing_double_quote_no_longer_breaks_parsing() {
        let v = vars(&[("q", r#"say\"hi"#)]);
        assert_eq!(build("echo {{ q }}", None, &v), vec!["echo", r#"say"hi"#]);
    }

    #[test]
    fn shell_mode_passes_expanded_text_verbatim() {
        let v = vars(&[("flags", "-c 10")]);
        assert_eq!(
            build("ping {{ flags }} 8.8.8.8", Some(WrappingShell::Bash), &v),
            vec!["bash", "-c", "ping -c 10 8.8.8.8"]
        );
    }

    #[test]
    fn plain_processes_have_no_container_reference() {
        let v = vars(&[]);
        let networks = HashMap::new();
        let ctx = RunContext {
            name: "test",
            vars: &v,
            global_shell: None,
            default_docker_network: None,
            defined_networks: &networks,
            background: false,
        };
        let run = RunManifest::Shorthand("true".to_string());
        assert_eq!(run.build_command(&ctx).unwrap().container, None);
    }

    #[test]
    fn unknown_var_is_left_verbatim() {
        let v = vars(&[]);
        assert_eq!(build("echo {{ nope }}", None, &v), vec!["echo", "{{", "nope", "}}"]);
    }
}
