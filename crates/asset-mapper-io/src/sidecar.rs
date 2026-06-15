use std::path::{Path, PathBuf};

use asset_mapper_core::PackRecord;

use crate::error::IoError;

pub const METADATA_DIR: &str = ".asset-mapper";
pub const SIDECAR_FILE: &str = "pack.assetmap.json";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PackInputKind {
    PackFolder,
    DirectSidecar,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedPackInput {
    pub kind: PackInputKind,
    pub sidecar_path: PathBuf,
    pub pack_root: Option<PathBuf>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LoadedPack {
    pub pack: PackRecord,
    pub resolved: ResolvedPackInput,
}

pub fn canonical_sidecar_path(pack_root: impl AsRef<Path>) -> PathBuf {
    pack_root.as_ref().join(METADATA_DIR).join(SIDECAR_FILE)
}

pub fn resolve_pack_input_path(input: impl AsRef<Path>) -> Result<ResolvedPackInput, IoError> {
    let input = input.as_ref();
    if input.is_dir() {
        return Ok(ResolvedPackInput {
            kind: PackInputKind::PackFolder,
            sidecar_path: canonical_sidecar_path(input),
            pack_root: Some(input.to_path_buf()),
        });
    }

    if input.is_file() {
        if !is_direct_sidecar_path(input) {
            return Err(IoError::InvalidPackInput {
                path: input.to_path_buf(),
            });
        }

        return Ok(ResolvedPackInput {
            kind: PackInputKind::DirectSidecar,
            sidecar_path: input.to_path_buf(),
            pack_root: infer_pack_root_from_sidecar(input),
        });
    }

    Err(IoError::InvalidPackInput {
        path: input.to_path_buf(),
    })
}

pub fn read_pack_from_input(input: impl AsRef<Path>) -> Result<LoadedPack, IoError> {
    let resolved = resolve_pack_input_path(input)?;
    if !resolved.sidecar_path.is_file() {
        return Err(IoError::MissingSidecar {
            path: resolved.sidecar_path,
        });
    }

    let input =
        std::fs::read_to_string(&resolved.sidecar_path).map_err(|source| IoError::ReadFile {
            path: resolved.sidecar_path.clone(),
            source,
        })?;
    let pack = serde_json::from_str(&input).map_err(|source| IoError::ParseJson {
        path: resolved.sidecar_path.clone(),
        source,
    })?;

    Ok(LoadedPack { pack, resolved })
}

pub fn write_pack_sidecar(
    pack_root: impl AsRef<Path>,
    pack: &PackRecord,
) -> Result<PathBuf, IoError> {
    let sidecar_path = canonical_sidecar_path(pack_root);
    let metadata_dir = sidecar_path
        .parent()
        .expect("canonical sidecar path always has a parent");

    std::fs::create_dir_all(metadata_dir).map_err(|source| IoError::CreateDir {
        path: metadata_dir.to_path_buf(),
        source,
    })?;

    let output = serde_json::to_string_pretty(pack).map_err(IoError::SerializeJson)?;
    std::fs::write(&sidecar_path, format!("{output}\n")).map_err(|source| IoError::WriteFile {
        path: sidecar_path.clone(),
        source,
    })?;

    Ok(sidecar_path)
}

fn is_direct_sidecar_path(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.ends_with(".assetmap.json"))
}

fn infer_pack_root_from_sidecar(sidecar_path: &Path) -> Option<PathBuf> {
    if sidecar_path.file_name()?.to_str()? != SIDECAR_FILE {
        return None;
    }

    let metadata_dir = sidecar_path.parent()?;
    if metadata_dir.file_name()?.to_str()? != METADATA_DIR {
        return None;
    }

    metadata_dir.parent().map(Path::to_path_buf)
}
