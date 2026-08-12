mod schema;

use std::collections::BTreeSet;

pub use schema::*;

impl PinchManifest {
    pub fn audit(&self) -> PinchAudit {
        let mut context_vars = crate::vars::builtin_vars();
        if let Some(vars) = &self.vars {
            context_vars.extend(vars.clone());
        }

        let mut containers = BTreeSet::new();
        if let Some(processes) = &self.processes {
            for proc in processes {
                if let RunManifest::Detailed(RunKind::Docker(ref docker_cfg)) = proc.run {
                    let resolved_image = crate::vars::apply_vars(&docker_cfg.image, &context_vars, false);
                    containers.insert(resolved_image);
                }
            }
        }

        PinchAudit {
            project: self.project.clone(),
            containers: containers.into_iter().collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn manifest(json: &str) -> PinchManifest {
        serde_json::from_str(json).expect("valid manifest fixture")
    }

    #[test]
    fn audit_extracts_docker_images_and_ignores_others() {
        let m = manifest(r#"{
            "schema_version": 1,
            "project": { "name": "acme" },
            "processes": [
                { "name": "db",  "run": { "type": "docker", "image": "postgres:16" } },
                { "name": "web", "run": "npm run dev" }
            ]
        }"#);
        let audit = m.audit();
        assert_eq!(audit.project.name, "acme");
        assert_eq!(audit.containers, vec!["postgres:16".to_string()]);
    }

    #[test]
    fn audit_deduplicates_identical_images() {
        let m = manifest(r#"{
            "schema_version": 1,
            "project": { "name": "acme" },
            "processes": [
                { "name": "db1", "run": { "type": "docker", "image": "postgres:16" } },
                { "name": "db2", "run": { "type": "docker", "image": "postgres:16" } }
            ]
        }"#);
        assert_eq!(m.audit().containers, vec!["postgres:16".to_string()]);
    }

    #[test]
    fn audit_expands_vars_in_image_names() {
        let m = manifest(r#"{
            "schema_version": 1,
            "project": { "name": "acme" },
            "vars": { "registry": "registry.example.com" },
            "processes": [
                { "name": "db", "run": { "type": "docker", "image": "{{registry}}/postgres:16" } }
            ]
        }"#);
        assert_eq!(m.audit().containers, vec!["registry.example.com/postgres:16".to_string()]);
    }

    #[test]
    fn audit_has_no_containers_when_none_defined() {
        let m = manifest(r#"{ "schema_version": 1, "project": { "name": "acme" } }"#);
        assert!(m.audit().containers.is_empty());
    }

    #[test]
    fn rejects_unknown_top_level_field() {
        let result: Result<PinchManifest, _> = serde_json::from_str(r#"{
            "schema_version": 1,
            "project": { "name": "acme" },
            "bogus_field": true
        }"#);
        assert!(result.is_err());
    }
}
