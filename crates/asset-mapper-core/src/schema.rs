pub const CURRENT_SCHEMA_VERSION: u32 = 2;

pub type Vec2 = [f32; 2];
pub type Vec3 = [f32; 3];
pub type QuatXyzw = [f32; 4];

/// Default controlled vocabulary from the product design (compact initial set).
pub fn default_semantic_tags() -> Vec<String> {
    [
        "wall",
        "floor",
        "corner",
        "door",
        "window",
        "walkable",
        "cover",
        "decorative",
        "hazard",
        "lootable",
        "entry",
        "exit",
        "corridor",
        "roof",
        "prop",
    ]
    .into_iter()
    .map(str::to_owned)
    .collect()
}

pub fn default_affordances() -> Vec<String> {
    [
        "block_movement",
        "provide_cover",
        "openable",
        "climbable",
        "sittable",
        "interactable",
        "light_source",
        "walkable",
    ]
    .into_iter()
    .map(str::to_owned)
    .collect()
}

pub fn default_placement_constraints() -> Vec<String> {
    [
        "grounded",
        "wall_mounted",
        "ceiling_mounted",
        "indoor_only",
        "outdoor_only",
        "requires_floor",
        "requires_wall",
        "upright_only",
    ]
    .into_iter()
    .map(str::to_owned)
    .collect()
}

/// Placeholder written by `init` / migrate when the author has not yet set a license.
///
/// Production validation rejects any `license_summary` that is empty or starts
/// with `UNSPECIFIED` (case-insensitive) so packs cannot ship on this sentinel.
pub const PLACEHOLDER_LICENSE_SUMMARY: &str =
    "UNSPECIFIED — set license_summary before distributing this pack";

/// Returns true when `summary` is non-empty and is not an `UNSPECIFIED` placeholder.
pub fn license_summary_is_production_ready(summary: &str) -> bool {
    let trimmed = summary.trim();
    !trimmed.is_empty() && !trimmed.to_ascii_uppercase().starts_with("UNSPECIFIED")
}

/// Pack-level provenance for auditability and redistribution context.
#[derive(
    Debug, Clone, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
pub struct PackProvenance {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub author: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub created_at: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
}

impl PackProvenance {
    pub fn is_empty(&self) -> bool {
        self.source.as_ref().is_none_or(|s| s.trim().is_empty())
            && self.author.as_ref().is_none_or(|s| s.trim().is_empty())
            && self.created_at.as_ref().is_none_or(|s| s.trim().is_empty())
            && self.notes.as_ref().is_none_or(|s| s.trim().is_empty())
    }

    /// Production packs must identify `source` and/or `author`.
    ///
    /// Notes or `created_at` alone (including migrate fillers) are not enough.
    pub fn meets_production_requirements(&self) -> bool {
        self.source.as_ref().is_some_and(|s| !s.trim().is_empty())
            || self.author.as_ref().is_some_and(|s| !s.trim().is_empty())
    }
}

/// Controlled vocabularies for semantic tags, affordances, and placement constraints.
///
/// Tags may use namespaced extensions (`project:custom_tag`) when
/// [`ControlledVocabulary::allow_namespaced_extensions`] is true.
#[derive(
    Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
pub struct ControlledVocabulary {
    #[serde(default)]
    pub semantic_tags: Vec<String>,
    #[serde(default)]
    pub affordances: Vec<String>,
    #[serde(default)]
    pub placement_constraints: Vec<String>,
    /// When true, tags containing `:` are allowed even if not listed.
    #[serde(default = "default_true")]
    pub allow_namespaced_extensions: bool,
}

fn default_true() -> bool {
    true
}

impl Default for ControlledVocabulary {
    fn default() -> Self {
        Self {
            semantic_tags: default_semantic_tags(),
            affordances: default_affordances(),
            placement_constraints: default_placement_constraints(),
            allow_namespaced_extensions: true,
        }
    }
}

impl ControlledVocabulary {
    pub fn allows_term(&self, list: &[String], term: &str) -> bool {
        let term = term.trim();
        if term.is_empty() {
            return false;
        }
        if list.iter().any(|entry| entry == term) {
            return true;
        }
        self.allow_namespaced_extensions
            && term.contains(':')
            && term.split_once(':').is_some_and(|(ns, rest)| {
                !ns.trim().is_empty() && !rest.trim().is_empty() && !rest.contains(':')
            })
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub struct PackRecord {
    pub schema_version: u32,
    pub pack_id: String,
    pub display_name: String,
    pub coordinate_convention: CoordinateConvention,
    pub default_units: Unit,
    /// Short human-readable license summary (required for production packs).
    #[serde(default)]
    pub license_summary: String,
    #[serde(default)]
    pub provenance: PackProvenance,
    #[serde(default)]
    pub vocabulary: ControlledVocabulary,
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
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub review_flags: Vec<ReviewFlag>,
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

#[derive(
    Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum ReviewFlag {
    BoundsPlaceholder,
    OrientationPlaceholder,
    PivotPlaceholder,
    /// Connectors were proposed from AABB faces because mesh sockets were unavailable.
    AutoFromBoundsFallback,
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
    /// Face-plane dimensions `[width, height]` in pack units for LLM / size pairing.
    ///
    /// Optional so existing packs load without a schema version bump.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub face_size: Option<Vec2>,
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
