#[derive(
    Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    Error,
    Warning,
}

#[derive(
    Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
pub struct Diagnostic {
    pub code: String,
    pub severity: Severity,
    pub message: String,
    pub asset_id: Option<String>,
    pub connector_id: Option<String>,
}

impl Diagnostic {
    pub fn error(code: &str, message: impl Into<String>) -> Self {
        Self {
            code: code.to_owned(),
            severity: Severity::Error,
            message: message.into(),
            asset_id: None,
            connector_id: None,
        }
    }

    pub fn warning(code: &str, message: impl Into<String>) -> Self {
        Self {
            code: code.to_owned(),
            severity: Severity::Warning,
            message: message.into(),
            asset_id: None,
            connector_id: None,
        }
    }

    pub fn with_asset(mut self, asset_id: impl Into<String>) -> Self {
        self.asset_id = Some(asset_id.into());
        self
    }

    pub fn with_connector(mut self, connector_id: impl Into<String>) -> Self {
        self.connector_id = Some(connector_id.into());
        self
    }
}

#[derive(
    Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
pub struct ValidationReport {
    pub diagnostics: Vec<Diagnostic>,
}

impl ValidationReport {
    pub fn new(diagnostics: Vec<Diagnostic>) -> Self {
        Self { diagnostics }
    }

    pub fn is_valid(&self) -> bool {
        !self
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.severity == Severity::Error)
    }
}
