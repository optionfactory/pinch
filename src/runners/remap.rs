use crate::vars::apply_vars;
use std::collections::HashMap;

/// When `remap_ids` is set, wrap an already-built command so it runs under
/// `docker-bluff`, which idmaps the mounted directories so the process and the
/// host each see the mounted files as their own. `remap_paths` names extra
/// directories for a generic command; a `docker run` command needs none, since
/// docker-bluff remaps its `-v`/`--mount` sources itself. When `remap_ids` is
/// absent the command is returned unchanged.
///
/// Any shell wrapping is already baked into `cmd` by the run builder and is
/// preserved untouched: it simply becomes the program docker-bluff execs after
/// the `--` separator.
pub fn wrap_with_docker_bluff(
    cmd: Vec<String>,
    remap_ids: Option<&[String]>,
    remap_paths: Option<&[String]>,
    vars: &HashMap<String, String>,
    name: &str,
) -> Result<Vec<String>, String> {
    let paths = remap_paths.unwrap_or(&[]);
    let ids = match remap_ids {
        Some(ids) if !ids.is_empty() => ids,
        Some(_) => {
            return Err(format!(
                "Process '{name}' has an empty `remap_ids`; add at least one swap (e.g. `me:0`)."
            ));
        }
        None => {
            if !paths.is_empty() {
                return Err(format!(
                    "Process '{name}' sets `remap_paths` but no `remap_ids`; paths cannot be remapped without an id swap (e.g. `me:0`)."
                ));
            }
            return Ok(cmd);
        }
    };

    let mut wrapped = vec!["docker-bluff".to_string()];
    for spec in ids {
        let expanded = apply_vars(spec.trim(), vars);
        validate_id_spec(&expanded).map_err(|e| format!("Process '{name}': {e}"))?;
        wrapped.push("--id".to_string());
        wrapped.push(expanded);
    }
    for dir in paths {
        let expanded = apply_vars(dir.trim(), vars);
        validate_path_spec(&expanded).map_err(|e| format!("Process '{name}': {e}"))?;
        wrapped.push("--map".to_string());
        wrapped.push(expanded);
    }
    wrapped.push("--".to_string());
    wrapped.extend(cmd);
    Ok(wrapped)
}

/// Reject a `remap_ids` swap that docker-bluff would reject at spawn time, so a
/// typo fails at config load instead. Mirrors docker-bluff's `--id
/// [u:|g:]DISK:SEEN` grammar, where DISK/SEEN is `me` or a number.
fn validate_id_spec(spec: &str) -> Result<(), String> {
    let (disk, seen) = match spec.split(':').collect::<Vec<_>>().as_slice() {
        [d, s] => (*d, *s),
        ["u", d, s] | ["g", d, s] => (*d, *s),
        _ => {
            return Err(format!(
                "invalid remap_ids entry {spec:?}: expected [u:|g:]DISK:SEEN (e.g. `me:0`, `u:0:33`)"
            ));
        }
    };
    for tok in [disk, seen] {
        if tok != "me" && tok.parse::<u32>().is_err() {
            return Err(format!(
                "invalid remap_ids entry {spec:?}: id {tok:?} is not a number or `me`"
            ));
        }
    }
    Ok(())
}

/// Reject a `remap_paths` entry docker-bluff would reject: its `--map SRC[:DST]`
/// requires non-empty, absolute paths.
fn validate_path_spec(spec: &str) -> Result<(), String> {
    let (src, dst) = spec.split_once(':').unwrap_or((spec, spec));
    if src.is_empty() || dst.is_empty() {
        return Err(format!(
            "invalid remap_paths entry {spec:?}: expected SRC[:DST] with non-empty paths"
        ));
    }
    if !src.starts_with('/') || !dst.starts_with('/') {
        return Err(format!("invalid remap_paths entry {spec:?}: paths must be absolute"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn vars(pairs: &[(&str, &str)]) -> HashMap<String, String> {
        pairs.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect()
    }

    fn list(items: &[&str]) -> Vec<String> {
        items.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn without_remap_ids_the_command_is_returned_unchanged() {
        let cmd = vec!["make".to_string()];
        assert_eq!(
            wrap_with_docker_bluff(cmd.clone(), None, None, &vars(&[]), "p").unwrap(),
            cmd
        );
    }

    #[test]
    fn empty_remap_ids_is_rejected() {
        let cmd = vec!["make".to_string()];
        assert!(wrap_with_docker_bluff(cmd, Some(&[]), None, &vars(&[]), "p").is_err());
    }

    #[test]
    fn remap_paths_without_remap_ids_is_rejected() {
        let cmd = vec!["make".to_string()];
        let paths = list(&["/srv"]);
        assert!(wrap_with_docker_bluff(cmd, None, Some(&paths), &vars(&[]), "p").is_err());
    }

    #[test]
    fn remap_ids_without_paths_wraps_with_only_id_flags() {
        // The docker-run case: docker-bluff remaps the run's own -v sources, so
        // no --map is emitted.
        let ids = list(&["me:0"]);
        let cmd = vec!["docker".to_string(), "run".to_string(), "alpine".to_string()];
        let out = wrap_with_docker_bluff(cmd, Some(&ids), None, &vars(&[]), "d").expect("wraps");
        assert_eq!(
            out,
            vec!["docker-bluff", "--id", "me:0", "--", "docker", "run", "alpine"]
        );
    }

    #[test]
    fn prefixes_docker_bluff_with_ids_and_paths_then_the_command() {
        let ids = list(&["me:0"]);
        let paths = list(&["/srv/data"]);
        let out = wrap_with_docker_bluff(
            vec!["make".to_string(), "build".to_string()],
            Some(&ids),
            Some(&paths),
            &vars(&[]),
            "build",
        )
        .expect("wraps");
        assert_eq!(
            out,
            vec![
                "docker-bluff",
                "--id",
                "me:0",
                "--map",
                "/srv/data",
                "--",
                "make",
                "build"
            ]
        );
    }

    #[test]
    fn preserves_a_shell_wrapped_command_after_the_separator() {
        // The run builder already produced `bash -c "..."`; wrapping must not
        // re-parse or drop it.
        let cmd = vec!["bash".to_string(), "-c".to_string(), "cargo build && ls".to_string()];
        let ids = list(&["me:0"]);
        let paths = list(&["/w"]);
        let out = wrap_with_docker_bluff(cmd, Some(&ids), Some(&paths), &vars(&[]), "build").expect("wraps");
        assert_eq!(
            out,
            vec![
                "docker-bluff",
                "--id",
                "me:0",
                "--map",
                "/w",
                "--",
                "bash",
                "-c",
                "cargo build && ls"
            ]
        );
    }

    #[test]
    fn several_id_swaps_and_paths_each_get_their_own_flag() {
        let ids = list(&["u:0:33", "g:0:33"]);
        let paths = list(&["/a", "/b:/c"]);
        let out =
            wrap_with_docker_bluff(vec!["true".to_string()], Some(&ids), Some(&paths), &vars(&[]), "p").expect("wraps");
        assert_eq!(
            out,
            vec![
                "docker-bluff",
                "--id",
                "u:0:33",
                "--id",
                "g:0:33",
                "--map",
                "/a",
                "--map",
                "/b:/c",
                "--",
                "true"
            ]
        );
    }

    #[test]
    fn malformed_remap_ids_are_rejected_at_config_time() {
        let cmd = vec!["make".to_string()];
        // no colon (the "me 0" typo), and a non-numeric/non-`me` id
        assert!(wrap_with_docker_bluff(cmd.clone(), Some(&list(&["me 0"])), None, &vars(&[]), "p").is_err());
        assert!(wrap_with_docker_bluff(cmd, Some(&list(&["x:0"])), None, &vars(&[]), "p").is_err());
    }

    #[test]
    fn relative_remap_paths_are_rejected_at_config_time() {
        let cmd = vec!["make".to_string()];
        let ids = list(&["me:0"]);
        assert!(wrap_with_docker_bluff(cmd, Some(&ids), Some(&list(&["relative/dir"])), &vars(&[]), "p").is_err());
    }

    #[test]
    fn valid_id_forms_are_accepted() {
        for spec in ["me:0", "0:me", "1000:0", "u:0:33", "g:0:33", "me:me"] {
            assert!(super::validate_id_spec(spec).is_ok(), "{spec} should be valid");
        }
    }

    #[test]
    fn ids_and_paths_expand_variables() {
        let v = vars(&[("uid", "1000"), ("pwd", "/home/me/project")]);
        let ids = list(&["{{ uid }}:0"]);
        let paths = list(&["{{ pwd }}"]);
        let out = wrap_with_docker_bluff(vec!["ls".to_string()], Some(&ids), Some(&paths), &v, "p").expect("wraps");
        assert_eq!(
            out,
            vec![
                "docker-bluff",
                "--id",
                "1000:0",
                "--map",
                "/home/me/project",
                "--",
                "ls"
            ]
        );
    }
}
