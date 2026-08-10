use schemars::{JsonSchema, Schema, SchemaGenerator};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[doc = "Root configuration manifest for a Pinch project (`pinch.yaml`)."]
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PinchManifest {
    #[doc = "Explicit schema version for manifest compatibility (must be 1)."]
    #[schemars(schema_with = "schema_version_schema")]
    pub schema_version: u32,

    #[doc = "Core project metadata, governance attributes, and architecture classification."]
    pub project: ProjectManifest,

    #[doc = "Custom configuration variables available for string expansion (e.g., `{{var_name}}`)."]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "HashMap<String, String>")]
    pub vars: Option<HashMap<String, String>>,

    #[doc = "List of supervised processes, containers, and background services to execute."]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "Vec<ProcessManifest>")]
    pub processes: Option<Vec<ProcessManifest>>,

    #[doc = "Maximum number of log lines to retain in memory per process pane."]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "usize")]
    pub logs_max_size: Option<usize>,

    #[doc = "Whether processes start automatically upon launching the supervisor (default: `true`)."]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "bool")]
    pub auto_start: Option<bool>,

    #[doc = "Whether processes restart automatically if they exit or crash (default: `true`)."]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "bool")]
    pub auto_restart: Option<bool>,

    #[doc = "Delay in milliseconds before automatically restarting a process after an exit (default: `3000`)."]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "u64")]
    pub grace_period: Option<u64>,

    #[doc = "If present, executes commands using the configed shell '-c' option globally."]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "WrappingShell")]
    pub shell: Option<WrappingShell>,

    #[doc = "Custom Docker bridge networks managed and initialized by Pinch."]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "HashMap<String, DockerNetworkConfig>")]
    pub docker_networks: Option<HashMap<String, DockerNetworkConfig>>,

    #[doc = "Debounce delay in milliseconds when watching files for auto-restarting (default: `800`)."]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "u64")]
    pub watch_settle_time_ms: Option<u64>,

    #[doc = "Progressive edge-carving layout rules defining how process panes are arranged."]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "Vec<LayoutNode>")]
    pub layout: Option<Vec<LayoutNode>>,
}

#[doc = "Shell to be used to run commands."]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
#[serde(deny_unknown_fields)]
pub enum WrappingShell {
    Bash,
    Zsh,
    Fish,
}

#[doc = "High-level project identification, governance, and architecture classification."]
#[derive(Debug, Deserialize, Serialize, Clone, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ProjectManifest {
    pub name: String,

    #[serde(rename = "type", default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "ProjectType")]
    pub project_type: Option<ProjectType>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "LifecycleType")]
    pub lifecycle: Option<LifecycleType>,

    #[doc = "Operational service tier dictating on-call priority and internal incident response targets."]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "ServiceTier")]
    pub tier: Option<ServiceTier>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "Vec<AuthType>")]
    pub authentication: Option<Vec<AuthType>>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "Vec<Sensitivity>")]
    pub sensitivity: Option<Vec<Sensitivity>>,

    #[doc = "Regulatory compliance and resilience framework mappings."]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "ComplianceManifest")]
    pub compliance: Option<ComplianceManifest>,

    #[doc = "Internal project stewards and domain experts to contact for reference and context."]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stewards: Option<Vec<String>>,

    #[doc = "Company or legal entity that commissioned or paid for the project."]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub commissioner: Option<String>,

    #[doc = "Channel partner, agency, or system integrator brokering the project relationship."]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub channel: Option<String>,

    #[doc = "Environment-specific deployment configurations."]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub environments: Option<Vec<EnvironmentManifest>>,
}

#[doc = "Primary architectural role of the project."]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
#[serde(deny_unknown_fields)]
pub enum ProjectType {
    #[doc = "Shared code dependency imported by other projects."]
    Library,
    #[doc = "Long-running server, API, or backend daemon."]
    Service,
    #[doc = "CLI utility or internal developer tool."]
    Tool,
    #[doc = "Short-lived batch process, cron, or CI/CD script."]
    Job,
    #[doc = "Infrastructure-as-Code or IAC libraries (e.g., Terraform, OpenTofu, Pulumi, or Ansible blueprints)."]
    Infrastructure,
}

#[doc = "Operational maintenance status of the repository."]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
#[serde(deny_unknown_fields)]
pub enum LifecycleType {
    #[doc = "Actively developed and supported in production."]
    Active,
    #[doc = "Scheduled for decommissioning; do not add new dependencies."]
    Deprecated,
    #[doc = "Decommissioned, no longer actively developed, deployed or running."]
    EndOfLife,
    #[doc = "In production but receiving only critical bug/security fixes."]
    Maintenance,
    #[doc = "Experimental proof-of-concept; no production SLA."]
    Prototype,
}

#[doc = "Operational service tier dictating on-call priority and internal incident response targets."]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
#[serde(deny_unknown_fields)]
pub enum ServiceTier {
    #[doc = "Tier 1: Critical production path (24/7 on-call, immediate response SLO)."]
    Tier1,
    #[doc = "Tier 2: Core supporting functionality (business hours support, rapid degradation fallback)."]
    Tier2,
    #[doc = "Tier 3: Internal developer tools, async batch jobs, or non-critical support utilities."]
    Tier3,
    #[doc = "Tier 4: Experimental prototypes, sandbox playgrounds, or best-effort utilities."]
    Tier4,
}

#[doc = "Primary authentication mechanism for ingress requests."]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
#[serde(deny_unknown_fields)]
pub enum AuthType {
    #[doc = "Identity provider abstraction."]
    Idp,
    #[doc = "OpenID Connect (OIDC) identity authentication layer."]
    Oidc,
    #[doc = "Generic OAuth 2.0 framework."]
    Oauth2,
    #[doc = "Generic SAML 2.0 Web SSO."]
    Saml,
    #[doc = "API Key header or query parameter validation."]
    ApiKey,
    #[doc = "Stateless JSON Web Token (JWT) signature validation."]
    Jwt,
    #[doc = "Mutual TLS client certificate authentication."]
    Mtls,
    #[doc = "HMAC request signature validation (e.g., webhooks)."]
    Hmac,
    #[doc = "Stateful cookie or HTTP session authentication."]
    Session,
    #[doc = "Custom bearer token or generic token authentication."]
    Token,
    #[doc = "HTTP Basic authentication (legacy/internal)."]
    Basic,
    #[doc = "Passkey authentication."]
    Passkey,
    #[doc = "LDAP authentication."]
    Ldap,
    #[doc = "Custom HTML form authentication."]
    Form,
    #[doc = "Explicitly public endpoint accepting unauthenticated requests (e.g., public API or landing page)."]
    Unauthenticated,
    #[doc = "Authentication is not applicable (e.g., offline CLI utility, library, or background job)."]
    NotApplicable,
}

#[doc = "Sensitive data classifications handled by the service."]
#[doc = ""]
#[doc = "### Data Classification Hierarchy"]
#[doc = "* **Public** → **Internal** → **Confidential** → **Restricted**"]
#[doc = ""]
#[doc = "### Regulatory Standards Covered"]
#[doc = "* **GDPR / CCPA / BIPA:** `Pii`, `Spi`, `Biometric`"]
#[doc = "* **PCI-DSS:** `Pci`"]
#[doc = "* **SOX:** `Financial`"]
#[doc = "* **HIPAA:** `Phi`"]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
#[serde(deny_unknown_fields)]
pub enum Sensitivity {
    #[doc = "Non-sensitive data safe for public distribution."]
    Public,
    #[doc = "Standard internal business data."]
    Internal,
    #[doc = "Restricted company intellectual property or business trade secrets."]
    Confidential,
    #[doc = "Maximum security assets requiring strict isolation (e.g., root credentials, master keys)."]
    Restricted,
    #[doc = "Personally Identifiable Information (names, emails, addresses)."]
    Pii,
    #[doc = "Sensitive PII / Special Category Data (genetics, political/religious beliefs under GDPR Art. 9)."]
    Spi,
    #[doc = "Biometric identification data (fingerprints, facial recognition, voice signatures)."]
    Biometric,
    #[doc = "Payment Card Industry data (credit cards, billing details, PANs)."]
    Pci,
    #[doc = "Non-PCI financial records (SOX audit trails, payroll, company accounting, bank accounts)."]
    Financial,
    #[doc = "Protected Health Information (medical records, insurance claims, health data)."]
    Phi,
}

#[doc = "Regulatory compliance and cybersecurity resilience specifications."]
#[derive(Debug, Deserialize, Serialize, Clone, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ComplianceManifest {
    #[doc = "DORA (Digital Operational Resilience Act) criticality tag."]
    #[doc = "Applicable if your project serves the EU financial sector or acts as an ICT third-party provider to financial entities."]
    pub dora: DoraCriticality,

    #[doc = "EU Cyber Resilience Act (CRA) product classification tier."]
    #[doc = "Applicable if your software product contains digital elements and is sold or distributed within the EU."]
    pub cra: CraClass,

    #[doc = "EU NIS2 Directive criticality classification."]
    #[doc = "Applicable to non-financial critical and important infrastructure operating in the EU (e.g., energy, health, SaaS, MSPs)."]
    pub nis2: Nis2Category,

    #[doc = "EU AI Act risk classification tier."]
    #[doc = "Applicable to systems integrating AI/ML models, LLMs, or automated inference."]
    pub ai_act: AiActClass,

    #[doc = "GDPR data protection role."]
    #[doc = "Applicable to any project that processes, stores, or transmits EU/EEA personal data."]
    pub gdpr: GdprRole,

    #[doc = "Whether personal data is processed or stored strictly within the EU/EEA."]
    pub data_residency: DataResidency,

    #[doc = "Italian Garante Privacy 'Amministratore di Sistema' (AdS) compliance block."]
    #[doc = "Applicable to projects falling under the Italian Provvedimento Garante AdS for tracking privileged IT and database administrators."]
    pub ads: AdsResponsibility,
}

#[doc = "EU NIS2 Directive classification tier for cybersecurity resilience."]
#[doc = "Applicable to non-financial critical and important infrastructure operating in the EU (e.g., energy, health, SaaS, MSPs)."]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
#[serde(deny_unknown_fields)]
pub enum Nis2Category {
    #[doc = "Essential Entity (Soggetti Essenziali): Energy, transport, banking, health, digital infrastructure."]
    EssentialEntity,
    #[doc = "Important Entity (Soggetti Importanti): Postal, waste, chemicals, food, digital providers (SaaS/marketplaces)."]
    ImportantEntity,
    #[doc = "Not in scope of NIS2 requirements."]
    OutOfScope,
    #[doc = "Applicability has not yet been assessed."]
    PendingAssessment,
}

#[doc = "EU AI Act risk classification tier."]
#[doc = "Applicable to systems integrating AI/ML models, LLMs, or automated inference."]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
#[serde(deny_unknown_fields)]
pub enum AiActClass {
    #[doc = "High-Risk AI System (biometrics, critical infrastructure, employment, credit scoring)."]
    HighRisk,
    #[doc = "General Purpose AI (GPAI) model or foundation model integration."]
    GeneralPurposeAi,
    #[doc = "Limited Risk AI System (requires transparency disclosures, e.g., chatbots/AI assistants)."]
    LimitedRisk,
    #[doc = "Minimal or no risk AI system."]
    MinimalRisk,
    #[doc = "Not in scope of AI Act requirements."]
    OutOfScope,
    #[doc = "Applicability has not yet been assessed."]
    PendingAssessment,
}

#[doc = "Geographic data storage and processing boundary for compliance."]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
#[serde(deny_unknown_fields)]
pub enum DataResidency {
    #[doc = "Data is stored and processed strictly within European Union member states."]
    Eu,
    #[doc = "Data is stored and processed within the European Economic Area (EU + Iceland, Liechtenstein, Norway)."]
    Eea,
    #[doc = "Data is stored or processed in the US."]
    Us,
    #[doc = "Data is stored or processed globally across international regions."]
    Global,
    #[doc = "No data is stored."]
    NotApplicable,
    #[doc = "Applicability has not yet been assessed."]
    PendingAssessment,
}

#[doc = "Legal processing role under GDPR (Art. 4)."]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
#[serde(deny_unknown_fields)]
pub enum GdprRole {
    #[doc = "Data Controller (Titolare del trattamento) - determines purposes and means of processing."]
    Controller,
    #[doc = "Data Processor (Responsabile del trattamento) - processes data on behalf of a Controller."]
    Processor,
    #[doc = "Sub-processor (Sub-responsabile) - third-party processor engaged by a Processor."]
    SubProcessor,
    #[doc = "No personal data processed."]
    None,
    #[doc = "Not in scope of GRPR requirements."]
    OutOfScope,
    #[doc = "Applicability has not yet been assessed."]
    PendingAssessment,
}

#[doc = "DORA (Digital Operational Resilience Act) operational criticality tag."]
#[doc = "Applicable if your project serves the EU financial sector or acts as an ICT third-party provider to financial entities."]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
#[serde(deny_unknown_fields)]
pub enum DoraCriticality {
    #[doc = "Supports a Critical or Important Function (CIF) subject to strict RTO/RPO SLAs."]
    CifSupported,
    #[doc = "Non-critical ICT supporting service or internal developer utility."]
    NonCritical,
    #[doc = "Not in scope of DORA requirements."]
    OutOfScope,
    #[doc = "Applicability has not yet been assessed."]
    PendingAssessment,
}

#[doc = "EU Cyber Resilience Act (CRA) product classification tier for cybersecurity compliance."]
#[doc = "Applicable if your software product contains digital elements and is sold or distributed within the EU."]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
#[serde(deny_unknown_fields)]
pub enum CraClass {
    #[doc = "Default category: Standard software product with digital elements."]
    Default,
    #[doc = "Important Class I: Identity management, password managers, VPNs, network monitors."]
    ImportantClass1,
    #[doc = "Important Class II: Hypervisors, container runtimes, firewalls, IDS/IPS."]
    ImportantClass2,
    #[doc = "Critical: Smartcards, hardware security modules, and core cryptographic hardware/software."]
    Critical,
    #[doc = "Not in scope of CRA requirements."]
    OutOfScope,
    #[doc = "Applicability has not yet been assessed."]
    PendingAssessment,
}

#[doc = "System Administrator (AdS) operational responsibility boundary."]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
#[serde(deny_unknown_fields)]
pub enum AdsResponsibility {
    #[doc = "Our organization directly manages the system/infrastructure as AdS."]
    Internal,
    #[doc = "An external Managed Service Provider (MSP) / Vendor holds AdS duties."]
    ExternalProcessor,
    #[doc = "The client / customer holds AdS responsibility for their own environment."]
    ClientManaged,
    #[doc = "Status has not yet been assessed."]
    PendingAssessment,
    #[doc = "Not in scope of ADS requirements."]
    OutOfScope,
    #[doc = "Pending formal designation letters ('Lettera di Nomina') to be executed."]
    PendingNomination,
}

#[doc = "Environment-specific deployment configuration."]
#[derive(Debug, Deserialize, Serialize, Clone, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EnvironmentManifest {
    #[doc = "Target deployment environment."]
    #[schemars(schema_with = "identifier_schema")]
    pub name: String,

    #[doc = "Target deployment environment type."]
    #[serde(rename = "type")]
    pub environment_type: EnvironmentType,

    #[doc = "Underlying deployment platform (cloud, on-premises, edge)."]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "PlatformType")]
    pub platform: Option<PlatformType>,

    #[doc = "Architectural stack layers owned and maintained directly by our organization."]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "Vec<StackLayer>")]
    pub ownership: Option<Vec<StackLayer>>,

    #[doc = "Network ingress reachability for this specific environment."]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "ExposureType")]
    pub ingress: Option<ExposureType>,

    #[doc = "Network management reachability for this specific environment."]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "ExposureType")]
    pub management: Option<ExposureType>,

    #[doc = "DNS management and ownership origin."]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "DnsManagement")]
    pub dns: Option<DnsManagement>,

    #[doc = "List of domain names assigned to this environment."]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub domains: Option<Vec<String>>,

    #[doc = "TLS certificate provisioning and renewal mechanism."]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "CertManagement")]
    pub certificates: Option<CertManagement>,
}

#[doc = "Supported deployment environment types."]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
#[serde(deny_unknown_fields)]
pub enum EnvironmentType {
    Local,
    Development,
    Preview,
    Testing,
    Qa,
    Uat,
    Sandbox,
    PreProduction,
    Staging,
    Demo,
    Performance,
    Shadow,
    Production,
    DisasterRecovery,
}

#[doc = "Underlying deployment platform or infrastructure substrate."]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
#[serde(deny_unknown_fields)]
pub enum PlatformType {
    Cloud,
    CloudAws,
    CloudAzure,
    CloudGcp,
    CloudHetzner,
    Datacenter,
    OnPremises,
    Colocation,
    Hybrid,
    Edge,
    DevNetwork,
}

#[doc = "Architectural stack layer managed directly by our organization."]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
#[serde(deny_unknown_fields)]
pub enum StackLayer {
    Infrastructure,
    OperatingSystem,
    Services,
    Applications,
}

#[doc = "Network ingress reachability for the project."]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
#[serde(deny_unknown_fields)]
pub enum ExposureType {
    Local,
    RestrictedVpn,
    RestrictedIp,
    RestrictedPam,
    Internet,
    None,
}

#[doc = "Domain management and ownership origin."]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
#[serde(deny_unknown_fields)]
pub enum DnsManagement {
    Managed,
    External,
}

#[doc = "TLS certificate provisioning and renewal mechanism."]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
#[serde(deny_unknown_fields)]
pub enum CertManagement {
    ManagedAcmeDns01,
    ManagedAcmeHttp01,
    ManagedAutoRenew,
    ManagedManual,
    ThirdPartyProvided,
    ThirdPartyManaged,
    NotApplicable,
}

#[doc = "Configuration for an individual supervised process or container."]
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ProcessManifest {
    #[doc = "Name of the process."]
    #[schemars(schema_with = "identifier_schema")]
    pub name: String,

    #[doc = "Display name of the process pane in the TUI dashboard (defaults to name)."]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "String")]
    pub title: Option<String>,

    #[doc = "Execution definition specifying how the command or container is spawned."]
    pub run: RunManifest,

    #[doc = "Working directory for process execution (supports variable expansion like `{{pwd}}`)."]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "String")]
    pub cwd: Option<String>,

    #[doc = "Optional web URL or link associated with this process (clickable in the TUI header)."]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "String")]
    pub link: Option<String>,

    #[doc = "List of file or directory paths that trigger an automatic restart when modified."]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "Vec<String>")]
    pub watch: Option<Vec<String>>,

    #[doc = "Process-specific debounce delay in milliseconds for file watching."]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "u64")]
    pub watch_settle_time_ms: Option<u64>,

    #[doc = "Display mode for the process output in the TUI dashboard (`log` or `tui`)."]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "PaneMode")]
    pub mode: Option<PaneMode>,

    #[doc = "Override global auto-start default for this specific process."]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "bool")]
    pub auto_start: Option<bool>,

    #[doc = "Override global auto-restart default for this specific process."]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "bool")]
    pub auto_restart: Option<bool>,

    #[doc = "Override global grace period delay in milliseconds for this specific process."]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "u64")]
    pub grace_period: Option<u64>,
}

#[doc = "How the process command is specified in YAML."]
#[derive(Debug, Deserialize, Serialize, Clone, JsonSchema)]
#[serde(untagged)]
#[serde(deny_unknown_fields)]
pub enum RunManifest {
    #[doc = "Simple shell command string executed as a local OS process."]
    Shorthand(String),
    #[doc = "Explicit execution configuration specifying process type (`process`, `docker`, or `docker-intrude`)."]
    Detailed(RunKind),
}

#[doc = "Explicit runtime environment type for a supervised process."]
#[derive(Debug, Deserialize, Serialize, Clone, JsonSchema)]
#[serde(tag = "type", rename_all = "kebab-case")]
#[serde(deny_unknown_fields)]
pub enum RunKind {
    #[doc = "Spawns a standard local OS process."]
    Process(ProcessRunConfig),
    #[doc = "Spawns a managed Docker container using `docker run --rm`."]
    Docker(DockerRunConfig),
    #[doc = "Spawns a local host binary attached inside a Docker bridge network namespace (`docker-intrude`)."]
    DockerIntrude(DockerIntrudeRunConfig),
}

#[doc = "Configuration for executing a local OS process."]
#[derive(Debug, Deserialize, Serialize, Clone, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ProcessRunConfig {
    #[doc = "If present, executes commands using the configed shell '-c' option."]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "WrappingShell")]
    pub shell: Option<WrappingShell>,
    #[doc = "The command string to execute."]
    pub cmd: String,
}

#[doc = "Configuration for executing a Docker container."]
#[derive(Debug, Deserialize, Serialize, Clone, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DockerRunConfig {
    #[doc = "The Docker image reference to run."]
    pub image: String,
    #[doc = "Arguments passed to `docker run --rm` (e.g., `--name`, `--network`, `--ip`)."]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "String")]
    pub opts: Option<String>,
    #[doc = "Arguments passed to the container's entrypoint."]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "String")]
    pub args: Option<String>,
}

#[doc = "Configuration for executing a host binary inside a Docker network namespace."]
#[derive(Debug, Deserialize, Serialize, Clone, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DockerIntrudeRunConfig {
    #[doc = "Static IP address assigned to the namespace container on the target bridge subnet."]
    pub ip: String,
    #[doc = "Target Docker network name (optional if exactly one network is defined in `docker_networks`)."]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "String")]
    pub network: Option<String>,
    #[doc = "If present, executes commands using the configed shell '-c' option."]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "WrappingShell")]
    pub shell: Option<WrappingShell>,
    #[doc = "Host command string to execute within the target network namespace."]
    pub cmd: String,
}

#[doc = "Configuration for creating a Docker bridge network."]
#[derive(Debug, Deserialize, Serialize, Clone, JsonSchema)]
#[serde(untagged)]
#[serde(deny_unknown_fields)]
pub enum DockerNetworkConfig {
    #[doc = "CIDR subnet string for the bridge network (e.g., `\"172.18.23.0/24\"`)."]
    Simple(String),
    #[doc = "Explicit network configuration specifying subnet CIDR and custom Docker creation arguments."]
    Detailed {
        subnet: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[schemars(with = "Vec<String>")]
        args: Option<Vec<String>>,
    },
}

impl DockerNetworkConfig {
    pub fn subnet(&self) -> &str {
        match self {
            Self::Simple(s) => s,
            Self::Detailed { subnet, .. } => subnet,
        }
    }

    pub fn args(&self) -> Option<&[String]> {
        match self {
            Self::Simple(_) => None,
            Self::Detailed { args, .. } => args.as_deref(),
        }
    }
}

#[doc = "Display rendering mode for a process pane."]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
#[serde(deny_unknown_fields)]
pub enum PaneMode {
    Log,
    Tui,
}

#[doc = "A recursive edge-carving and splitting node for arranging terminal panes."]
#[derive(Debug, Deserialize, Serialize, Clone, JsonSchema)]
#[serde(deny_unknown_fields)]
#[schemars(extend("oneOf" = [
    {
        "description": "Leaf node targeting a specific process name or 'combined-logs'",
        "required": ["name"],
        "not": { "anyOf": [{ "required": ["unassigned"] }, { "required": ["items"] }] }
    },
    {
        "description": "Leaf node acting as the container for unassigned process panes",
        "required": ["unassigned"],
        "not": { "anyOf": [{ "required": ["name"] }, { "required": ["items"] }] }
    },
    {
        "description": "Branch node splitting screen area recursively among child nodes",
        "required": ["items"],
        "not": { "anyOf": [{ "required": ["name"] }, { "required": ["unassigned"] }] }
    }
]))]
pub struct LayoutNode {
    #[doc = "Side of the remaining terminal space to carve from (`top`, `bottom`, `left`, `right`)."]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "LayoutEdge")]
    pub edge: Option<LayoutEdge>,

    #[doc = "Percentage of available space to allocate (0 to 100). Auto-calculated among siblings if omitted."]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "u16")]
    #[schemars(range(min = 0, max = 100))]
    pub size: Option<u16>,

    #[doc = "Split orientation for child panes inside this node (`horizontal` or `vertical`)."]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "LayoutDirection")]
    pub direction: Option<LayoutDirection>,

    #[doc = "Target process name to place inside this pane (or `\"combined-logs\"`)."]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(schema_with = "identifier_schema")]
    pub name: Option<String>,

    #[doc = "If true, automatically places all unassigned process panes inside this node."]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "bool")]
    pub unassigned: Option<bool>,

    #[doc = "Layout nodes to recursively arrange within this node."]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub items: Option<Vec<LayoutNode>>,
}

#[doc = "Target edge of the terminal space to carve from."]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
#[serde(deny_unknown_fields)]
pub enum LayoutEdge {
    Top,
    Bottom,
    Left,
    Right,
}

#[doc = "Split orientation for sub-panes (`horizontal` or `vertical`)."]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
#[serde(deny_unknown_fields)]
pub enum LayoutDirection {
    Horizontal,
    Vertical,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct PinchAudit {
    pub project: ProjectManifest,
    pub containers: Vec<String>,
}

fn schema_version_schema(_generator: &mut SchemaGenerator) -> Schema {
    let schema_val = serde_json::json!({
        "type": "integer",
        "const": 1,
        "description": "Explicit schema version for manifest compatibility (must be 1)."
    });
    serde_json::from_value(schema_val).expect("valid schema")
}

fn identifier_schema(_generator: &mut SchemaGenerator) -> Schema {
    let schema_val = serde_json::json!({
        "type": "string",
        "pattern": "^[A-Za-z0-9-_]+$",
        "description": "A valid identifier."
    });
    serde_json::from_value(schema_val).expect("valid schema")
}
