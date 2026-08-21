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
    fn parses_docker_run_container_block() {
        let m = manifest(
            r#"{
            "schema_version": 1,
            "name": "acme",
            "processes": [
                {
                    "name": "nginx",
                    "run": {
                        "type": "docker",
                        "engine": "podman",
                        "image": "nginx:alpine",
                        "network": "hi",
                        "ip": "172.18.23.10",
                        "env": { "TZ": "Europe/Rome" },
                        "publish": ["0.0.0.0:8888:8888"],
                        "mounts": [
                            {
                                "type": "bind",
                                "source": "/opt/nginx.conf",
                                "target": "/etc/nginx/nginx.conf",
                                "readonly": true,
                                "opts": "bind-propagation=rshared"
                            }
                        ],
                        "volumes": ["mydata:/var/lib/app"],
                        "opts": "--name hi-nginx",
                        "args": "nginx -g 'daemon off;'"
                    }
                }
            ]
        }"#,
        );
        let run = &m.processes.as_ref().unwrap()[0].run;
        let RunManifest::Detailed(RunKind::Docker(cfg)) = run else {
            panic!("expected docker run config");
        };
        assert_eq!(cfg.engine, Some(ContainerEngine::Podman));
        assert_eq!(cfg.image, "nginx:alpine");
        assert_eq!(cfg.network.as_deref(), Some("hi"));
        assert_eq!(cfg.ip.as_deref(), Some("172.18.23.10"));
        assert_eq!(cfg.env.as_ref().unwrap()["TZ"], "Europe/Rome");
        assert_eq!(cfg.publish.as_ref().unwrap(), &vec!["0.0.0.0:8888:8888"]);
        let mount = &cfg.mounts.as_ref().unwrap()[0];
        assert_eq!(mount.mount_type.as_deref(), Some("bind"));
        assert_eq!(mount.source, "/opt/nginx.conf");
        assert_eq!(mount.target.as_deref(), Some("/etc/nginx/nginx.conf"));
        assert_eq!(mount.readonly, Some(true));
        assert_eq!(cfg.volumes.as_ref().unwrap(), &vec!["mydata:/var/lib/app"]);
        assert_eq!(cfg.opts.as_deref(), Some("--name hi-nginx"));
        assert_eq!(cfg.args.as_deref(), Some("nginx -g 'daemon off;'"));
    }

    #[test]
    fn rejects_unknown_mount_field() {
        let result: Result<PinchManifest, _> = serde_json::from_str(
            r#"{
            "schema_version": 1,
            "name": "acme",
            "processes": [
                { "name": "db", "run": { "type": "docker", "image": "postgres:16", "bogus": true } }
            ]
        }"#,
        );
        assert!(result.is_err());
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
