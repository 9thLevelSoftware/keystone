use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum IoError {
    #[error("pack sidecar already exists at `{path}`")]
    SidecarAlreadyExists { path: PathBuf },

    #[error("pack sidecar does not exist at `{path}`")]
    MissingSidecar { path: PathBuf },

    #[error("path `{path}` is neither a file nor a directory")]
    InvalidPackInput { path: PathBuf },

    #[error("failed to read `{path}`: {source}")]
    ReadFile {
        path: PathBuf,
        source: std::io::Error,
    },

    #[error("failed to write `{path}`: {source}")]
    WriteFile {
        path: PathBuf,
        source: std::io::Error,
    },

    #[error("failed to create directory `{path}`: {source}")]
    CreateDir {
        path: PathBuf,
        source: std::io::Error,
    },

    #[error("failed to scan directory `{path}`: {source}")]
    ScanDir {
        path: PathBuf,
        source: std::io::Error,
    },

    #[error("failed to read directory entry under `{path}`: {source}")]
    ReadDirEntry {
        path: PathBuf,
        source: std::io::Error,
    },

    #[error("failed to hash `{path}`: {source}")]
    HashFile {
        path: PathBuf,
        source: std::io::Error,
    },

    #[error("failed to derive a relative source path for `{path}` under `{root}`")]
    StripPackRoot { path: PathBuf, root: PathBuf },

    #[error("failed to parse JSON `{path}`: {source}")]
    ParseJson {
        path: PathBuf,
        source: serde_json::Error,
    },

    #[error("failed to serialize pack JSON: {0}")]
    SerializeJson(serde_json::Error),
}
