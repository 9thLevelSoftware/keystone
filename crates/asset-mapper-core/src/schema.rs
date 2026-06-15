pub const CURRENT_SCHEMA_VERSION: u32 = 1;

pub type Vec2 = [f32; 2];
pub type Vec3 = [f32; 3];
pub type QuatXyzw = [f32; 4];

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub struct PackRecord {
    pub schema_version: u32,
    pub pack_id: String,
    pub display_name: String,
    pub coordinate_convention: CoordinateConvention,
    pub default_units: Unit,
    pub connector_classes: Vec<ConnectorClass>,
    pub compatibility_rules: Vec<CompatibilityRule>,
    pub assets: Vec<AssetRecord>,
}

#[derive(
    Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
pub struct CoordinateConvention {
    pub handedness: Handedness,
    pub up_axis: Axis3,
    pub forward_axis: Axis3,
}

#[derive(
    Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum Handedness {
    Right,
    Left,
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum Axis3 {
    PosX,
    NegX,
    PosY,
    NegY,
    PosZ,
    NegZ,
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum Unit {
    Meters,
    Centimeters,
    Pixels,
}

#[derive(
    Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
pub struct ConnectorClass {
    pub class: String,
    pub display_name: String,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub struct CompatibilityRule {
    pub a_class: String,
    pub b_class: String,
    pub rotation: AllowedRotation,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum AllowedRotation {
    Locked,
    StepsDeg { values: Vec<f32> },
    Free,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub struct AssetRecord {
    pub asset_id: String,
    pub source_path: String,
    pub content_hash: String,
    pub display_name: String,
    pub asset_type: AssetType,
    pub bounds: Bounds3,
    pub dimensions: Vec3,
    pub pivot: Pivot,
    pub up_axis: Axis3,
    pub forward_axis: Axis3,
    pub semantic_tags: Vec<String>,
    pub affordances: Vec<String>,
    pub placement_constraints: Vec<String>,
    pub connectors: Vec<ConnectorRecord>,
}

#[derive(
    Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum AssetType {
    Model3d,
    Sprite2d,
    Tile2d,
}

#[derive(
    Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
pub struct Bounds3 {
    pub min: Vec3,
    pub max: Vec3,
}

#[derive(
    Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum Pivot {
    Origin,
    BaseCenter,
    Center,
    Custom,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub struct ConnectorRecord {
    pub connector_id: String,
    pub display_name: String,
    pub class: String,
    pub role: ConnectorRole,
    pub frame: ConnectorFrame,
    pub mating_axis: Axis3,
    pub up_reference: Axis3,
    pub snap_tolerance: f32,
}

#[derive(
    Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum ConnectorRole {
    Symmetric,
    Plug,
    Receptacle,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ConnectorFrame {
    Frame3d {
        position: Vec3,
        orientation_quat_xyzw: QuatXyzw,
    },
    Frame2d {
        position: Vec2,
        normal: Vec2,
        grid_cell: Option<[i32; 2]>,
    },
}

#[derive(
    Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
pub struct Transform3d {
    pub translation: Vec3,
    pub rotation_quat_xyzw: QuatXyzw,
}

impl Transform3d {
    pub fn identity() -> Self {
        Self {
            translation: [0.0, 0.0, 0.0],
            rotation_quat_xyzw: [0.0, 0.0, 0.0, 1.0],
        }
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub struct AssemblyPlan {
    pub root_asset_id: String,
    pub operations: Vec<AssemblyOperation>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub struct AssemblyOperation {
    pub placed_asset_id: String,
    pub placed_connector_id: String,
    pub anchor_asset_id: String,
    pub anchor_connector_id: String,
    pub rotation_choice_deg: Option<f32>,
}
