use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::path::{Path, PathBuf};

use asset_mapper_core::{
    AnalyzeOptions, AnalyzeReport, AssetRecord, AssetType, Axis3, Bounds3, CURRENT_SCHEMA_VERSION,
    ControlledVocabulary, CoordinateConvention, Handedness, PackProvenance, PackRecord, Pivot,
    ReviewFlag, Unit, analyze_pack, hash::sha256_file,
};
use serde::{Deserialize, Serialize};

use crate::bounds::measure_asset_bounds;
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

/// Options for creating a production-ready pack sidecar.
#[derive(Debug, Clone)]
pub struct InitPackOptions {
    pub display_name: String,
    /// Must pass [`asset_mapper_core::license_summary_is_production_ready`].
    pub license_summary: String,
    pub author: Option<String>,
    pub source: Option<String>,
}

impl InitPackOptions {
    /// Convenience constructor for unit tests and fixtures.
    pub fn for_tests(display_name: impl Into<String>) -> Self {
        Self {
            display_name: display_name.into(),
            license_summary: "MIT".to_owned(),
            author: Some("Test Author".to_owned()),
            source: None,
        }
    }

    pub fn validate(&self) -> Result<(), IoError> {
        if self.display_name.trim().is_empty() {
            return Err(IoError::InvalidInitOptions {
                message: "display name must not be empty".to_owned(),
            });
        }
        if !asset_mapper_core::license_summary_is_production_ready(&self.license_summary) {
            return Err(IoError::InvalidInitOptions {
                message: "license_summary is required and must not be an UNSPECIFIED placeholder"
                    .to_owned(),
            });
        }
        let author = self.author.as_ref().map(|s| s.trim()).filter(|s| !s.is_empty());
        let source = self.source.as_ref().map(|s| s.trim()).filter(|s| !s.is_empty());
        if author.is_none() && source.is_none() {
            return Err(IoError::InvalidInitOptions {
                message: "at least one of author or source is required for provenance".to_owned(),
            });
        }
        Ok(())
    }
}

pub fn init_pack_folder(
    pack_root: impl AsRef<Path>,
    options: InitPackOptions,
) -> Result<IndexReport, IoError> {
    options.validate()?;
    let pack_root = pack_root.as_ref();
    let sidecar_path = canonical_sidecar_path(pack_root);
    if sidecar_path.exists() {
        return Err(IoError::SidecarAlreadyExists { path: sidecar_path });
    }

    let display_name = options.display_name.trim().to_owned();
    let license_summary = options.license_summary.trim().to_owned();
    let author = options
        .author
        .as_ref()
        .map(|s| s.trim().to_owned())
        .filter(|s| !s.is_empty());
    let source = options
        .source
        .as_ref()
        .map(|s| s.trim().to_owned())
        .filter(|s| !s.is_empty())
        .or_else(|| Some(pack_root.display().to_string()));

    let indexed = scan_assets(pack_root)?;
    let pack_id = pack_id_from_display_name(&display_name);
    let mut used_asset_ids = HashSet::new();
    let assets = indexed
        .iter()
        .map(|asset| placeholder_asset(asset, &mut used_asset_ids))
        .collect::<Vec<_>>();

    let pack = PackRecord {
        schema_version: CURRENT_SCHEMA_VERSION,
        pack_id,
        display_name: display_name.clone(),
        coordinate_convention: CoordinateConvention {
            handedness: Handedness::Right,
            up_axis: Axis3::PosY,
            forward_axis: Axis3::PosZ,
        },
        default_units: Unit::Meters,
        license_summary,
        provenance: PackProvenance {
            source,
            author,
            created_at: None,
            notes: Some(format!("Initialized pack `{display_name}`")),
        },
        vocabulary: ControlledVocabulary::default(),
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
    let measured = measure_asset_bounds(&indexed.absolute_path).ok().flatten();

    let (bounds, dimensions, review_flags) = if let Some(measured) = measured {
        (
            measured.bounds,
            measured.dimensions,
            vec![
                ReviewFlag::OrientationPlaceholder,
                ReviewFlag::PivotPlaceholder,
            ],
        )
    } else {
        (
            Bounds3 {
                min: [-0.5, -0.5, -0.5],
                max: [0.5, 0.5, 0.5],
            },
            [1.0, 1.0, 1.0],
            vec![
                ReviewFlag::BoundsPlaceholder,
                ReviewFlag::OrientationPlaceholder,
                ReviewFlag::PivotPlaceholder,
            ],
        )
    };

    AssetRecord {
        asset_id: unique_asset_id(&indexed.source_path, used_asset_ids),
        source_path: indexed.source_path.clone(),
        content_hash: indexed.content_hash.clone(),
        display_name: display_name_from_source_path(&indexed.source_path),
        asset_type: indexed.asset_type.clone(),
        bounds,
        dimensions,
        pivot: Pivot::Origin,
        up_axis: Axis3::PosY,
        forward_axis: Axis3::PosZ,
        semantic_tags: Vec::new(),
        affordances: Vec::new(),
        placement_constraints: Vec::new(),
        review_flags,
        connectors: Vec::new(),
    }
}

/// Re-measure bounds from the source file and clear `BoundsPlaceholder` when successful.
pub fn apply_measured_bounds(
    asset: &mut AssetRecord,
    absolute_path: &Path,
) -> Result<bool, IoError> {
    let Some(measured) = measure_asset_bounds(absolute_path)? else {
        return Ok(false);
    };
    asset.bounds = measured.bounds;
    asset.dimensions = measured.dimensions;
    asset
        .review_flags
        .retain(|flag| *flag != ReviewFlag::BoundsPlaceholder);
    Ok(true)
}

/// Accept content-hash drift for one or more assets after human review.
///
/// Updates `content_hash` from the current on-disk file. Connectors and other
/// authored metadata are preserved unless `clear_connectors` is true.
pub fn accept_hash_drift(
    pack_root: impl AsRef<Path>,
    asset_ids: Option<Vec<String>>,
    clear_connectors: bool,
) -> Result<IndexReport, IoError> {
    let pack_root = pack_root.as_ref();
    let mut loaded = read_pack_from_input(pack_root)?;
    let indexed = scan_assets(pack_root)?;
    let indexed_by_source = indexed
        .iter()
        .map(|asset| (asset.source_path.clone(), asset))
        .collect::<BTreeMap<_, _>>();

    let filter: Option<HashSet<String>> =
        asset_ids.map(|ids| ids.into_iter().collect::<HashSet<_>>());

    if let Some(filter) = &filter {
        let any_known =
            loaded.pack.assets.iter().any(|asset| {
                filter.contains(&asset.asset_id) || filter.contains(&asset.source_path)
            });
        if !any_known {
            let unknown = filter
                .iter()
                .next()
                .cloned()
                .unwrap_or_else(|| "<empty>".to_owned());
            return Err(IoError::UnknownAsset { asset_id: unknown });
        }
    }

    let mut accepted = Vec::new();
    for asset in &mut loaded.pack.assets {
        if let Some(filter) = &filter {
            if !filter.contains(&asset.asset_id) && !filter.contains(&asset.source_path) {
                continue;
            }
        }

        let Some(indexed_asset) = indexed_by_source.get(&asset.source_path) else {
            continue;
        };
        if indexed_asset.content_hash == asset.content_hash {
            continue;
        }

        asset.content_hash = indexed_asset.content_hash.clone();
        if clear_connectors {
            asset.connectors.clear();
        }
        // Re-measure bounds after content change when possible.
        let _ = apply_measured_bounds(asset, &indexed_asset.absolute_path);
        accepted.push(asset.source_path.clone());
    }

    if accepted.is_empty() {
        return Err(IoError::NoDriftedAssets);
    }

    write_pack_sidecar(pack_root, &loaded.pack)?;

    // Reconcile report after write for callers that want full status.
    let report = index_pack_folder_report_only(pack_root, &loaded.pack, &indexed)?;
    Ok(IndexReport {
        sidecar_path: canonical_sidecar_path(pack_root)
            .to_string_lossy()
            .into_owned(),
        discovered_assets: report.discovered_assets,
        new_assets: report.new_assets,
        unchanged_assets: report.unchanged_assets,
        drifted_assets: report.drifted_assets,
        missing_assets: report.missing_assets,
    })
}

fn index_pack_folder_report_only(
    pack_root: &Path,
    pack: &PackRecord,
    indexed: &[IndexedAsset],
) -> Result<IndexReport, IoError> {
    let indexed_by_source = indexed
        .iter()
        .map(|asset| (asset.source_path.clone(), asset))
        .collect::<BTreeMap<_, _>>();
    let existing_sources = pack
        .assets
        .iter()
        .map(|asset| asset.source_path.clone())
        .collect::<BTreeSet<_>>();

    let mut unchanged_assets = Vec::new();
    let mut drifted_assets = Vec::new();
    let mut missing_assets = Vec::new();
    let mut new_assets = Vec::new();

    for asset in &pack.assets {
        match indexed_by_source.get(&asset.source_path) {
            Some(indexed_asset) if indexed_asset.content_hash == asset.content_hash => {
                unchanged_assets.push(asset.source_path.clone());
            }
            Some(_) => drifted_assets.push(asset.source_path.clone()),
            None => missing_assets.push(asset.source_path.clone()),
        }
    }
    for indexed_asset in indexed {
        if !existing_sources.contains(&indexed_asset.source_path) {
            new_assets.push(indexed_asset.source_path.clone());
        }
    }

    Ok(IndexReport {
        sidecar_path: canonical_sidecar_path(pack_root)
            .to_string_lossy()
            .into_owned(),
        discovered_assets: sorted_sources(indexed.iter().map(|a| a.source_path.clone())),
        new_assets: sorted_sources(new_assets),
        unchanged_assets: sorted_sources(unchanged_assets),
        drifted_assets: sorted_sources(drifted_assets),
        missing_assets: sorted_sources(missing_assets),
    })
}

/// Outcome of measuring bounds for every pack asset that has a source file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MeasureBoundsReport {
    pub sidecar_path: String,
    /// Source paths where real bounds were written and `BoundsPlaceholder` cleared.
    pub measured: Vec<String>,
    /// Source paths present on disk but not measurable (e.g. FBX, empty mesh).
    pub failed: Vec<String>,
    /// Sidecar assets with no on-disk source (or skipped).
    pub missing: Vec<String>,
}

/// Measure bounds for every asset in the pack that has a readable source file.
pub fn measure_pack_bounds(pack_root: impl AsRef<Path>) -> Result<MeasureBoundsReport, IoError> {
    let pack_root = pack_root.as_ref();
    let mut loaded = read_pack_from_input(pack_root)?;
    let indexed = scan_assets(pack_root)?;
    let indexed_by_source = indexed
        .iter()
        .map(|asset| (asset.source_path.clone(), asset))
        .collect::<BTreeMap<_, _>>();

    let mut measured = Vec::new();
    let mut failed = Vec::new();
    let mut missing = Vec::new();

    for asset in &mut loaded.pack.assets {
        match indexed_by_source.get(&asset.source_path) {
            Some(indexed_asset) => {
                // Per-asset errors (corrupt glTF, unreadable file) must not abort
                // the pack: classify as failed and continue so siblings still measure.
                match apply_measured_bounds(asset, &indexed_asset.absolute_path) {
                    Ok(true) => measured.push(asset.source_path.clone()),
                    Ok(false) | Err(_) => failed.push(asset.source_path.clone()),
                }
            }
            None => missing.push(asset.source_path.clone()),
        }
    }

    write_pack_sidecar(pack_root, &loaded.pack)?;
    Ok(MeasureBoundsReport {
        sidecar_path: canonical_sidecar_path(pack_root)
            .to_string_lossy()
            .into_owned(),
        measured: sorted_sources(measured),
        failed: sorted_sources(failed),
        missing: sorted_sources(missing),
    })
}

/// Measure bounds (best-effort) then auto-propose connectors, classes, and rules.
pub fn analyze_pack_folder(
    pack_root: impl AsRef<Path>,
    options: AnalyzeOptions,
) -> Result<AnalyzeReport, IoError> {
    let pack_root = pack_root.as_ref();
    // Prefer fresh bounds before proposing face connectors.
    let _ = measure_pack_bounds(pack_root);
    let mut loaded = read_pack_from_input(pack_root)?;
    let report = analyze_pack(&mut loaded.pack, &options);
    write_pack_sidecar(pack_root, &loaded.pack)?;
    Ok(report)
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

fn pack_id_from_display_name(display_name: &str) -> String {
    let slug = slug_from_text(display_name);
    if slug.is_empty() {
        "pack".to_owned()
    } else {
        slug
    }
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
