//! Schema migration framework.
//!
//! Packs declare `schema_version`. This module upgrades older versions to
//! [`crate::schema::CURRENT_SCHEMA_VERSION`].

use crate::schema::{
    CURRENT_SCHEMA_VERSION, ControlledVocabulary, PLACEHOLDER_LICENSE_SUMMARY, PackProvenance,
    PackRecord, ReviewFlag,
};

#[derive(Debug, thiserror::Error)]
pub enum MigrationError {
    #[error("unsupported schema_version {found}; cannot migrate to {target}")]
    UnsupportedVersion { found: u32, target: u32 },

    #[error("pack is already at schema_version {version}")]
    AlreadyCurrent { version: u32 },
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct MigrationReport {
    pub from_version: u32,
    pub to_version: u32,
    pub steps: Vec<String>,
}

/// Migrate a pack in-memory to the current schema version.
///
/// Returns the migrated pack and a report of applied steps. Fails if the pack
/// is already current or the version is newer/unknown.
pub fn migrate_pack(mut pack: PackRecord) -> Result<(PackRecord, MigrationReport), MigrationError> {
    let from_version = pack.schema_version;
    if from_version == CURRENT_SCHEMA_VERSION {
        return Err(MigrationError::AlreadyCurrent {
            version: from_version,
        });
    }
    if from_version > CURRENT_SCHEMA_VERSION {
        return Err(MigrationError::UnsupportedVersion {
            found: from_version,
            target: CURRENT_SCHEMA_VERSION,
        });
    }

    let mut steps = Vec::new();
    let mut version = from_version;

    while version < CURRENT_SCHEMA_VERSION {
        match version {
            0 => {
                migrate_v0_to_v1(&mut pack);
                steps.push(
                    "v0→v1: dedupe review flags; tag unit-cube bounds as BoundsPlaceholder"
                        .to_owned(),
                );
                version = 1;
            }
            1 => {
                migrate_v1_to_v2(&mut pack);
                steps.push(
                    "v1→v2: add license_summary, provenance, and controlled vocabulary defaults"
                        .to_owned(),
                );
                version = 2;
            }
            other => {
                return Err(MigrationError::UnsupportedVersion {
                    found: other,
                    target: CURRENT_SCHEMA_VERSION,
                });
            }
        }
    }

    pack.schema_version = CURRENT_SCHEMA_VERSION;
    Ok((
        pack,
        MigrationReport {
            from_version,
            to_version: CURRENT_SCHEMA_VERSION,
            steps,
        },
    ))
}

/// Best-effort parse of a legacy v0 pack JSON value into a modern [`PackRecord`].
///
/// v0 is the pre-release draft: same fields as v1 with optional missing
/// `review_flags` / looser defaults. Unknown extra fields are ignored by serde.
pub fn pack_from_legacy_json(value: serde_json::Value) -> Result<PackRecord, String> {
    let mut value = value;
    if let Some(obj) = value.as_object_mut() {
        // Ensure schema_version is present for migration routing.
        if !obj.contains_key("schema_version") {
            obj.insert("schema_version".to_owned(), serde_json::json!(0));
        }
        if let Some(assets) = obj.get_mut("assets").and_then(|a| a.as_array_mut()) {
            for asset in assets {
                if let Some(asset_obj) = asset.as_object_mut() {
                    asset_obj
                        .entry("review_flags")
                        .or_insert_with(|| serde_json::json!([]));
                    asset_obj
                        .entry("semantic_tags")
                        .or_insert_with(|| serde_json::json!([]));
                    asset_obj
                        .entry("affordances")
                        .or_insert_with(|| serde_json::json!([]));
                    asset_obj
                        .entry("placement_constraints")
                        .or_insert_with(|| serde_json::json!([]));
                    asset_obj
                        .entry("connectors")
                        .or_insert_with(|| serde_json::json!([]));
                }
            }
        }
    }

    serde_json::from_value(value).map_err(|error| error.to_string())
}

fn migrate_v0_to_v1(pack: &mut PackRecord) {
    for asset in &mut pack.assets {
        // Drop unknown placeholder-only noise: ensure flags are unique.
        let mut seen = std::collections::HashSet::new();
        asset.review_flags.retain(|flag| seen.insert(flag.clone()));

        // If bounds look like the unit cube and no explicit clearance, keep
        // BoundsPlaceholder so authors re-measure.
        let is_unit_cube = asset.bounds.min == [-0.5, -0.5, -0.5]
            && asset.bounds.max == [0.5, 0.5, 0.5]
            && asset.dimensions == [1.0, 1.0, 1.0];
        if is_unit_cube && !asset.review_flags.contains(&ReviewFlag::BoundsPlaceholder) {
            asset.review_flags.push(ReviewFlag::BoundsPlaceholder);
        }
    }
}

fn migrate_v1_to_v2(pack: &mut PackRecord) {
    // Seed discoverable placeholders. Production validate still rejects
    // UNSPECIFIED licenses and notes-only provenance until authors fill them.
    if pack.license_summary.trim().is_empty() {
        pack.license_summary = PLACEHOLDER_LICENSE_SUMMARY.to_owned();
    }
    if pack.provenance.is_empty() {
        pack.provenance = PackProvenance {
            notes: Some("Migrated to schema v2; set source or author for production.".to_owned()),
            ..PackProvenance::default()
        };
    }
    let vocab_empty = pack.vocabulary.semantic_tags.is_empty()
        && pack.vocabulary.affordances.is_empty()
        && pack.vocabulary.placement_constraints.is_empty();
    if vocab_empty {
        pack.vocabulary = ControlledVocabulary::default();
    }
}
