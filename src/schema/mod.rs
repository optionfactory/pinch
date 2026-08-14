mod schema;

pub use schema::*;

#[cfg(test)]
mod tests {
    use super::*;

    fn manifest(json: &str) -> PinchManifest {
        serde_json::from_str(json).expect("valid manifest fixture")
    }

    #[test]
    fn parses_minimal_manifest() {
        let m = manifest(r#"{ "schema_version": 1, "name": "acme" }"#);
        assert_eq!(m.name, "acme");
        assert!(m.processes.is_none());
    }

    #[test]
    fn parses_processes() {
        let m = manifest(
            r#"{
            "schema_version": 1,
            "name": "acme",
            "processes": [
                { "name": "db",  "run": { "type": "docker", "image": "postgres:16" } },
                { "name": "web", "run": "npm run dev" }
            ]
        }"#,
        );
        assert_eq!(m.processes.as_ref().unwrap().len(), 2);
    }

    #[test]
    fn rejects_unknown_top_level_field() {
        let result: Result<PinchManifest, _> = serde_json::from_str(
            r#"{
            "schema_version": 1,
            "name": "acme",
            "bogus_field": true
        }"#,
        );
        assert!(result.is_err());
    }

    #[test]
    fn rejects_legacy_project_block() {
        let result: Result<PinchManifest, _> = serde_json::from_str(
            r#"{
            "schema_version": 1,
            "project": { "name": "acme" }
        }"#,
        );
        assert!(result.is_err());
    }
}
