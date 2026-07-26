//! Disk-backed schema migration for pack folders / sidecars.

use std::path::Path;

use asset_mapper_core::{MigrationReport, migrate_pack, pack_from_legacy_json};

use crate::error::IoError;
use crate::sidecar::{
    LoadedPack, PackInputKind, read_pack_from_input, resolve_pack_input_path, write_pack_sidecar,
};

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct MigrateIoReport {
    pub sidecar_path: String,
    pub migration: MigrationReport,
}

/// Load a pack (including legacy JSON), migrate to current schema, and write back.
pub fn migrate_pack_input(path: impl AsRef<Path>) -> Result<MigrateIoReport, IoError> {
    let path = path.as_ref();
    let resolved = resolve_pack_input_path(path)?;

    // Prefer strict load. Only fall back to legacy JSON normalization on parse
    // failures — rethrow missing files, I/O, and invalid paths unchanged.
    let loaded = match read_pack_from_input(path) {
        Ok(loaded) => loaded,
        Err(error @ IoError::ParseJson { .. }) => {
            let input = std::fs::read_to_string(&resolved.sidecar_path).map_err(|source| {
                IoError::ReadFile {
                    path: resolved.sidecar_path.clone(),
                    source,
                }
            })?;
            let value: serde_json::Value =
                serde_json::from_str(&input).map_err(|source| IoError::ParseJson {
                    path: resolved.sidecar_path.clone(),
                    source,
                })?;
            let pack = pack_from_legacy_json(value).map_err(|legacy_error| {
                IoError::Migration(format!(
                    "legacy parse after schema failure ({error}): {legacy_error}"
                ))
            })?;
            LoadedPack {
                pack,
                resolved: resolved.clone(),
            }
        }
        Err(other) => return Err(other),
    };

    let (migrated, report) =
        migrate_pack(loaded.pack).map_err(|error| IoError::Migration(error.to_string()))?;

    match loaded.resolved.kind {
        PackInputKind::PackFolder => {
            let pack_root = loaded.resolved.pack_root.as_ref().ok_or_else(|| {
                IoError::Migration("pack folder root missing after resolve".to_owned())
            })?;
            write_pack_sidecar(pack_root, &migrated)?;
        }
        PackInputKind::DirectSidecar => {
            let json = serde_json::to_string_pretty(&migrated).map_err(IoError::SerializeJson)?;
            std::fs::write(&loaded.resolved.sidecar_path, format!("{json}\n")).map_err(
                |source| IoError::WriteFile {
                    path: loaded.resolved.sidecar_path.clone(),
                    source,
                },
            )?;
        }
    }

    Ok(MigrateIoReport {
        sidecar_path: loaded.resolved.sidecar_path.to_string_lossy().into_owned(),
        migration: report,
    })
}
