#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EditorCommandError {
    pub code: String,
    pub message: String,
}

impl EditorCommandError {
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
        }
    }
}

impl From<asset_mapper_io::IoError> for EditorCommandError {
    fn from(error: asset_mapper_io::IoError) -> Self {
        Self::new("io_error", error.to_string())
    }
}

impl From<serde_json::Error> for EditorCommandError {
    fn from(error: serde_json::Error) -> Self {
        Self::new("json_error", error.to_string())
    }
}

impl From<std::io::Error> for EditorCommandError {
    fn from(error: std::io::Error) -> Self {
        Self::new("io_error", error.to_string())
    }
}
