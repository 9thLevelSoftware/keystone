use std::collections::HashMap;
use std::f32::consts::PI;

use glam::{Quat, Vec3};

use crate::schema::{
    AllowedRotation, AssemblyPlan, AssetRecord, CompatibilityRule, ConnectorFrame, ConnectorRecord,
    PackRecord, Transform3d,
};

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub struct AssetPlacement {
    pub asset_id: String,
    pub transform: Transform3d,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub struct ResolvedScene {
    pub placements: Vec<AssetPlacement>,
}

#[derive(Debug, thiserror::Error)]
pub enum ResolveError {
    #[error("root asset `{root_asset_id}` does not exist in the pack")]
    UnknownRootAsset { root_asset_id: String },

    #[error("placed asset `{asset_id}` does not exist in the pack")]
    UnknownPlacedAsset { asset_id: String },

    #[error("anchor asset `{anchor_asset_id}` has not been placed")]
    AnchorAssetNotPlaced { anchor_asset_id: String },

    #[error("asset `{asset_id}` does not have connector `{connector_id}`")]
    UnknownConnector {
        asset_id: String,
        connector_id: String,
    },

    #[error("connector `{connector_id}` on asset `{asset_id}` is not a 3D frame")]
    Non3dConnector {
        asset_id: String,
        connector_id: String,
    },

    #[error("connector classes `{placed_class}` and `{anchor_class}` are incompatible")]
    IncompatibleConnectorClasses {
        placed_class: String,
        anchor_class: String,
    },

    #[error("rotation choice {choice} is not permitted")]
    RotationChoiceNotAllowed { choice: f32 },
}

pub fn resolve_plan(pack: &PackRecord, plan: &AssemblyPlan) -> Result<ResolvedScene, ResolveError> {
    let root_asset =
        find_asset(pack, &plan.root_asset_id).ok_or_else(|| ResolveError::UnknownRootAsset {
            root_asset_id: plan.root_asset_id.clone(),
        })?;

    let mut placements_by_asset_id = HashMap::new();
    placements_by_asset_id.insert(root_asset.asset_id.clone(), Pose3::identity());

    let mut placements = vec![AssetPlacement {
        asset_id: root_asset.asset_id.clone(),
        transform: Transform3d::identity(),
    }];

    for operation in &plan.operations {
        let placed_asset = find_asset(pack, &operation.placed_asset_id).ok_or_else(|| {
            ResolveError::UnknownPlacedAsset {
                asset_id: operation.placed_asset_id.clone(),
            }
        })?;
        let anchor_asset = find_asset(pack, &operation.anchor_asset_id).ok_or_else(|| {
            ResolveError::UnknownPlacedAsset {
                asset_id: operation.anchor_asset_id.clone(),
            }
        })?;

        let anchor_asset_pose = *placements_by_asset_id
            .get(&operation.anchor_asset_id)
            .ok_or_else(|| ResolveError::AnchorAssetNotPlaced {
                anchor_asset_id: operation.anchor_asset_id.clone(),
            })?;

        let placed_connector = find_connector(placed_asset, &operation.placed_connector_id)?;
        let anchor_connector = find_connector(anchor_asset, &operation.anchor_connector_id)?;

        let rule = find_compatibility_rule(
            &pack.compatibility_rules,
            &placed_connector.class,
            &anchor_connector.class,
        )
        .ok_or_else(|| ResolveError::IncompatibleConnectorClasses {
            placed_class: placed_connector.class.clone(),
            anchor_class: anchor_connector.class.clone(),
        })?;

        validate_rotation_choice(&rule.rotation, operation.rotation_choice_deg)?;

        let placed_connector_local = connector_pose(placed_asset, placed_connector)?;
        let anchor_connector_local = connector_pose(anchor_asset, anchor_connector)?;
        let anchor_connector_world = anchor_asset_pose.then(anchor_connector_local);

        let flip = Pose3 {
            translation: Vec3::ZERO,
            rotation: Quat::from_rotation_y(PI),
        };
        let roll = Pose3 {
            translation: Vec3::ZERO,
            rotation: Quat::from_rotation_z(
                operation.rotation_choice_deg.unwrap_or(0.0).to_radians(),
            ),
        };

        let desired_placed_connector_world = anchor_connector_world.then(flip).then(roll);
        let placed_asset_world =
            desired_placed_connector_world.then(placed_connector_local.inverse());

        placements_by_asset_id.insert(operation.placed_asset_id.clone(), placed_asset_world);
        placements.push(AssetPlacement {
            asset_id: operation.placed_asset_id.clone(),
            transform: placed_asset_world.into_transform(),
        });
    }

    Ok(ResolvedScene { placements })
}

fn find_asset<'a>(pack: &'a PackRecord, asset_id: &str) -> Option<&'a AssetRecord> {
    pack.assets.iter().find(|asset| asset.asset_id == asset_id)
}

fn find_connector<'a>(
    asset: &'a AssetRecord,
    connector_id: &str,
) -> Result<&'a ConnectorRecord, ResolveError> {
    asset
        .connectors
        .iter()
        .find(|connector| connector.connector_id == connector_id)
        .ok_or_else(|| ResolveError::UnknownConnector {
            asset_id: asset.asset_id.clone(),
            connector_id: connector_id.to_owned(),
        })
}

fn find_compatibility_rule<'a>(
    rules: &'a [CompatibilityRule],
    placed_class: &str,
    anchor_class: &str,
) -> Option<&'a CompatibilityRule> {
    rules.iter().find(|rule| {
        (rule.a_class == placed_class && rule.b_class == anchor_class)
            || (rule.a_class == anchor_class && rule.b_class == placed_class)
    })
}

fn validate_rotation_choice(
    allowed_rotation: &AllowedRotation,
    rotation_choice_deg: Option<f32>,
) -> Result<(), ResolveError> {
    let choice = rotation_choice_deg.unwrap_or(0.0);
    match allowed_rotation {
        AllowedRotation::Locked => {
            if choice.abs() < 0.001 {
                Ok(())
            } else {
                Err(ResolveError::RotationChoiceNotAllowed { choice })
            }
        }
        AllowedRotation::StepsDeg { values } => {
            if values.iter().any(|value| (*value - choice).abs() < 0.001) {
                Ok(())
            } else {
                Err(ResolveError::RotationChoiceNotAllowed { choice })
            }
        }
        AllowedRotation::Free => Ok(()),
    }
}

fn connector_pose(asset: &AssetRecord, connector: &ConnectorRecord) -> Result<Pose3, ResolveError> {
    match connector.frame {
        ConnectorFrame::Frame3d {
            position,
            orientation_quat_xyzw,
        } => Ok(Pose3 {
            translation: Vec3::from_array(position),
            rotation: Quat::from_array(orientation_quat_xyzw).normalize(),
        }),
        ConnectorFrame::Frame2d { .. } => Err(ResolveError::Non3dConnector {
            asset_id: asset.asset_id.clone(),
            connector_id: connector.connector_id.clone(),
        }),
    }
}

#[derive(Debug, Clone, Copy)]
struct Pose3 {
    translation: Vec3,
    rotation: Quat,
}

impl Pose3 {
    fn identity() -> Self {
        Self {
            translation: Vec3::ZERO,
            rotation: Quat::IDENTITY,
        }
    }

    fn then(self, child: Pose3) -> Self {
        Self {
            translation: self.translation + self.rotation * child.translation,
            rotation: (self.rotation * child.rotation).normalize(),
        }
    }

    fn inverse(self) -> Self {
        let inverse_rotation = self.rotation.inverse();
        Self {
            translation: inverse_rotation * -self.translation,
            rotation: inverse_rotation,
        }
    }

    fn into_transform(self) -> Transform3d {
        Transform3d {
            translation: self.translation.to_array(),
            rotation_quat_xyzw: self.rotation.normalize().to_array(),
        }
    }
}
