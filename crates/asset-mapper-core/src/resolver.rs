use std::collections::HashMap;

use glam::{Mat3, Quat, Vec3};

use crate::schema::{
    AllowedRotation, AssemblyPlan, AssetRecord, Axis3, CompatibilityRule, ConnectorFrame,
    ConnectorRecord, PackRecord, Transform3d,
};

const AXIS_EPSILON: f32 = 0.000_001;
const QUATERNION_EPSILON: f32 = 0.000_001;

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub struct AssetPlacement {
    pub asset_id: String,
    pub transform: Transform3d,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub struct ResolvedScene {
    pub placements: Vec<AssetPlacement>,
}

/// Whether the plan author or pack author should fix a resolve failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResolveFixTarget {
    /// Compatibility rules, connector geometry, or pack metadata need changes.
    FixPack,
    /// Plan references, order, or rotation choices need changes.
    FixPlan,
}

/// Structured guidance for tooling when resolve fails.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ResolveErrorReport {
    pub code: String,
    pub message: String,
    pub fix_target: ResolveFixTarget,
    pub guidance: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub asset_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub connector_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub secondary_asset_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub secondary_connector_id: Option<String>,
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

    #[error("connector `{connector_id}` on asset `{asset_id}` is not a 2D frame")]
    Non2dConnector {
        asset_id: String,
        connector_id: String,
    },

    #[error(
        "connector frame kinds differ for `{}` and `{}`",
        .0.placed_connector_id, .0.anchor_connector_id
    )]
    MixedConnectorDimensions(Box<MateEndpoints>),

    #[error("connector classes `{placed_class}` and `{anchor_class}` are incompatible")]
    IncompatibleConnectorClasses {
        placed_class: String,
        anchor_class: String,
        endpoints: Box<MateEndpoints>,
    },

    #[error("rotation choice {choice} is not permitted")]
    RotationChoiceNotAllowed {
        choice: f32,
        endpoints: Box<MateEndpoints>,
    },

    #[error("connector `{connector_id}` on asset `{asset_id}` has invalid mating/up axes")]
    InvalidConnectorAxes {
        asset_id: String,
        connector_id: String,
    },

    #[error("connector `{connector_id}` on asset `{asset_id}` has invalid connector orientation")]
    InvalidConnectorOrientation {
        asset_id: String,
        connector_id: String,
    },

    #[error("connector `{connector_id}` on asset `{asset_id}` has a zero-length 2D normal")]
    Invalid2dNormal {
        asset_id: String,
        connector_id: String,
    },

    #[error("asset `{asset_id}` is already placed in this plan (unique assets only)")]
    DuplicatePlacedAsset { asset_id: String },
}

/// Placed/anchor asset + connector ids for resolve failures (boxed in error variants).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MateEndpoints {
    pub placed_asset_id: String,
    pub placed_connector_id: String,
    pub anchor_asset_id: String,
    pub anchor_connector_id: String,
}

impl MateEndpoints {
    pub fn new(
        placed_asset_id: impl Into<String>,
        placed_connector_id: impl Into<String>,
        anchor_asset_id: impl Into<String>,
        anchor_connector_id: impl Into<String>,
    ) -> Self {
        Self {
            placed_asset_id: placed_asset_id.into(),
            placed_connector_id: placed_connector_id.into(),
            anchor_asset_id: anchor_asset_id.into(),
            anchor_connector_id: anchor_connector_id.into(),
        }
    }
}

impl ResolveError {
    /// Stable machine-readable code (snake_case).
    pub fn code(&self) -> &'static str {
        match self {
            Self::UnknownRootAsset { .. } => "unknown_root_asset",
            Self::UnknownPlacedAsset { .. } => "unknown_placed_asset",
            Self::UnknownAnchorAsset { .. } => "unknown_anchor_asset",
            Self::AnchorAssetNotPlaced { .. } => "anchor_asset_not_placed",
            Self::UnknownConnector { .. } => "unknown_connector",
            Self::Non3dConnector { .. } => "non_3d_connector",
            Self::Non2dConnector { .. } => "non_2d_connector",
            Self::MixedConnectorDimensions(_) => "mixed_connector_dimensions",
            Self::IncompatibleConnectorClasses { .. } => "incompatible_connector_classes",
            Self::RotationChoiceNotAllowed { .. } => "rotation_choice_not_allowed",
            Self::InvalidConnectorAxes { .. } => "invalid_connector_axes",
            Self::InvalidConnectorOrientation { .. } => "invalid_connector_orientation",
            Self::Invalid2dNormal { .. } => "invalid_2d_normal",
            Self::DuplicatePlacedAsset { .. } => "duplicate_placed_asset",
        }
    }

    pub fn fix_target(&self) -> ResolveFixTarget {
        match self {
            Self::UnknownRootAsset { .. }
            | Self::UnknownPlacedAsset { .. }
            | Self::UnknownAnchorAsset { .. }
            | Self::AnchorAssetNotPlaced { .. }
            | Self::UnknownConnector { .. }
            | Self::MixedConnectorDimensions(_)
            | Self::RotationChoiceNotAllowed { .. }
            | Self::IncompatibleConnectorClasses { .. }
            | Self::DuplicatePlacedAsset { .. } => ResolveFixTarget::FixPlan,
            Self::Non3dConnector { .. }
            | Self::Non2dConnector { .. }
            | Self::InvalidConnectorAxes { .. }
            | Self::InvalidConnectorOrientation { .. }
            | Self::Invalid2dNormal { .. } => ResolveFixTarget::FixPack,
        }
    }

    pub fn guidance(&self) -> String {
        match self {
            Self::UnknownRootAsset { .. } => {
                "Set root_asset_id to an asset_id that exists in the pack.".to_owned()
            }
            Self::UnknownPlacedAsset { .. } => {
                "Use placed_asset_id values from the pack asset list.".to_owned()
            }
            Self::UnknownAnchorAsset { .. } => {
                "Use anchor_asset_id values from the pack asset list.".to_owned()
            }
            Self::AnchorAssetNotPlaced { .. } => {
                "Order operations so the anchor is the root or was placed earlier.".to_owned()
            }
            Self::UnknownConnector { .. } => {
                "Pick connector_ids from the asset's connectors (bundle or pack sidecar)."
                    .to_owned()
            }
            Self::Non3dConnector { .. } | Self::Non2dConnector { .. } => {
                "Mate connectors of the same frame kind (both frame3d or both frame2d).".to_owned()
            }
            Self::MixedConnectorDimensions(_) => {
                "Do not mix 2D and 3D connectors in one attach operation.".to_owned()
            }
            Self::IncompatibleConnectorClasses {
                placed_class,
                anchor_class,
                ..
            } => {
                format!(
                    "Pick a connector pair whose classes are covered by a compatibility rule \
(classes `{placed_class}` and `{anchor_class}` are not paired). Optionally add a pack rule \
if this pairing should be allowed."
                )
            }
            Self::RotationChoiceNotAllowed { .. } => {
                "Use rotation_choice_deg allowed by the rule (locked/steps_deg/free).".to_owned()
            }
            Self::InvalidConnectorAxes { .. } => {
                "Fix mating_axis / up_reference so they are non-parallel unit axes.".to_owned()
            }
            Self::InvalidConnectorOrientation { .. } => {
                "Fix the connector orientation quaternion (must be unit, non-zero).".to_owned()
            }
            Self::Invalid2dNormal { .. } => "Set a non-zero 2D connector normal.".to_owned(),
            Self::DuplicatePlacedAsset { .. } => {
                "Each asset_id may appear at most once per plan (root or placed). \
For repeated tiles, place extra instances outside Keystone."
                    .to_owned()
            }
        }
    }

    /// Primary asset implicated in the error (for editor selection).
    pub fn primary_asset_id(&self) -> Option<&str> {
        match self {
            Self::UnknownRootAsset { root_asset_id } => Some(root_asset_id),
            Self::UnknownPlacedAsset { asset_id }
            | Self::DuplicatePlacedAsset { asset_id }
            | Self::UnknownConnector { asset_id, .. }
            | Self::Non3dConnector { asset_id, .. }
            | Self::Non2dConnector { asset_id, .. }
            | Self::InvalidConnectorAxes { asset_id, .. }
            | Self::InvalidConnectorOrientation { asset_id, .. }
            | Self::Invalid2dNormal { asset_id, .. } => Some(asset_id),
            Self::UnknownAnchorAsset { anchor_asset_id }
            | Self::AnchorAssetNotPlaced { anchor_asset_id } => Some(anchor_asset_id),
            Self::MixedConnectorDimensions(ep) => Some(&ep.placed_asset_id),
            Self::IncompatibleConnectorClasses { endpoints, .. }
            | Self::RotationChoiceNotAllowed { endpoints, .. } => Some(&endpoints.placed_asset_id),
        }
    }

    pub fn primary_connector_id(&self) -> Option<&str> {
        match self {
            Self::UnknownConnector { connector_id, .. }
            | Self::Non3dConnector { connector_id, .. }
            | Self::Non2dConnector { connector_id, .. }
            | Self::InvalidConnectorAxes { connector_id, .. }
            | Self::InvalidConnectorOrientation { connector_id, .. }
            | Self::Invalid2dNormal { connector_id, .. } => Some(connector_id),
            Self::MixedConnectorDimensions(ep) => Some(&ep.placed_connector_id),
            Self::IncompatibleConnectorClasses { endpoints, .. }
            | Self::RotationChoiceNotAllowed { endpoints, .. } => {
                Some(&endpoints.placed_connector_id)
            }
            _ => None,
        }
    }

    pub fn secondary_asset_id(&self) -> Option<&str> {
        match self {
            Self::MixedConnectorDimensions(ep) => Some(&ep.anchor_asset_id),
            Self::IncompatibleConnectorClasses { endpoints, .. }
            | Self::RotationChoiceNotAllowed { endpoints, .. } => Some(&endpoints.anchor_asset_id),
            _ => None,
        }
    }

    pub fn secondary_connector_id(&self) -> Option<&str> {
        match self {
            Self::MixedConnectorDimensions(ep) => Some(&ep.anchor_connector_id),
            Self::IncompatibleConnectorClasses { endpoints, .. }
            | Self::RotationChoiceNotAllowed { endpoints, .. } => {
                Some(&endpoints.anchor_connector_id)
            }
            _ => None,
        }
    }

    pub fn to_report(&self) -> ResolveErrorReport {
        ResolveErrorReport {
            code: self.code().to_owned(),
            message: self.to_string(),
            fix_target: self.fix_target(),
            guidance: self.guidance(),
            asset_id: self.primary_asset_id().map(str::to_owned),
            connector_id: self.primary_connector_id().map(str::to_owned),
            secondary_asset_id: self.secondary_asset_id().map(str::to_owned),
            secondary_connector_id: self.secondary_connector_id().map(str::to_owned),
        }
    }
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

        // Unique-asset plan contract: each asset_id appears at most once (root or placed).
        if placements_by_asset_id.contains_key(&operation.placed_asset_id) {
            return Err(ResolveError::DuplicatePlacedAsset {
                asset_id: operation.placed_asset_id.clone(),
            });
        }

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
            endpoints: Box::new(MateEndpoints::new(
                operation.placed_asset_id.clone(),
                operation.placed_connector_id.clone(),
                operation.anchor_asset_id.clone(),
                operation.anchor_connector_id.clone(),
            )),
        })?;

        let mate_endpoints = MateEndpoints::new(
            operation.placed_asset_id.clone(),
            operation.placed_connector_id.clone(),
            operation.anchor_asset_id.clone(),
            operation.anchor_connector_id.clone(),
        );
        let rotation_choice_deg = validate_rotation_choice(
            &rule.rotation,
            operation.rotation_choice_deg,
            &mate_endpoints,
        )?;

        let placed_asset_world = match (&placed_connector.frame, &anchor_connector.frame) {
            (ConnectorFrame::Frame3d { .. }, ConnectorFrame::Frame3d { .. }) => {
                let placed_connector_local = connector_pose_3d(placed_asset, placed_connector)?;
                let anchor_connector_local = connector_pose_3d(anchor_asset, anchor_connector)?;
                let anchor_connector_world = anchor_asset_pose.then(anchor_connector_local);
                let desired_placed_connector_world = desired_connector_world_pose(
                    anchor_asset,
                    anchor_connector,
                    anchor_connector_world,
                    placed_asset,
                    placed_connector,
                    rotation_choice_deg,
                )?;
                desired_placed_connector_world.then(placed_connector_local.inverse())
            }
            (ConnectorFrame::Frame2d { .. }, ConnectorFrame::Frame2d { .. }) => {
                resolve_2d_attachment(
                    placed_asset,
                    placed_connector,
                    anchor_asset,
                    anchor_connector,
                    anchor_asset_pose,
                    rotation_choice_deg,
                )?
            }
            _ => {
                return Err(ResolveError::MixedConnectorDimensions(Box::new(
                    MateEndpoints::new(
                        operation.placed_asset_id.clone(),
                        placed_connector.connector_id.clone(),
                        operation.anchor_asset_id.clone(),
                        anchor_connector.connector_id.clone(),
                    ),
                )));
            }
        };

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
    endpoints: &MateEndpoints,
) -> Result<f32, ResolveError> {
    let choice = rotation_choice_deg.unwrap_or(0.0);
    let not_allowed = |choice: f32| ResolveError::RotationChoiceNotAllowed {
        choice,
        endpoints: Box::new(endpoints.clone()),
    };
    if !choice.is_finite() {
        return Err(not_allowed(choice));
    }

    match allowed_rotation {
        AllowedRotation::Locked => {
            if choice.abs() < 0.001 {
                Ok(choice)
            } else {
                Err(not_allowed(choice))
            }
        }
        AllowedRotation::StepsDeg { values } => {
            if values
                .iter()
                .any(|value| angles_nearly_equal_deg(*value, choice))
            {
                Ok(choice)
            } else {
                Err(not_allowed(choice))
            }
        }
        AllowedRotation::Free => Ok(choice),
    }
}

fn connector_pose_3d(
    asset: &AssetRecord,
    connector: &ConnectorRecord,
) -> Result<Pose3, ResolveError> {
    match &connector.frame {
        ConnectorFrame::Frame3d {
            position,
            orientation_quat_xyzw,
        } => {
            let length_squared = orientation_quat_xyzw
                .iter()
                .map(|component| component * component)
                .sum::<f32>();
            if !length_squared.is_finite() || length_squared <= QUATERNION_EPSILON {
                return Err(ResolveError::InvalidConnectorOrientation {
                    asset_id: asset.asset_id.clone(),
                    connector_id: connector.connector_id.clone(),
                });
            }

            Ok(Pose3 {
                translation: Vec3::from_array(*position),
                rotation: Quat::from_array(*orientation_quat_xyzw).normalize(),
            })
        }
        ConnectorFrame::Frame2d { .. } => Err(ResolveError::Non3dConnector {
            asset_id: asset.asset_id.clone(),
            connector_id: connector.connector_id.clone(),
        }),
    }
}

/// Resolve a 2D Frame2d attachment into a 3D pose on the XY plane (Z = 0).
///
/// Normals face each other (placed normal maps to -anchor normal in world),
/// positions coincide, and `rotation_choice_deg` spins around Z after mating.
fn resolve_2d_attachment(
    placed_asset: &AssetRecord,
    placed_connector: &ConnectorRecord,
    anchor_asset: &AssetRecord,
    anchor_connector: &ConnectorRecord,
    anchor_asset_pose: Pose3,
    rotation_choice_deg: f32,
) -> Result<Pose3, ResolveError> {
    let (placed_pos, placed_normal) = frame_2d_parts(placed_asset, placed_connector)?;
    let (anchor_pos, anchor_normal) = frame_2d_parts(anchor_asset, anchor_connector)?;

    let anchor_normal_world = rotate_xy(anchor_asset_pose.rotation, anchor_normal);
    let anchor_pos_world = {
        let local = Vec3::new(anchor_pos[0], anchor_pos[1], 0.0);
        (anchor_asset_pose.translation + anchor_asset_pose.rotation * local).truncate()
    };

    let desired_normal_world = -anchor_normal_world;
    let placed_angle = normal_angle(placed_normal);
    let desired_angle = normal_angle(desired_normal_world) + rotation_choice_deg.to_radians();
    let delta = desired_angle - placed_angle;
    let rotation = Quat::from_rotation_z(delta);

    let placed_pos_local = Vec3::new(placed_pos[0], placed_pos[1], 0.0);
    let rotated_local = rotation * placed_pos_local;
    let translation = Vec3::new(
        anchor_pos_world.x - rotated_local.x,
        anchor_pos_world.y - rotated_local.y,
        0.0,
    );

    Ok(Pose3 {
        translation,
        rotation,
    })
}

fn frame_2d_parts(
    asset: &AssetRecord,
    connector: &ConnectorRecord,
) -> Result<([f32; 2], glam::Vec2), ResolveError> {
    match &connector.frame {
        ConnectorFrame::Frame2d {
            position, normal, ..
        } => {
            if !position[0].is_finite()
                || !position[1].is_finite()
                || !normal[0].is_finite()
                || !normal[1].is_finite()
            {
                return Err(ResolveError::Invalid2dNormal {
                    asset_id: asset.asset_id.clone(),
                    connector_id: connector.connector_id.clone(),
                });
            }
            let n = glam::Vec2::new(normal[0], normal[1]);
            if n.length_squared() <= AXIS_EPSILON {
                return Err(ResolveError::Invalid2dNormal {
                    asset_id: asset.asset_id.clone(),
                    connector_id: connector.connector_id.clone(),
                });
            }
            Ok((*position, n.normalize()))
        }
        ConnectorFrame::Frame3d { .. } => Err(ResolveError::Non2dConnector {
            asset_id: asset.asset_id.clone(),
            connector_id: connector.connector_id.clone(),
        }),
    }
}

/// Minimal angular distance on the circle, in degrees (0 ≡ 360).
fn angles_nearly_equal_deg(a: f32, b: f32) -> bool {
    if !a.is_finite() || !b.is_finite() {
        return false;
    }
    let diff = (a - b).rem_euclid(360.0);
    let distance = diff.min(360.0 - diff);
    distance < 0.001
}

fn rotate_xy(rotation: Quat, v: glam::Vec2) -> glam::Vec2 {
    let rotated = rotation * Vec3::new(v.x, v.y, 0.0);
    glam::Vec2::new(rotated.x, rotated.y).normalize_or_zero()
}

fn normal_angle(normal: glam::Vec2) -> f32 {
    normal.y.atan2(normal.x)
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
