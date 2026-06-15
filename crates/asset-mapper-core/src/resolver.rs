use std::collections::HashMap;

use glam::{Mat3, Quat, Vec3};

use crate::schema::{
    AllowedRotation, AssemblyPlan, AssetRecord, Axis3, CompatibilityRule, ConnectorFrame,
    ConnectorRecord, PackRecord, Transform3d,
};

const AXIS_EPSILON: f32 = 0.000_001;

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

    #[error("anchor asset `{anchor_asset_id}` does not exist in the pack")]
    UnknownAnchorAsset { anchor_asset_id: String },

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

    #[error("connector `{connector_id}` on asset `{asset_id}` has invalid mating/up axes")]
    InvalidConnectorAxes {
        asset_id: String,
        connector_id: String,
    },
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
            ResolveError::UnknownAnchorAsset {
                anchor_asset_id: operation.anchor_asset_id.clone(),
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

        let rotation_choice_deg =
            validate_rotation_choice(&rule.rotation, operation.rotation_choice_deg)?;

        let placed_connector_local = connector_pose(placed_asset, placed_connector)?;
        let anchor_connector_local = connector_pose(anchor_asset, anchor_connector)?;
        let anchor_connector_world = anchor_asset_pose.then(anchor_connector_local);
        let desired_placed_connector_world = desired_connector_world_pose(
            anchor_asset,
            anchor_connector,
            anchor_connector_world,
            placed_asset,
            placed_connector,
            rotation_choice_deg,
        )?;
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
) -> Result<f32, ResolveError> {
    let choice = rotation_choice_deg.unwrap_or(0.0);
    if !choice.is_finite() {
        return Err(ResolveError::RotationChoiceNotAllowed { choice });
    }

    match allowed_rotation {
        AllowedRotation::Locked => {
            if choice.abs() < 0.001 {
                Ok(choice)
            } else {
                Err(ResolveError::RotationChoiceNotAllowed { choice })
            }
        }
        AllowedRotation::StepsDeg { values } => {
            if values.iter().any(|value| (*value - choice).abs() < 0.001) {
                Ok(choice)
            } else {
                Err(ResolveError::RotationChoiceNotAllowed { choice })
            }
        }
        AllowedRotation::Free => Ok(choice),
    }
}

fn connector_pose(asset: &AssetRecord, connector: &ConnectorRecord) -> Result<Pose3, ResolveError> {
    match &connector.frame {
        ConnectorFrame::Frame3d {
            position,
            orientation_quat_xyzw,
        } => Ok(Pose3 {
            translation: Vec3::from_array(*position),
            rotation: Quat::from_array(*orientation_quat_xyzw).normalize(),
        }),
        ConnectorFrame::Frame2d { .. } => Err(ResolveError::Non3dConnector {
            asset_id: asset.asset_id.clone(),
            connector_id: connector.connector_id.clone(),
        }),
    }
}

fn desired_connector_world_pose(
    anchor_asset: &AssetRecord,
    anchor_connector: &ConnectorRecord,
    anchor_connector_world: Pose3,
    placed_asset: &AssetRecord,
    placed_connector: &ConnectorRecord,
    rotation_choice_deg: f32,
) -> Result<Pose3, ResolveError> {
    let anchor_mating_world =
        (anchor_connector_world.rotation * axis_to_vec(anchor_connector.mating_axis)).normalize();
    let anchor_up_world =
        anchor_connector_world.rotation * axis_to_vec(anchor_connector.up_reference);
    let desired_mating_world = -anchor_mating_world;
    let desired_up_world = project_axis(
        anchor_up_world,
        desired_mating_world,
        anchor_asset,
        anchor_connector,
    )?;
    let rolled_up_world =
        Quat::from_axis_angle(desired_mating_world, rotation_choice_deg.to_radians())
            * desired_up_world;

    Ok(Pose3 {
        translation: anchor_connector_world.translation,
        rotation: connector_rotation_from_axes(
            placed_asset,
            placed_connector,
            desired_mating_world,
            rolled_up_world,
        )?,
    })
}

fn connector_rotation_from_axes(
    asset: &AssetRecord,
    connector: &ConnectorRecord,
    desired_mating_world: Vec3,
    desired_up_world: Vec3,
) -> Result<Quat, ResolveError> {
    let local_mating = axis_to_vec(connector.mating_axis);
    let local_up = project_axis(
        axis_to_vec(connector.up_reference),
        local_mating,
        asset,
        connector,
    )?;
    let world_up = project_axis(desired_up_world, desired_mating_world, asset, connector)?;

    let local_basis = basis_from_mating_and_up(local_mating, local_up);
    let world_basis = basis_from_mating_and_up(desired_mating_world, world_up);
    Ok(Quat::from_mat3(&(world_basis * local_basis.transpose())).normalize())
}

fn basis_from_mating_and_up(mating: Vec3, up: Vec3) -> Mat3 {
    Mat3::from_cols(
        mating.normalize(),
        up.normalize(),
        mating.cross(up).normalize(),
    )
}

fn project_axis(
    axis: Vec3,
    plane_normal: Vec3,
    asset: &AssetRecord,
    connector: &ConnectorRecord,
) -> Result<Vec3, ResolveError> {
    let normal = plane_normal.normalize();
    let projected = axis - normal * axis.dot(normal);
    if projected.length_squared() <= AXIS_EPSILON {
        Err(ResolveError::InvalidConnectorAxes {
            asset_id: asset.asset_id.clone(),
            connector_id: connector.connector_id.clone(),
        })
    } else {
        Ok(projected.normalize())
    }
}

fn axis_to_vec(axis: Axis3) -> Vec3 {
    match axis {
        Axis3::PosX => Vec3::X,
        Axis3::NegX => -Vec3::X,
        Axis3::PosY => Vec3::Y,
        Axis3::NegY => -Vec3::Y,
        Axis3::PosZ => Vec3::Z,
        Axis3::NegZ => -Vec3::Z,
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
