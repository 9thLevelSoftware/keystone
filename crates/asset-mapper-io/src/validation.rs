use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use asset_mapper_core::{Diagnostic, PackRecord, ValidationReport};

use crate::error::IoError;
use crate::index::scan_assets;

pub fn validate_pack_sources(
    pack_root: impl AsRef<Path>,
    pack: &PackRecord,
) -> Result<ValidationReport, IoError> {
    let indexed = scan_assets(pack_root)?;
    let indexed_by_source = indexed
        .iter()
        .map(|asset| (asset.source_path.as_str(), asset))
        .collect::<BTreeMap<_, _>>();

    let mut diagnostics = Vec::new();
    let mut metadata_sources = BTreeSet::new();

    for asset in &pack.assets {
        metadata_sources.insert(asset.source_path.as_str());
        match indexed_by_source.get(asset.source_path.as_str()) {
            Some(indexed_asset) if indexed_asset.content_hash == asset.content_hash => {}
            Some(_) => {
                diagnostics.push(
                    Diagnostic::warning(
                        "source_hash_drift",
                        format!(
                            "source file `{}` content hash differs from metadata",
                            asset.source_path
                        ),
                    )
                    .with_asset(asset.asset_id.clone()),
                );
            }
            None => {
                diagnostics.push(
                    Diagnostic::warning(
                        "source_file_missing",
                        format!(
                            "source file `{}` is referenced but missing",
                            asset.source_path
                        ),
                    )
                    .with_asset(asset.asset_id.clone()),
                );
            }
        }
    }

    for indexed_asset in &indexed {
        if !metadata_sources.contains(indexed_asset.source_path.as_str()) {
            diagnostics.push(Diagnostic::warning(
                "source_file_untracked",
                format!(
                    "source file `{}` is not represented in metadata",
                    indexed_asset.source_path
                ),
            ));
        }
    }

    Ok(ValidationReport::new(diagnostics))
}
