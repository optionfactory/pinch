use schemars::{JsonSchema, Schema, SchemaGenerator};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};

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

    #[doc = "If true, executes shorthand command strings using `bash -c` globally (default: `false`)."]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "bool")]
    pub shell: Option<bool>,

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
    #[schemars(with = "Vec<LayoutBlock>")]
    pub layout: Option<Vec<LayoutBlock>>,
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
#[doc = ""]
#[doc = "### Framework Applicability Guide"]
#[doc = "* **`dora`:** EU financial sector & ICT third-party providers to finance."]
#[doc = "* **`cra`:** Software products with digital elements sold/distributed in the EU."]
#[doc = "* **`nis2`:** Critical/important non-financial infrastructure (energy, health, SaaS, MSPs)."]
#[doc = "* **`aiact`:** Systems integrating AI/ML models, LLMs, or automated inference."]
#[doc = "* **`gdpr`:** Any project processing, storing, or transmitting EU/EEA personal data."]
#[doc = "* **`ads`:** Italian Garante Privacy governance for privileged IT/database administrators."]
#[derive(Debug, Deserialize, Serialize, Clone, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ComplianceManifest {
    #[doc = "DORA (Digital Operational Resilience Act) criticality tag."]
    #[doc = ""]
    #[doc = "### Applicability Scope"]
    #[doc = "Applies if the project operates within or provides ICT services to **EU financial institutions** (banks, insurance companies, investment firms, payment processors, or crypto-asset service providers)."]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "DoraCriticality")]
    pub dora: Option<DoraCriticality>,

    #[doc = "EU Cyber Resilience Act (CRA) product classification tier."]
    #[doc = ""]
    #[doc = "### Applicability Scope"]
    #[doc = "Applies to any **software product with digital elements** distributed, commercialized, or deployed in the EU marketplace (including standalone software, firmware, and SaaS runtimes)."]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "CraClass")]
    pub cra: Option<CraClass>,

    #[doc = "EU NIS2 Directive criticality classification."]
    #[doc = ""]
    #[doc = "### Applicability Scope"]
    #[doc = "Applies if the project supports **critical or important non-financial infrastructure** across the EU (such as energy, healthcare, transport, water, digital providers, SaaS marketplaces, or managed ICT services)."]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "Nis2Category")]
    pub nis2: Option<Nis2Category>,

    #[doc = "EU AI Act risk classification tier."]
    #[doc = ""]
    #[doc = "### Applicability Scope"]
    #[doc = "Applies whenever the project develops, deploys, or integrates **AI/ML models, LLMs, foundation models, or automated decision-making/inference engines**."]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "AiActClass")]
    pub ai_act: Option<AiActClass>,

    #[doc = "GDPR data protection role and data residency boundaries."]
    #[doc = ""]
    #[doc = "### Applicability Scope"]
    #[doc = "Applies to **any project that processes, stores, transmits, or logs personal data** (PII/SPI) belonging to individuals located in the EU/EEA."]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "GdprManifest")]
    pub gdpr: Option<GdprManifest>,

    #[doc = "Italian Garante Privacy 'Amministratore di Sistema' (AdS) compliance block."]
    #[doc = ""]
    #[doc = "### Applicability Scope"]
    #[doc = "Mandatory under **Italian data protection regulations (Provvedimento Garante Privacy 2008/2009)** whenever technical personnel or automated processes hold **privileged administrative access** over systems or databases containing personal data."]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "AdsManifest")]
    pub ads: Option<AdsManifest>,
}

#[doc = "EU NIS2 Directive classification tier for cybersecurity resilience."]
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
}

#[doc = "EU AI Act risk classification tier."]
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
    #[doc = "Not an AI system; EU AI Act not applicable."]
    NotApplicable,
}

#[doc = "General Data Protection Regulation (GDPR) architectural specification."]
#[derive(Debug, Deserialize, Serialize, Clone, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct GdprManifest {
    #[doc = "Our organization's legal role regarding personal data in this project."]
    pub role: GdprRole,

    #[doc = "Whether personal data is processed or stored strictly within the EU/EEA."]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "DataResidency")]
    pub data_residency: Option<DataResidency>,
}

#[doc = "Geographic data storage and processing boundary for compliance."]
#[doc = ""]
#[doc = "Storing or transferring personal data across any EEA country (e.g., hosting servers in Norway or Iceland instead of Germany)"]
#[doc = "meets standard EU data residency requirements without requiring special cross-border transfer safeguards like Standard Contractual Clauses (SCCs)"]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
#[serde(deny_unknown_fields)]
pub enum DataResidency {
    #[doc = "Data is stored and processed strictly within European Union member states."]
    Eu,
    #[doc = "Data is stored and processed within the European Economic Area (EU + Iceland, Liechtenstein, Norway)."]
    Eea,
    #[doc = "Data is stored or processed globally across international regions."]
    Global,
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
}

#[doc = "DORA (Digital Operational Resilience Act) operational criticality tag."]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
#[serde(deny_unknown_fields)]
pub enum DoraCriticality {
    #[doc = "Supports a Critical or Important Function (CIF) subject to strict RTO/RPO SLAs."]
    CifSupported,

    #[doc = "Non-critical ICT supporting service or internal developer utility."]
    NonCritical,
}

#[doc = "EU Cyber Resilience Act (CRA) product classification tier for cybersecurity compliance."]
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
}

#[doc = "Italian Garante Privacy 'Amministratore di Sistema' (AdS) compliance specification."]
#[derive(Debug, Deserialize, Serialize, Clone, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AdsManifest {
    #[doc = "Responsibility boundary for System Administration (AdS) operations."]
    pub responsibility: AdsResponsibility,

    #[doc = "Immutable access log retention status (login/logout/failed attempts)."]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "AdsLoggingStatus")]
    pub logging: Option<AdsLoggingStatus>,

    #[doc = "Whether formal designation letters ('Lettera di Nomina') have been executed."]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "bool")]
    pub nominated: Option<bool>,

    #[doc = "Date when the last annual verification/audit was completed (ISO 8601 format: YYYY-MM-DD)."]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(schema_with = "iso_date_schema")]
    pub latest_audit: Option<String>,
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

    #[doc = "No AdS scope applies (no personal data processed or no privileged access)."]
    NotApplicable,
}

#[doc = "Status of immutable AdS access log generation and retention."]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
#[serde(deny_unknown_fields)]
pub enum AdsLoggingStatus {
    #[doc = "Immutable access logs recorded and retained for at least 6 months (standard)."]
    Immutable6Months,

    #[doc = "Immutable access logs recorded and retained for at least 12 months (banking/health)."]
    Immutable12Months,

    #[doc = "Logging is enabled but not tamper-proof or integrity-verified."]
    StandardLoggingOnly,

    #[doc = "Access logging is delegated to external cloud/infrastructure provider."]
    ExternalProvider,

    #[doc = "Logging is not implemented or disabled."]
    Disabled,
}

#[doc = "Supported deployment environment types."]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
#[serde(deny_unknown_fields)]
pub enum EnvironmentType {
    #[doc = "Local developer workstation or local machine loopback."]
    Local,
    #[doc = "Shared cloud or hosted development environment."]
    Development,
    #[doc = "Short-lived, dynamic per-PR or per-branch preview environment."]
    Preview,
    #[doc = "Automated CI/CD integration and unit testing environment."]
    Testing,
    #[doc = "Dedicated Quality Assurance and manual regression environment."]
    Qa,
    #[doc = "User Acceptance Testing environment for business stakeholder validation."]
    Uat,
    #[doc = "Isolated sandbox playground for external integrations and experimentation."]
    Sandbox,
    #[doc = "Near-exact production replica environment running parallel cutover validation (ambiente di parallelo)."]
    PreProduction,
    #[doc = "Pre-production mirror environment for final release candidate acceptance."]
    Staging,
    #[doc = "Sales demo, customer sandbox, or product preview environment."]
    Demo,
    #[doc = "Dedicated load testing and performance benchmarking environment."]
    Performance,
    #[doc = "Dark-launch or traffic-mirrored environment receiving real-time production traffic passively."]
    Shadow,
    #[doc = "Live customer-facing production environment."]
    Production,
    #[doc = "Disaster recovery, warm standby, or secondary failover region."]
    DisasterRecovery,
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

    #[doc = "Network ingress reachability for this specific environment."]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "ExposureType")]
    pub ingress: Option<ExposureType>,

    #[doc = "Network management reachability for this specific environment."]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "ExposureType")]
    pub management: Option<ExposureType>,

    #[doc = "Map of domain names to their management origin (managed vs external)."]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "BTreeMap<String, DomainManagement>")]
    pub domains: Option<BTreeMap<String, DomainManagement>>,
}

#[doc = "Network ingress reachability for the project."]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
#[serde(deny_unknown_fields)]
pub enum ExposureType {
    #[doc = "Reachable only on localhost or local machine process loopback."]
    Local,
    #[doc = "Reachable only inside the private corporate VPN, VPC, or internal service mesh."]
    RestrictedVpn,
    #[doc = "Reachable from the internet but restricted by IP allowlisting or CIDR controls."]
    RestrictedIp,
    #[doc = "Reachable only through a Privileged Access Management (PAM) broker or session proxy (e.g., CyberArk, Teleport)."]
    RestrictedPam,
    #[doc = "Publicly reachable from the internet."]
    Internet,
    #[doc = "No incoming network traffic (e.g., background worker or CLI tool)."]
    None,
}

#[doc = "Domain management and ownership origin."]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
#[serde(deny_unknown_fields)]
pub enum DomainManagement {
    #[doc = "Managed directly by our infrastructure / DNS / CDN rules."]
    Managed,
    #[doc = "Managed externally by a client, partner, or third party."]
    External,
}

#[doc = "Configuration for an individual supervised process or container."]
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ProcessManifest {
    #[doc = "Name of the process."]
    #[schemars(schema_with = "identifier_schema")]    
    pub name: String,

    #[doc = "Display name of the process pane in the TUI dashboard. (defaults to name)"]
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
    #[doc = "Whether to wrap the command string in `bash -c` (default: `false`)."]
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub bash: bool,
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
    #[doc = "Whether to wrap the command string in `bash -c` (default: `false`)."]
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub bash: bool,
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
        #[doc = "CIDR subnet string for the bridge network."]
        subnet: String,
        #[doc = "Custom CLI arguments passed to `docker network create`."]
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
    #[doc = "Standard streaming log tailer with wrap and truncation controls."]
    Log,
    #[doc = "Allocates a PTY for interactive terminal applications (`top`, `vim`, `htop`)."]
    Tui,
}

#[doc = "A progressive edge-carving block ruleset for arranging terminal panes."]
#[derive(Debug, Deserialize, Serialize, Clone, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct LayoutBlock {
    #[doc = "Target process name to place inside this block (or `\"Combined Logs\"`)."]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(schema_with = "identifier_schema")]
    pub name: Option<String>,

    #[doc = "Side of the remaining terminal space to carve from (`top`, `bottom`, `left`, `right`)."]
    pub edge: LayoutEdge,

    #[doc = "Percentage of currently available space to allocate (0 to 100)."]
    pub size_percentage: u16,

    #[doc = "Split orientation for sub-panes inside an edge-carved block."]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "LayoutDirection")]
    pub direction: Option<LayoutDirection>,

    #[doc = "Sub-panes to arrange within this carved edge block."]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "Vec<LayoutSplit>")]
    pub splits: Option<Vec<LayoutSplit>>,

    #[doc = "If true, automatically places all unassigned process panes inside this block."]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "bool")]
    pub unassigned: Option<bool>,
}

#[doc = "Target edge of the terminal space to carve from."]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
#[serde(deny_unknown_fields)]
pub enum LayoutEdge {
    #[doc = "Carve from the top edge."]
    Top,
    #[doc = "Carve from the bottom edge."]
    Bottom,
    #[doc = "Carve from the left edge."]
    Left,
    #[doc = "Carve from the right edge."]
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

#[doc = "A sub-pane division within an edge-carved layout block."]
#[derive(Debug, Deserialize, Serialize, Clone, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct LayoutSplit {
    #[doc = "Target process name to place inside this split (or `\"combined-logs\"`)."]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(schema_with = "identifier_schema")]
    pub name: Option<String>,

    #[doc = "Percentage of space within the parent block to allocate (0 to 100)."]
    pub size: u16,

    #[doc = "If true, automatically places all unassigned process panes inside this split."]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "bool")]
    pub unassigned: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct PinchAudit {
    pub project: ProjectManifest,
    pub containers: Vec<String>,
}

fn iso_date_schema(_generator: &mut SchemaGenerator) -> Schema {
    let schema_val = serde_json::json!({
        "type": "string",
        "format": "date"
    });
    serde_json::from_value(schema_val).expect("valid schema")
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