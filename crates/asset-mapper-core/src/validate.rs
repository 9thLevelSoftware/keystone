use std::collections::HashSet;

use crate::diagnostics::{Diagnostic, ValidationReport};
use crate::schema::{CURRENT_SCHEMA_VERSION, ConnectorFrame, PackRecord, ReviewFlag};

const QUAT_NORMALIZED_EPSILON: f32 = 0.001;
const VECTOR_LENGTH_EPSILON: f32 = 0.0001;

pub fn validate_pack(pack: &PackRecord) -> ValidationReport {
    let mut diagnostics = Vec::new();

    if pack.schema_version != CURRENT_SCHEMA_VERSION {
        diagnostics.push(Diagnostic::error(
            "unsupported_schema_version",
            format!(
                "schema_version {} is not supported; expected {}",
                pack.schema_version, CURRENT_SCHEMA_VERSION
            ),
        ));
    }

    let mut class_names = HashSet::new();
    for class in &pack.connector_classes {
        if !class_names.insert(class.class.as_str()) {
            diagnostics.push(Diagnostic::error(
                "duplicate_connector_class",
                format!("connector class `{}` is duplicated", class.class),
            ));
        }
    }

    let mut classes_with_rules = HashSet::new();
    for rule in &pack.compatibility_rules {
        if !class_names.contains(rule.a_class.as_str()) {
            diagnostics.push(Diagnostic::error(
                "unknown_rule_class",
                format!(
                    "compatibility rule references unknown a_class `{}`",
                    rule.a_class
                ),
            ));
        }
        if !class_names.contains(rule.b_class.as_str()) {
            diagnostics.push(Diagnostic::error(
                "unknown_rule_class",
                format!(
                    "compatibility rule references unknown b_class `{}`",
                    rule.b_class
                ),
            ));
        }
        classes_with_rules.insert(rule.a_class.as_str());
        classes_with_rules.insert(rule.b_class.as_str());
    }

    for class in &pack.connector_classes {
        if !classes_with_rules.contains(class.class.as_str()) {
            diagnostics.push(Diagnostic::warning(
                "connector_class_has_no_rule",
                format!(
                    "connector class `{}` does not participate in any compatibility rule",
                    class.class
                ),
            ));
        }
    }

    let mut asset_ids = HashSet::new();
    let mut source_paths = HashSet::new();
    for asset in &pack.assets {
        if !asset_ids.insert(asset.asset_id.as_str()) {
            diagnostics.push(
                Diagnostic::error(
                    "duplicate_asset_id",
                    format!("asset_id `{}` is duplicated", asset.asset_id),
                )
                .with_asset(asset.asset_id.clone()),
            );
        }

        if !source_paths.insert(asset.source_path.as_str()) {
            diagnostics.push(
                Diagnostic::error(
                    "duplicate_source_path",
                    format!("source_path `{}` is duplicated", asset.source_path),
                )
                .with_asset(asset.asset_id.clone()),
            );
        }

        if asset.content_hash.trim().is_empty() {
            diagnostics.push(
                Diagnostic::error("missing_content_hash", "asset content_hash is empty")
                    .with_asset(asset.asset_id.clone()),
            );
        }

        if !validate_finite_vec3(&asset.dimensions) {
            diagnostics.push(
                Diagnostic::error("non_finite_dimensions", "asset dimensions must be finite")
                    .with_asset(asset.asset_id.clone()),
            );
        }

        let bounds_are_finite =
            validate_finite_vec3(&asset.bounds.min) && validate_finite_vec3(&asset.bounds.max);
        if !bounds_are_finite {
            diagnostics.push(
                Diagnostic::error(
                    "non_finite_bounds",
                    "asset bounds min and max must be finite",
                )
                .with_asset(asset.asset_id.clone()),
            );
        } else if !bounds_are_ordered(asset.bounds.min, asset.bounds.max) {
            diagnostics.push(
                Diagnostic::error(
                    "invalid_bounds",
                    "asset bounds min must be less than or equal to max on every axis",
                )
                .with_asset(asset.asset_id.clone()),
            );
        }

        validate_review_flags(
            &mut diagnostics,
            asset.asset_id.as_str(),
            asset.review_flags.as_slice(),
        );

        let mut connector_ids = HashSet::new();
        for connector in &asset.connectors {
            if !connector_ids.insert(connector.connector_id.as_str()) {
                diagnostics.push(
                    Diagnostic::error(
                        "duplicate_connector_id",
                        format!(
                            "connector_id `{}` is duplicated within asset `{}`",
                            connector.connector_id, asset.asset_id
                        ),
                    )
                    .with_asset(asset.asset_id.clone())
                    .with_connector(connector.connector_id.clone()),
                );
            }

            if !class_names.contains(connector.class.as_str()) {
                diagnostics.push(
                    Diagnostic::error(
                        "unknown_connector_class",
                        format!(
                            "connector `{}` references unknown class `{}`",
                            connector.connector_id, connector.class
                        ),
                    )
                    .with_asset(asset.asset_id.clone())
                    .with_connector(connector.connector_id.clone()),
                );
            }

            if !connector.snap_tolerance.is_finite() {
                diagnostics.push(
                    Diagnostic::error("non_finite_snap_tolerance", "snap_tolerance must be finite")
                        .with_asset(asset.asset_id.clone())
                        .with_connector(connector.connector_id.clone()),
                );
            } else if connector.snap_tolerance < 0.0 {
                diagnostics.push(
                    Diagnostic::error(
                        "negative_snap_tolerance",
                        "snap_tolerance must be zero or positive",
                    )
                    .with_asset(asset.asset_id.clone())
                    .with_connector(connector.connector_id.clone()),
                );
            }

            validate_connector_frame(
                &mut diagnostics,
                asset.asset_id.as_str(),
                connector.connector_id.as_str(),
                &connector.frame,
            );
        }
    }

    ValidationReport::new(diagnostics)
}

fn validate_connector_frame(
    diagnostics: &mut Vec<Diagnostic>,
    asset_id: &str,
    connector_id: &str,
    frame: &ConnectorFrame,
) {
    match frame {
        ConnectorFrame::Frame3d {
            orientation_quat_xyzw,
            ..
        } => {
            if !orientation_quat_xyzw
                .iter()
                .all(|component| component.is_finite())
            {
                diagnostics.push(
                    Diagnostic::error(
                        "non_finite_connector_quaternion",
                        "3D connector quaternion components must be finite",
                    )
                    .with_asset(asset_id.to_owned())
                    .with_connector(connector_id.to_owned()),
                );
                return;
            }
            let length_squared = orientation_quat_xyzw
                .iter()
                .map(|component| component * component)
                .sum::<f32>();
            if (length_squared - 1.0).abs() > QUAT_NORMALIZED_EPSILON {
                diagnostics.push(
                    Diagnostic::error(
                        "connector_quaternion_not_normalized",
                        format!(
                            "3D connector quaternion length squared was {}",
                            length_squared
                        ),
                    )
                    .with_asset(asset_id.to_owned())
                    .with_connector(connector_id.to_owned()),
                );
            }
        }
        ConnectorFrame::Frame2d { normal, .. } => {
            let length_squared = normal[0] * normal[0] + normal[1] * normal[1];
            if length_squared < VECTOR_LENGTH_EPSILON {
                diagnostics.push(
                    Diagnostic::error(
                        "connector_2d_normal_degenerate",
                        "2D connector normal must have non-zero length",
                    )
                    .with_asset(asset_id.to_owned())
                    .with_connector(connector_id.to_owned()),
                );
            }
        }
    }
}

fn bounds_are_ordered(min: [f32; 3], max: [f32; 3]) -> bool {
    min[0] <= max[0] && min[1] <= max[1] && min[2] <= max[2]
}

fn validate_finite_vec3(vector: &[f32; 3]) -> bool {
    vector.iter().all(|component| component.is_finite())
}

fn validate_review_flags(
    diagnostics: &mut Vec<Diagnostic>,
    asset_id: &str,
    review_flags: &[ReviewFlag],
) {
    for review_flag in review_flags {
        let (code, message) = match review_flag {
            ReviewFlag::BoundsPlaceholder => (
                "placeholder_bounds",
                "asset bounds are placeholder values and need review",
            ),
            ReviewFlag::OrientationPlaceholder => (
                "placeholder_orientation",
                "asset orientation is placeholder data and needs review",
            ),
            ReviewFlag::PivotPlaceholder => (
                "placeholder_pivot",
                "asset pivot is placeholder data and needs review",
            ),
        };
        diagnostics.push(Diagnostic::warning(code, message).with_asset(asset_id.to_owned()));
    }
}
