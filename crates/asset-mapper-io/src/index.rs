use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::path::{Path, PathBuf};

use asset_mapper_core::{
    AssetRecord, AssetType, Axis3, Bounds3, CURRENT_SCHEMA_VERSION, CoordinateConvention,
    Handedness, PackRecord, Pivot, ReviewFlag, Unit, hash::sha256_file,
};
use serde::{Deserialize, Serialize};

use crate::error::IoError;
use crate::sidecar::{canonical_sidecar_path, read_pack_from_input, write_pack_sidecar};

pub const SUPPORTED_ASSET_EXTENSIONS: &[&str] =
    &["glb", "gltf", "obj", "fbx", "png", "jpg", "jpeg", "webp"];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IndexedAsset {
    pub source_path: String,
    pub absolute_path: PathBuf,
    pub content_hash: String,
    pub asset_type: AssetType,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IndexReport {
    pub sidecar_path: String,
    pub discovered_assets: Vec<String>,
    pub new_assets: Vec<String>,
    pub unchanged_assets: Vec<String>,
    pub drifted_assets: Vec<String>,
    pub missing_assets: Vec<String>,
}

pub fn init_pack_folder(
    pack_root: impl AsRef<Path>,
    display_name: String,
) -> Result<IndexReport, IoError> {
    let pack_root = pack_root.as_ref();
    let sidecar_path = canonical_sidecar_path(pack_root);
    if sidecar_path.exists() {
        return Err(IoError::SidecarAlreadyExists { path: sidecar_path });
    }

    let indexed = scan_assets(pack_root)?;
    let pack_id = slug_from_text(&display_name);
    let mut used_asset_ids = HashSet::new();
    let assets = indexed
        .iter()
        .map(|asset| placeholder_asset(asset, &mut used_asset_ids))
        .collect::<Vec<_>>();

    let pack = PackRecord {
        schema_version: CURRENT_SCHEMA_VERSION,
        pack_id,
        display_name,
        coordinate_convention: CoordinateConvention {
            handedness: Handedness::Right,
            up_axis: Axis3::PosY,
            forward_axis: Axis3::PosZ,
        },
        default_units: Unit::Meters,
        connector_classes: Vec::new(),
        compatibility_rules: Vec::new(),
        assets,
    };

    write_pack_sidecar(pack_root, &pack)?;

    Ok(IndexReport {
        sidecar_path: sidecar_path.to_string_lossy().into_owned(),
        discovered_assets: sorted_sources(indexed.iter().map(|asset| asset.source_path.clone())),
        new_assets: sorted_sources(indexed.iter().map(|asset| asset.source_path.clone())),
        unchanged_assets: Vec::new(),
        drifted_assets: Vec::new(),
        missing_assets: Vec::new(),
    })
}

pub fn index_pack_folder(pack_root: impl AsRef<Path>) -> Result<IndexReport, IoError> {
    let pack_root = pack_root.as_ref();
    let mut loaded = read_pack_from_input(pack_root)?;
    let indexed = scan_assets(pack_root)?;
    let indexed_by_source = indexed
        .iter()
        .map(|asset| (asset.source_path.clone(), asset))
        .collect::<BTreeMap<_, _>>();
    let existing_sources = loaded
        .pack
        .assets
        .iter()
        .map(|asset| asset.source_path.clone())
        .collect::<BTreeSet<_>>();

    let mut unchanged_assets = Vec::new();
    let mut drifted_assets = Vec::new();
    let mut missing_assets = Vec::new();

    for asset in &loaded.pack.assets {
        match indexed_by_source.get(&asset.source_path) {
            Some(indexed_asset) if indexed_asset.content_hash == asset.content_hash => {
                unchanged_assets.push(asset.source_path.clone());
            }
            Some(_) => {
                drifted_assets.push(asset.source_path.clone());
            }
            None => {
                missing_assets.push(asset.source_path.clone());
            }
        }
    }

    let mut used_asset_ids = loaded
        .pack
        .assets
        .iter()
        .map(|asset| asset.asset_id.clone())
        .collect::<HashSet<_>>();
    let mut new_assets = Vec::new();
    for indexed_asset in &indexed {
        if existing_sources.contains(&indexed_asset.source_path) {
            continue;
        }

        new_assets.push(indexed_asset.source_path.clone());
        loaded
            .pack
            .assets
            .push(placeholder_asset(indexed_asset, &mut used_asset_ids));
    }

    loaded
        .pack
        .assets
        .sort_by(|left, right| left.source_path.cmp(&right.source_path));
    write_pack_sidecar(pack_root, &loaded.pack)?;

    Ok(IndexReport {
        sidecar_path: canonical_sidecar_path(pack_root)
            .to_string_lossy()
            .into_owned(),
        discovered_assets: sorted_sources(indexed.iter().map(|asset| asset.source_path.clone())),
        new_assets: sorted_sources(new_assets),
        unchanged_assets: sorted_sources(unchanged_assets),
        drifted_assets: sorted_sources(drifted_assets),
        missing_assets: sorted_sources(missing_assets),
    })
}

pub fn scan_assets(pack_root: impl AsRef<Path>) -> Result<Vec<IndexedAsset>, IoError> {
    let pack_root = pack_root.as_ref();
    let mut assets = Vec::new();
    scan_dir(pack_root, pack_root, &mut assets)?;
    assets.sort_by(|left, right| left.source_path.cmp(&right.source_path));
    Ok(assets)
}

fn scan_dir(
    pack_root: &Path,
    current_dir: &Path,
    assets: &mut Vec<IndexedAsset>,
) -> Result<(), IoError> {
    let entries = std::fs::read_dir(current_dir).map_err(|source| IoError::ScanDir {
        path: current_dir.to_path_buf(),
        source,
    })?;

    for entry in entries {
        let entry = entry.map_err(|source| IoError::ReadDirEntry {
            path: current_dir.to_path_buf(),
            source,
        })?;
        let path = entry.path();
        let file_type = entry.file_type().map_err(|source| IoError::ReadDirEntry {
            path: current_dir.to_path_buf(),
            source,
        })?;

        if file_type.is_dir() {
            if entry.file_name().to_string_lossy() == ".asset-mapper" {
                continue;
            }
            scan_dir(pack_root, &path, assets)?;
        } else if file_type.is_file() && is_supported_asset_file(&path) {
            let relative = path
                .strip_prefix(pack_root)
                .map_err(|_| IoError::StripPackRoot {
                    path: path.clone(),
                    root: pack_root.to_path_buf(),
                })?;
            let source_path = path_to_forward_slashes(relative);
            let hash = sha256_file(&path).map_err(|source| IoError::HashFile {
                path: path.clone(),
                source,
            })?;
            let asset_type = asset_type_from_path(relative);
            assets.push(IndexedAsset {
                source_path,
                absolute_path: path,
                content_hash: format!("sha256:{hash}"),
                asset_type,
            });
        }
    }

    Ok(())
}

fn placeholder_asset(indexed: &IndexedAsset, used_asset_ids: &mut HashSet<String>) -> AssetRecord {
    AssetRecord {
        asset_id: unique_asset_id(&indexed.source_path, used_asset_ids),
        source_path: indexed.source_path.clone(),
        content_hash: indexed.content_hash.clone(),
        display_name: display_name_from_source_path(&indexed.source_path),
        asset_type: indexed.asset_type.clone(),
        bounds: Bounds3 {
            min: [-0.5, -0.5, -0.5],
            max: [0.5, 0.5, 0.5],
        },
        dimensions: [1.0, 1.0, 1.0],
        pivot: Pivot::Origin,
        up_axis: Axis3::PosY,
        forward_axis: Axis3::PosZ,
        semantic_tags: Vec::new(),
        affordances: Vec::new(),
        placement_constraints: Vec::new(),
        review_flags: vec![
            ReviewFlag::BoundsPlaceholder,
            ReviewFlag::OrientationPlaceholder,
            ReviewFlag::PivotPlaceholder,
        ],
        connectors: Vec::new(),
    }
}

fn is_supported_asset_file(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .map(|extension| {
            SUPPORTED_ASSET_EXTENSIONS
                .iter()
                .any(|supported| supported.eq_ignore_ascii_case(extension))
        })
        .unwrap_or(false)
}

fn asset_type_from_path(path: &Path) -> AssetType {
    match path
        .extension()
        .and_then(|extension| extension.to_str())
        .map(str::to_ascii_lowercase)
        .as_deref()
    {
        Some("png" | "jpg" | "jpeg" | "webp") => AssetType::Sprite2d,
        _ => AssetType::Model3d,
    }
}

fn path_to_forward_slashes(path: &Path) -> String {
    path.components()
        .map(|component| component.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
}

fn display_name_from_source_path(source_path: &str) -> String {
    let stem = source_path
        .rsplit('/')
        .next()
        .unwrap_or(source_path)
        .rsplit_once('.')
        .map(|(name, _)| name)
        .unwrap_or(source_path);

    stem.replace(['_', '-'], " ")
}

fn unique_asset_id(source_path: &str, used_asset_ids: &mut HashSet<String>) -> String {
    let base = slug_from_text(
        source_path
            .rsplit_once('.')
            .map(|(path, _)| path)
            .unwrap_or(source_path),
    );
    let mut candidate = if base.is_empty() {
        "asset".to_owned()
    } else {
        base
    };

    if used_asset_ids.insert(candidate.clone()) {
        return candidate;
    }

    let root = candidate;
    for suffix in 2.. {
        candidate = format!("{root}_{suffix}");
        if used_asset_ids.insert(candidate.clone()) {
            return candidate;
        }
    }

    unreachable!("unbounded suffix loop always returns");
}

fn slug_from_text(input: &str) -> String {
    let mut output = String::new();
    let mut previous_was_separator = false;

    for character in input.chars() {
        if character.is_ascii_alphanumeric() {
            output.push(character.to_ascii_lowercase());
            previous_was_separator = false;
        } else if !previous_was_separator {
            output.push('_');
            previous_was_separator = true;
        }
    }

    output.trim_matches('_').to_owned()
}

fn sorted_sources(sources: impl IntoIterator<Item = String>) -> Vec<String> {
    sources
        .into_iter()
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}
