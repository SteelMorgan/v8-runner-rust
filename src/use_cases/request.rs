/// Transport-neutral request for the `build` use case.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuildRequest {
    pub full_rebuild: bool,
}

/// Transport-neutral request for the `test` use case.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TestRequest {
    pub full: bool,
    pub scope: TestScopeRequest,
}

/// Transport-neutral test scope.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TestScopeRequest {
    All,
    Module { name: String },
}

/// Transport-neutral request for the `dump` use case.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DumpRequest {
    pub mode: String,
    pub source_set: Option<String>,
    pub extension: Option<String>,
    pub objects: Vec<String>,
}

/// Transport-neutral request for the `syntax` use case.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyntaxRequest {
    pub target: SyntaxTargetRequest,
}

/// Transport-neutral syntax target.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SyntaxTargetRequest {
    DesignerConfig(DesignerConfigSyntaxRequest),
    DesignerModules(DesignerModulesSyntaxRequest),
    Edt { projects: Vec<String> },
}

/// Transport-neutral request for Designer configuration checks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DesignerConfigSyntaxRequest {
    pub config_log_integrity: bool,
    pub incorrect_references: bool,
    pub thin_client: bool,
    pub web_client: bool,
    pub mobile_client: bool,
    pub server: bool,
    pub external_connection: bool,
    pub external_connection_server: bool,
    pub mobile_app_client: bool,
    pub mobile_app_server: bool,
    pub thick_client_managed_application: bool,
    pub thick_client_server_managed_application: bool,
    pub thick_client_ordinary_application: bool,
    pub thick_client_server_ordinary_application: bool,
    pub mobile_client_digi_sign: bool,
    pub distributive_modules: bool,
    pub unreference_procedures: bool,
    pub handlers_existence: bool,
    pub empty_handlers: bool,
    pub extended_modules_check: bool,
    pub check_use_synchronous_calls: bool,
    pub check_use_modality: bool,
    pub unsupported_functional: bool,
    pub extension: Option<String>,
    pub all_extensions: bool,
}

/// Transport-neutral request for Designer module checks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DesignerModulesSyntaxRequest {
    pub thin_client: bool,
    pub web_client: bool,
    pub server: bool,
    pub external_connection: bool,
    pub thick_client_ordinary_application: bool,
    pub mobile_app_client: bool,
    pub mobile_app_server: bool,
    pub mobile_client: bool,
    pub extended_modules_check: bool,
    pub extension: Option<String>,
    pub all_extensions: bool,
}

/// Transport-neutral request for the `launch` use case.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LaunchRequest {
    pub mode: String,
}
