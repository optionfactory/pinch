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