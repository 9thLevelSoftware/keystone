# Phase 0 Proof Harness Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the headless Phase 0 proof harness for Asset Mapper: schema, fixture metadata, validation, deterministic connector resolving, LLM bundle export, and CLI checks.

**Architecture:** Create a Rust workspace with a graphics-free `asset-mapper-core` crate and a small `asset-mapper-cli` binary. The core owns schema types, validation, connector transform math, hash helpers, and LLM bundle projection; the CLI proves the core can validate, bundle, and resolve fixture packs without editor code.

**Tech Stack:** Rust 2024 edition, `serde`, `serde_json`, `schemars`, `glam`, `thiserror`, `sha2`, `clap`, `insta`, `proptest`, `assert_cmd`, `predicates`, `tempfile`.

---

## Scope

This plan implements only Phase 0 from `docs/superpowers/specs/2026-06-15-asset-pack-semantic-mapper-design.md`.

Included:

- Rust workspace bootstrap
- canonical schema draft for pack, asset, connector, compatibility, and assembly plans
- hand-authored fixture pack
- validator with machine-readable diagnostics
- deterministic 3D connector resolver
- compact LLM bundle exporter
- CLI commands for `validate`, `bundle`, and `resolve`
- unit, snapshot, property, and CLI integration tests

Excluded:

- editor UI
- Bevy, Tauri, Three.js, or any rendering stack
- asset generation
- scene generation
- chat workflow
- broad import support
- 2D resolver behavior

## File Structure

- `Cargo.toml`  
  Workspace definition and shared dependency versions.

- `.gitignore`  
  Rust build artifacts and local editor noise.

- `crates/asset-mapper-core/Cargo.toml`  
  Core crate manifest.

- `crates/asset-mapper-core/src/lib.rs`  
  Public module surface and re-exports.

- `crates/asset-mapper-core/src/schema.rs`  
  Canonical data structures for packs, assets, connectors, compatibility rules, and assembly plans.

- `crates/asset-mapper-core/src/diagnostics.rs`  
  Validation diagnostic types and validation report helpers.

- `crates/asset-mapper-core/src/validate.rs`  
  Pack validation logic.

- `crates/asset-mapper-core/src/resolver.rs`  
  Deterministic connector math and assembly plan resolution.

- `crates/asset-mapper-core/src/bundle.rs`  
  Compact LLM bundle projection.

- `crates/asset-mapper-core/src/hash.rs`  
  SHA-256 file hash helper for stable asset identity checks.

- `crates/asset-mapper-core/tests/schema_roundtrip.rs`  
  Schema serialization and JSON Schema tests.

- `crates/asset-mapper-core/tests/validation.rs`  
  Validator behavior tests.

- `crates/asset-mapper-core/tests/resolver.rs`  
  Resolver golden and property tests.

- `crates/asset-mapper-core/tests/bundle.rs`  
  LLM bundle snapshot tests.

- `crates/asset-mapper-cli/Cargo.toml`  
  CLI crate manifest.

- `crates/asset-mapper-cli/src/main.rs`  
  `asset-mapper` CLI entrypoint.

- `crates/asset-mapper-cli/tests/cli.rs`  
  CLI integration tests.

- `fixtures/phase0/simple_pack.assetmap.json`  
  Hand-authored modular 3D fixture pack.

- `fixtures/phase0/simple_plan.json`  
  Assembly plan attaching one corridor segment to another.

- `fixtures/phase0/invalid_pack_unknown_class.assetmap.json`  
  Invalid fixture proving validation catches unknown connector classes.

---

### Task 1: Bootstrap Rust Workspace

**Files:**
- Create: `Cargo.toml`
- Create: `.gitignore`
- Create: `crates/asset-mapper-core/Cargo.toml`
- Create: `crates/asset-mapper-core/src/lib.rs`
- Create: `crates/asset-mapper-cli/Cargo.toml`
- Create: `crates/asset-mapper-cli/src/main.rs`

- [ ] **Step 1: Create the workspace manifest**

Write `Cargo.toml`:

```toml
[workspace]
members = [
    "crates/asset-mapper-core",
    "crates/asset-mapper-cli",
]
resolver = "2"

[workspace.package]
edition = "2024"
license = "MIT OR Apache-2.0"
rust-version = "1.85"

[workspace.dependencies]
assert_cmd = "2"
clap = { version = "4", features = ["derive"] }
glam = { version = "0.33", features = ["serde"] }
insta = { version = "1", features = ["json"] }
predicates = "3"
proptest = "1"
schemars = { version = "1", features = ["derive"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
sha2 = "0.11"
tempfile = "3"
thiserror = "2"
```

- [ ] **Step 2: Add Rust build ignores**

Write `.gitignore`:

```gitignore
/target/
**/*.rs.bk
.idea/
.vscode/
```

- [ ] **Step 3: Add the core crate manifest**

Write `crates/asset-mapper-core/Cargo.toml`:

```toml
[package]
name = "asset-mapper-core"
version = "0.1.0"
edition.workspace = true
license.workspace = true
rust-version.workspace = true

[dependencies]
glam.workspace = true
schemars.workspace = true
serde.workspace = true
serde_json.workspace = true
sha2.workspace = true
thiserror.workspace = true

[dev-dependencies]
insta.workspace = true
proptest.workspace = true
tempfile.workspace = true
```

- [ ] **Step 4: Add the initial core module surface**

Write `crates/asset-mapper-core/src/lib.rs`:

```rust
pub mod bundle;
pub mod diagnostics;
pub mod hash;
pub mod resolver;
pub mod schema;
pub mod validate;

pub use bundle::{BundleAsset, BundleConnector, LlmBundle};
pub use diagnostics::{Diagnostic, Severity, ValidationReport};
pub use resolver::{AssetPlacement, ResolveError, ResolvedScene};
pub use schema::*;
pub use validate::validate_pack;
```

- [ ] **Step 5: Add initial modules so the crate parses**

Write these files with the exact contents shown.

`crates/asset-mapper-core/src/bundle.rs`:

```rust
use crate::schema::PackRecord;

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub struct LlmBundle {
    pub pack_id: String,
    pub display_name: String,
    pub assets: Vec<BundleAsset>,
    pub compatibility_rules: Vec<crate::schema::CompatibilityRule>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub struct BundleAsset {
    pub asset_id: String,
    pub display_name: String,
    pub asset_type: crate::schema::AssetType,
    pub dimensions: crate::schema::Vec3,
    pub semantic_tags: Vec<String>,
    pub affordances: Vec<String>,
    pub placement_constraints: Vec<String>,
    pub connectors: Vec<BundleConnector>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub struct BundleConnector {
    pub connector_id: String,
    pub display_name: String,
    pub class: String,
    pub role: crate::schema::ConnectorRole,
}

impl LlmBundle {
    pub fn from_pack(pack: &PackRecord) -> Self {
        Self {
            pack_id: pack.pack_id.clone(),
            display_name: pack.display_name.clone(),
            assets: pack
                .assets
                .iter()
                .map(|asset| BundleAsset {
                    asset_id: asset.asset_id.clone(),
                    display_name: asset.display_name.clone(),
                    asset_type: asset.asset_type.clone(),
                    dimensions: asset.dimensions,
                    semantic_tags: asset.semantic_tags.clone(),
                    affordances: asset.affordances.clone(),
                    placement_constraints: asset.placement_constraints.clone(),
                    connectors: asset
                        .connectors
                        .iter()
                        .map(|connector| BundleConnector {
                            connector_id: connector.connector_id.clone(),
                            display_name: connector.display_name.clone(),
                            class: connector.class.clone(),
                            role: connector.role.clone(),
                        })
                        .collect(),
                })
                .collect(),
            compatibility_rules: pack.compatibility_rules.clone(),
        }
    }
}
```

`crates/asset-mapper-core/src/diagnostics.rs`:

```rust
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    Error,
    Warning,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub struct Diagnostic {
    pub code: String,
    pub severity: Severity,
    pub message: String,
    pub asset_id: Option<String>,
    pub connector_id: Option<String>,
}

impl Diagnostic {
    pub fn error(code: &str, message: impl Into<String>) -> Self {
        Self {
            code: code.to_owned(),
            severity: Severity::Error,
            message: message.into(),
            asset_id: None,
            connector_id: None,
        }
    }

    pub fn warning(code: &str, message: impl Into<String>) -> Self {
        Self {
            code: code.to_owned(),
            severity: Severity::Warning,
            message: message.into(),
            asset_id: None,
            connector_id: None,
        }
    }

    pub fn with_asset(mut self, asset_id: impl Into<String>) -> Self {
        self.asset_id = Some(asset_id.into());
        self
    }

    pub fn with_connector(mut self, connector_id: impl Into<String>) -> Self {
        self.connector_id = Some(connector_id.into());
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub struct ValidationReport {
    pub diagnostics: Vec<Diagnostic>,
}

impl ValidationReport {
    pub fn new(diagnostics: Vec<Diagnostic>) -> Self {
        Self { diagnostics }
    }

    pub fn is_valid(&self) -> bool {
        !self
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.severity == Severity::Error)
    }
}
```

`crates/asset-mapper-core/src/hash.rs`:

```rust
use std::fs::File;
use std::io::{self, Read};
use std::path::Path;

use sha2::{Digest, Sha256};

pub fn sha256_file(path: impl AsRef<Path>) -> io::Result<String> {
    let mut file = File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 8192];

    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }

    Ok(format!("{:x}", hasher.finalize()))
}
```

`crates/asset-mapper-core/src/resolver.rs`:

```rust
use crate::schema::{AssemblyPlan, PackRecord, Transform3d};

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
    #[error("resolver stub reached for plan root asset `{root_asset_id}`")]
    ResolverStub { root_asset_id: String },
}

pub fn resolve_plan(_pack: &PackRecord, plan: &AssemblyPlan) -> Result<ResolvedScene, ResolveError> {
    Err(ResolveError::ResolverStub {
        root_asset_id: plan.root_asset_id.clone(),
    })
}
```

`crates/asset-mapper-core/src/schema.rs`:

```rust
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

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub struct CoordinateConvention {
    pub handedness: Handedness,
    pub up_axis: Axis3,
    pub forward_axis: Axis3,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Handedness {
    Right,
    Left,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Axis3 {
    PosX,
    NegX,
    PosY,
    NegY,
    PosZ,
    NegZ,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Unit {
    Meters,
    Centimeters,
    Pixels,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
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

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum AssetType {
    Model3d,
    Sprite2d,
    Tile2d,
}

#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub struct Bounds3 {
    pub min: Vec3,
    pub max: Vec3,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
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

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
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

#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
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
```

`crates/asset-mapper-core/src/validate.rs`:

```rust
use crate::diagnostics::ValidationReport;
use crate::schema::PackRecord;

pub fn validate_pack(_pack: &PackRecord) -> ValidationReport {
    ValidationReport::new(Vec::new())
}
```

- [ ] **Step 6: Add the CLI crate manifest**

Write `crates/asset-mapper-cli/Cargo.toml`:

```toml
[package]
name = "asset-mapper-cli"
version = "0.1.0"
edition.workspace = true
license.workspace = true
rust-version.workspace = true

[[bin]]
name = "asset-mapper"
path = "src/main.rs"

[dependencies]
asset-mapper-core = { path = "../asset-mapper-core" }
clap.workspace = true
serde_json.workspace = true

[dev-dependencies]
assert_cmd.workspace = true
predicates.workspace = true
tempfile.workspace = true
```

- [ ] **Step 7: Add the initial CLI banner**

Write `crates/asset-mapper-cli/src/main.rs`:

```rust
fn main() {
    println!("asset-mapper phase 0");
}
```

- [ ] **Step 8: Run workspace formatting**

Run:

```powershell
cargo fmt
```

Expected: command exits with code `0`.

- [ ] **Step 9: Run initial tests**

Run:

```powershell
cargo test
```

Expected: command exits with code `0`; there are no meaningful tests yet, but both crates compile.

- [ ] **Step 10: Commit workspace bootstrap**

Run:

```powershell
git add -- Cargo.toml .gitignore crates
git commit -m "chore: bootstrap rust workspace"
```

Expected: commit succeeds.

---

### Task 2: Add Schema Roundtrip And Fixture Files

**Files:**
- Create: `fixtures/phase0/simple_pack.assetmap.json`
- Create: `fixtures/phase0/simple_plan.json`
- Create: `fixtures/phase0/invalid_pack_unknown_class.assetmap.json`
- Create: `crates/asset-mapper-core/tests/schema_roundtrip.rs`
- Modify: `crates/asset-mapper-core/src/schema.rs`

- [ ] **Step 1: Add the valid fixture pack**

Write `fixtures/phase0/simple_pack.assetmap.json`:

```json
{
  "schema_version": 1,
  "pack_id": "phase0_corridor",
  "display_name": "Phase 0 Corridor Fixture",
  "coordinate_convention": {
    "handedness": "right",
    "up_axis": "pos_y",
    "forward_axis": "pos_z"
  },
  "default_units": "meters",
  "connector_classes": [
    {
      "class": "corridor_end",
      "display_name": "Corridor End"
    }
  ],
  "compatibility_rules": [
    {
      "a_class": "corridor_end",
      "b_class": "corridor_end",
      "rotation": {
        "kind": "locked"
      }
    }
  ],
  "assets": [
    {
      "asset_id": "corridor_a",
      "source_path": "corridor_a.glb",
      "content_hash": "sha256:fixture-corridor-a",
      "display_name": "Corridor Segment A",
      "asset_type": "model3d",
      "bounds": {
        "min": [-1.0, 0.0, -1.0],
        "max": [1.0, 2.0, 1.0]
      },
      "dimensions": [2.0, 2.0, 2.0],
      "pivot": "base_center",
      "up_axis": "pos_y",
      "forward_axis": "pos_z",
      "semantic_tags": ["corridor", "wall", "floor"],
      "affordances": ["walkable"],
      "placement_constraints": ["upright_only"],
      "connectors": [
        {
          "connector_id": "front",
          "display_name": "Front End",
          "class": "corridor_end",
          "role": "symmetric",
          "frame": {
            "kind": "frame3d",
            "position": [0.0, 0.0, 1.0],
            "orientation_quat_xyzw": [0.0, 0.0, 0.0, 1.0]
          },
          "mating_axis": "pos_z",
          "up_reference": "pos_y",
          "snap_tolerance": 0.01
        }
      ]
    },
    {
      "asset_id": "corridor_b",
      "source_path": "corridor_b.glb",
      "content_hash": "sha256:fixture-corridor-b",
      "display_name": "Corridor Segment B",
      "asset_type": "model3d",
      "bounds": {
        "min": [-1.0, 0.0, -1.0],
        "max": [1.0, 2.0, 1.0]
      },
      "dimensions": [2.0, 2.0, 2.0],
      "pivot": "base_center",
      "up_axis": "pos_y",
      "forward_axis": "pos_z",
      "semantic_tags": ["corridor", "wall", "floor"],
      "affordances": ["walkable"],
      "placement_constraints": ["upright_only"],
      "connectors": [
        {
          "connector_id": "back",
          "display_name": "Back End",
          "class": "corridor_end",
          "role": "symmetric",
          "frame": {
            "kind": "frame3d",
            "position": [0.0, 0.0, -1.0],
            "orientation_quat_xyzw": [0.0, 0.0, 0.0, 1.0]
          },
          "mating_axis": "pos_z",
          "up_reference": "pos_y",
          "snap_tolerance": 0.01
        }
      ]
    }
  ]
}
```

- [ ] **Step 2: Add the valid assembly plan**

Write `fixtures/phase0/simple_plan.json`:

```json
{
  "root_asset_id": "corridor_a",
  "operations": [
    {
      "placed_asset_id": "corridor_b",
      "placed_connector_id": "back",
      "anchor_asset_id": "corridor_a",
      "anchor_connector_id": "front",
      "rotation_choice_deg": 0.0
    }
  ]
}
```

- [ ] **Step 3: Add the invalid fixture pack**

Write `fixtures/phase0/invalid_pack_unknown_class.assetmap.json`:

```json
{
  "schema_version": 1,
  "pack_id": "phase0_invalid_unknown_class",
  "display_name": "Invalid Unknown Class Fixture",
  "coordinate_convention": {
    "handedness": "right",
    "up_axis": "pos_y",
    "forward_axis": "pos_z"
  },
  "default_units": "meters",
  "connector_classes": [
    {
      "class": "corridor_end",
      "display_name": "Corridor End"
    }
  ],
  "compatibility_rules": [],
  "assets": [
    {
      "asset_id": "bad_corridor",
      "source_path": "bad_corridor.glb",
      "content_hash": "sha256:fixture-bad-corridor",
      "display_name": "Bad Corridor",
      "asset_type": "model3d",
      "bounds": {
        "min": [-1.0, 0.0, -1.0],
        "max": [1.0, 2.0, 1.0]
      },
      "dimensions": [2.0, 2.0, 2.0],
      "pivot": "base_center",
      "up_axis": "pos_y",
      "forward_axis": "pos_z",
      "semantic_tags": ["corridor"],
      "affordances": ["walkable"],
      "placement_constraints": ["upright_only"],
      "connectors": [
        {
          "connector_id": "front",
          "display_name": "Front End",
          "class": "missing_class",
          "role": "symmetric",
          "frame": {
            "kind": "frame3d",
            "position": [0.0, 0.0, 1.0],
            "orientation_quat_xyzw": [0.0, 0.0, 0.0, 1.0]
          },
          "mating_axis": "pos_z",
          "up_reference": "pos_y",
          "snap_tolerance": 0.01
        }
      ]
    }
  ]
}
```

- [ ] **Step 4: Write schema roundtrip tests**

Write `crates/asset-mapper-core/tests/schema_roundtrip.rs`:

```rust
use asset_mapper_core::{AssemblyPlan, PackRecord};

#[test]
fn fixture_pack_round_trips_without_data_loss() {
    let input = include_str!("../../../fixtures/phase0/simple_pack.assetmap.json");
    let pack: PackRecord = serde_json::from_str(input).expect("fixture pack parses");

    assert_eq!(pack.schema_version, asset_mapper_core::CURRENT_SCHEMA_VERSION);
    assert_eq!(pack.pack_id, "phase0_corridor");
    assert_eq!(pack.assets.len(), 2);
    assert_eq!(pack.assets[0].connectors[0].connector_id, "front");

    let serialized = serde_json::to_string_pretty(&pack).expect("pack serializes");
    let reparsed: PackRecord = serde_json::from_str(&serialized).expect("serialized pack reparses");
    assert_eq!(pack, reparsed);
}

#[test]
fn fixture_plan_round_trips_without_data_loss() {
    let input = include_str!("../../../fixtures/phase0/simple_plan.json");
    let plan: AssemblyPlan = serde_json::from_str(input).expect("fixture plan parses");

    assert_eq!(plan.root_asset_id, "corridor_a");
    assert_eq!(plan.operations.len(), 1);
    assert_eq!(plan.operations[0].placed_asset_id, "corridor_b");

    let serialized = serde_json::to_string_pretty(&plan).expect("plan serializes");
    let reparsed: AssemblyPlan = serde_json::from_str(&serialized).expect("serialized plan reparses");
    assert_eq!(plan, reparsed);
}

#[test]
fn pack_record_has_json_schema() {
    let schema = schemars::schema_for!(PackRecord);
    let schema_json = serde_json::to_value(schema).expect("schema serializes to JSON");
    assert_eq!(
        schema_json["title"],
        serde_json::Value::String("PackRecord".to_owned())
    );
}
```

- [ ] **Step 5: Run schema tests**

Run:

```powershell
cargo test -p asset-mapper-core --test schema_roundtrip
```

Expected: all 3 tests pass.

- [ ] **Step 6: Run workspace formatting and tests**

Run:

```powershell
cargo fmt
cargo test
```

Expected: both commands exit with code `0`.

- [ ] **Step 7: Commit schema fixtures**

Run:

```powershell
git add fixtures crates/asset-mapper-core/tests/schema_roundtrip.rs
git commit -m "test: add phase 0 schema fixtures"
```

Expected: commit succeeds.

---

### Task 3: Implement Validator

**Files:**
- Create: `crates/asset-mapper-core/tests/validation.rs`
- Modify: `crates/asset-mapper-core/src/validate.rs`

- [ ] **Step 1: Write failing validator tests**

Write `crates/asset-mapper-core/tests/validation.rs`:

```rust
use asset_mapper_core::{validate_pack, Diagnostic, PackRecord, Severity};

fn load_pack(path: &str) -> PackRecord {
    let input = std::fs::read_to_string(path).expect("fixture can be read");
    serde_json::from_str(&input).expect("fixture parses")
}

#[test]
fn valid_fixture_has_no_validation_errors() {
    let pack = load_pack("fixtures/phase0/simple_pack.assetmap.json");
    let report = validate_pack(&pack);

    assert!(
        report.is_valid(),
        "expected no validation errors, got {:#?}",
        report.diagnostics
    );
}

#[test]
fn unknown_connector_class_is_an_error() {
    let pack = load_pack("fixtures/phase0/invalid_pack_unknown_class.assetmap.json");
    let report = validate_pack(&pack);

    assert!(!report.is_valid());
    assert!(contains_code(&report.diagnostics, "unknown_connector_class"));
}

#[test]
fn connector_class_without_rule_is_a_warning() {
    let pack = load_pack("fixtures/phase0/invalid_pack_unknown_class.assetmap.json");
    let report = validate_pack(&pack);

    assert!(report.diagnostics.iter().any(|diagnostic| {
        diagnostic.code == "connector_class_has_no_rule"
            && diagnostic.severity == Severity::Warning
    }));
}

#[test]
fn duplicate_asset_ids_are_errors() {
    let mut pack = load_pack("fixtures/phase0/simple_pack.assetmap.json");
    pack.assets[1].asset_id = pack.assets[0].asset_id.clone();

    let report = validate_pack(&pack);

    assert!(!report.is_valid());
    assert!(contains_code(&report.diagnostics, "duplicate_asset_id"));
}

#[test]
fn non_normalized_3d_connector_quaternion_is_an_error() {
    let mut pack = load_pack("fixtures/phase0/simple_pack.assetmap.json");
    if let asset_mapper_core::ConnectorFrame::Frame3d {
        orientation_quat_xyzw,
        ..
    } = &mut pack.assets[0].connectors[0].frame
    {
        *orientation_quat_xyzw = [0.0, 0.0, 0.0, 2.0];
    }

    let report = validate_pack(&pack);

    assert!(!report.is_valid());
    assert!(contains_code(&report.diagnostics, "connector_quaternion_not_normalized"));
}

fn contains_code(diagnostics: &[Diagnostic], code: &str) -> bool {
    diagnostics
        .iter()
        .any(|diagnostic| diagnostic.code == code)
}
```

- [ ] **Step 2: Run validator tests to confirm failure**

Run:

```powershell
cargo test -p asset-mapper-core --test validation
```

Expected: tests fail because `validate_pack` currently returns an empty report.

- [ ] **Step 3: Implement validation logic**

Replace `crates/asset-mapper-core/src/validate.rs` with:

```rust
use std::collections::HashSet;

use crate::diagnostics::{Diagnostic, ValidationReport};
use crate::schema::{ConnectorFrame, PackRecord, CURRENT_SCHEMA_VERSION};

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
                format!("compatibility rule references unknown a_class `{}`", rule.a_class),
            ));
        }
        if !class_names.contains(rule.b_class.as_str()) {
            diagnostics.push(Diagnostic::error(
                "unknown_rule_class",
                format!("compatibility rule references unknown b_class `{}`", rule.b_class),
            ));
        }
        classes_with_rules.insert(rule.a_class.as_str());
        classes_with_rules.insert(rule.b_class.as_str());
    }

    let mut asset_ids = HashSet::new();
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

        if asset.content_hash.trim().is_empty() {
            diagnostics.push(
                Diagnostic::error("missing_content_hash", "asset content_hash is empty")
                    .with_asset(asset.asset_id.clone()),
            );
        }

        if !bounds_are_ordered(asset.bounds.min, asset.bounds.max) {
            diagnostics.push(
                Diagnostic::error(
                    "invalid_bounds",
                    "asset bounds min must be less than or equal to max on every axis",
                )
                .with_asset(asset.asset_id.clone()),
            );
        }

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
            } else if !classes_with_rules.contains(connector.class.as_str()) {
                diagnostics.push(
                    Diagnostic::warning(
                        "connector_class_has_no_rule",
                        format!(
                            "connector class `{}` does not participate in any compatibility rule",
                            connector.class
                        ),
                    )
                    .with_asset(asset.asset_id.clone())
                    .with_connector(connector.connector_id.clone()),
                );
            }

            if connector.snap_tolerance < 0.0 {
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
```

- [ ] **Step 4: Run validator tests to verify pass**

Run:

```powershell
cargo test -p asset-mapper-core --test validation
```

Expected: all validation tests pass.

- [ ] **Step 5: Run full core tests**

Run:

```powershell
cargo test -p asset-mapper-core
```

Expected: all core tests pass.

- [ ] **Step 6: Commit validator**

Run:

```powershell
git add crates/asset-mapper-core/src/validate.rs crates/asset-mapper-core/tests/validation.rs
git commit -m "feat: validate asset pack metadata"
```

Expected: commit succeeds.

---

### Task 4: Implement Deterministic 3D Resolver

**Files:**
- Create: `crates/asset-mapper-core/tests/resolver.rs`
- Modify: `crates/asset-mapper-core/src/resolver.rs`

- [ ] **Step 1: Write failing resolver tests**

Write `crates/asset-mapper-core/tests/resolver.rs`:

```rust
use asset_mapper_core::{resolve_plan, AllowedRotation, AssemblyPlan, PackRecord, ResolveError};
use proptest::prelude::*;

fn load_pack() -> PackRecord {
    let input = std::fs::read_to_string("fixtures/phase0/simple_pack.assetmap.json")
        .expect("fixture pack can be read");
    serde_json::from_str(&input).expect("fixture pack parses")
}

fn load_plan() -> AssemblyPlan {
    let input = std::fs::read_to_string("fixtures/phase0/simple_plan.json")
        .expect("fixture plan can be read");
    serde_json::from_str(&input).expect("fixture plan parses")
}

#[test]
fn resolves_simple_corridor_attachment() {
    let pack = load_pack();
    let plan = load_plan();

    let scene = resolve_plan(&pack, &plan).expect("plan resolves");

    assert_eq!(scene.placements.len(), 2);
    assert_eq!(scene.placements[0].asset_id, "corridor_a");
    assert_eq!(scene.placements[0].transform.translation, [0.0, 0.0, 0.0]);
    assert_eq!(scene.placements[1].asset_id, "corridor_b");
    assert_close(scene.placements[1].transform.translation[0], 0.0);
    assert_close(scene.placements[1].transform.translation[1], 0.0);
    assert_close(scene.placements[1].transform.translation[2], 0.0);
}

#[test]
fn rejects_unplaced_anchor_asset() {
    let pack = load_pack();
    let mut plan = load_plan();
    plan.operations[0].anchor_asset_id = "corridor_b".to_owned();

    let error = resolve_plan(&pack, &plan).expect_err("plan should fail");

    assert!(matches!(
        error,
        ResolveError::AnchorAssetNotPlaced { anchor_asset_id } if anchor_asset_id == "missing"
    ));
}

#[test]
fn locked_rotation_rejects_non_zero_choice() {
    let pack = load_pack();
    let mut plan = load_plan();
    plan.operations[0].rotation_choice_deg = Some(90.0);

    let error = resolve_plan(&pack, &plan).expect_err("plan should fail");

    assert!(matches!(
        error,
        ResolveError::RotationChoiceNotAllowed { choice } if (choice - 90.0).abs() < 0.001
    ));
}

#[test]
fn rejects_incompatible_connector_classes() {
    let mut pack = load_pack();
    pack.compatibility_rules.clear();
    let plan = load_plan();

    let error = resolve_plan(&pack, &plan).expect_err("plan should fail");

    assert!(matches!(
        error,
        ResolveError::IncompatibleConnectorClasses { placed_class, anchor_class }
        if placed_class == "corridor_end" && anchor_class == "corridor_end"
    ));
}

proptest! {
    #[test]
    fn resolved_quaternion_is_normalized(rotation_choice in -360.0_f32..360.0_f32) {
        let mut pack = load_pack();
        pack.compatibility_rules[0].rotation = AllowedRotation::Free;
        let mut plan = load_plan();
        plan.operations[0].rotation_choice_deg = Some(rotation_choice);

        let scene = resolve_plan(&pack, &plan).expect("free rotation fixture resolves");
        let quat = scene.placements[1].transform.rotation_quat_xyzw;
        let length_squared = quat.iter().map(|component| component * component).sum::<f32>();
        prop_assert!((length_squared - 1.0).abs() < 0.001);
    }
}

fn assert_close(actual: f32, expected: f32) {
    assert!(
        (actual - expected).abs() < 0.001,
        "expected {actual} to be close to {expected}"
    );
}
```

- [ ] **Step 2: Run resolver tests to confirm failure**

Run:

```powershell
cargo test -p asset-mapper-core --test resolver
```

Expected: tests fail because `resolve_plan` still returns `ResolveError::ResolverStub`.

- [ ] **Step 3: Replace resolver implementation**

Replace `crates/asset-mapper-core/src/resolver.rs` with:

```rust
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
    let root_asset = find_asset(pack, &plan.root_asset_id).ok_or_else(|| {
        ResolveError::UnknownRootAsset {
            root_asset_id: plan.root_asset_id.clone(),
        }
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
            rotation: Quat::from_rotation_z(operation.rotation_choice_deg.unwrap_or(0.0).to_radians()),
        };

        let desired_placed_connector_world = anchor_connector_world.then(flip).then(roll);
        let placed_asset_world = desired_placed_connector_world.then(placed_connector_local.inverse());

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
        AllowedRotation::Free => Ok(()),
        AllowedRotation::StepsDeg { values } => {
            if values.iter().any(|value| (*value - choice).abs() < 0.001) {
                Ok(())
            } else {
                Err(ResolveError::RotationChoiceNotAllowed { choice })
            }
        }
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
```

- [ ] **Step 4: Update public re-export if needed**

Inspect `crates/asset-mapper-core/src/lib.rs`. It should still include:

```rust
pub use resolver::{AssetPlacement, ResolveError, ResolvedScene};
```

No change is needed if that line already matches.

- [ ] **Step 5: Run resolver tests**

Run:

```powershell
cargo test -p asset-mapper-core --test resolver
```

Expected: all resolver tests pass.

- [ ] **Step 6: Run all core tests**

Run:

```powershell
cargo test -p asset-mapper-core
```

Expected: all core tests pass.

- [ ] **Step 7: Commit resolver**

Run:

```powershell
git add crates/asset-mapper-core/src/resolver.rs crates/asset-mapper-core/tests/resolver.rs
git commit -m "feat: resolve 3d connector assemblies"
```

Expected: commit succeeds.

---

### Task 5: Add LLM Bundle Snapshot Coverage

**Files:**
- Create: `crates/asset-mapper-core/tests/bundle.rs`
- Modify: `crates/asset-mapper-core/src/bundle.rs`

- [ ] **Step 1: Write bundle snapshot test**

Write `crates/asset-mapper-core/tests/bundle.rs`:

```rust
use asset_mapper_core::{LlmBundle, PackRecord};

#[test]
fn bundle_omits_raw_connector_transforms() {
    let input = std::fs::read_to_string("fixtures/phase0/simple_pack.assetmap.json")
        .expect("fixture pack can be read");
    let pack: PackRecord = serde_json::from_str(&input).expect("fixture pack parses");

    let bundle = LlmBundle::from_pack(&pack);
    let bundle_json = serde_json::to_value(&bundle).expect("bundle serializes");

    assert!(bundle_json.get("assets").is_some());
    assert!(
        !bundle_json.to_string().contains("orientation_quat_xyzw"),
        "LLM bundle must not expose raw quaternion data"
    );
    assert!(
        !bundle_json.to_string().contains("frame3d"),
        "LLM bundle must not expose raw connector frame data"
    );

    insta::assert_json_snapshot!(bundle_json);
}
```

- [ ] **Step 2: Run bundle test and accept first snapshot**

Run:

```powershell
$env:INSTA_UPDATE = 'always'
cargo test -p asset-mapper-core --test bundle
Remove-Item Env:INSTA_UPDATE
```

Expected: the test writes the reviewed snapshot and exits with code `0`.

- [ ] **Step 3: Run bundle test again**

Run:

```powershell
cargo test -p asset-mapper-core --test bundle
```

Expected: test passes with the committed snapshot.

- [ ] **Step 4: Run all core tests**

Run:

```powershell
cargo test -p asset-mapper-core
```

Expected: all core tests pass.

- [ ] **Step 5: Commit bundle snapshot**

Run:

```powershell
git add crates/asset-mapper-core/src/bundle.rs crates/asset-mapper-core/tests/bundle.rs crates/asset-mapper-core/tests/snapshots
git commit -m "feat: export llm bundle"
```

Expected: commit succeeds.

---

### Task 6: Implement CLI Commands

**Files:**
- Modify: `crates/asset-mapper-cli/src/main.rs`
- Create: `crates/asset-mapper-cli/tests/cli.rs`

- [ ] **Step 1: Write failing CLI tests**

Write `crates/asset-mapper-cli/tests/cli.rs`:

```rust
use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn validate_accepts_valid_fixture() {
    let mut command = Command::cargo_bin("asset-mapper").expect("binary exists");

    command
        .args(["validate", "fixtures/phase0/simple_pack.assetmap.json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"diagnostics\": []"));
}

#[test]
fn validate_rejects_invalid_fixture() {
    let mut command = Command::cargo_bin("asset-mapper").expect("binary exists");

    command
        .args([
            "validate",
            "fixtures/phase0/invalid_pack_unknown_class.assetmap.json",
        ])
        .assert()
        .failure()
        .stdout(predicate::str::contains("unknown_connector_class"));
}

#[test]
fn bundle_emits_llm_context() {
    let mut command = Command::cargo_bin("asset-mapper").expect("binary exists");

    command
        .args(["bundle", "fixtures/phase0/simple_pack.assetmap.json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"pack_id\": \"phase0_corridor\""))
        .stdout(predicate::str::contains("\"connector_id\": \"front\""))
        .stdout(predicate::str::contains("orientation_quat_xyzw").not());
}

#[test]
fn resolve_emits_resolved_scene() {
    let mut command = Command::cargo_bin("asset-mapper").expect("binary exists");

    command
        .args([
            "resolve",
            "fixtures/phase0/simple_pack.assetmap.json",
            "fixtures/phase0/simple_plan.json",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"asset_id\": \"corridor_a\""))
        .stdout(predicate::str::contains("\"asset_id\": \"corridor_b\""));
}
```

- [ ] **Step 2: Run CLI tests to confirm failure**

Run:

```powershell
cargo test -p asset-mapper-cli --test cli
```

Expected: tests fail because the CLI only prints the initial banner.

- [ ] **Step 3: Implement CLI commands**

Replace `crates/asset-mapper-cli/src/main.rs` with:

```rust
use std::fs;
use std::path::PathBuf;
use std::process::ExitCode;

use asset_mapper_core::{resolve_plan, validate_pack, AssemblyPlan, LlmBundle, PackRecord};
use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(name = "asset-mapper")]
#[command(about = "Headless Asset Mapper Phase 0 proof harness")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    Validate {
        pack: PathBuf,
    },
    Bundle {
        pack: PathBuf,
    },
    Resolve {
        pack: PathBuf,
        plan: PathBuf,
    },
}

fn main() -> ExitCode {
    match run() {
        Ok(code) => code,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::from(1)
        }
    }
}

fn run() -> Result<ExitCode, Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Validate { pack } => {
            let pack = read_pack(pack)?;
            let report = validate_pack(&pack);
            println!("{}", serde_json::to_string_pretty(&report)?);
            if report.is_valid() {
                Ok(ExitCode::SUCCESS)
            } else {
                Ok(ExitCode::from(1))
            }
        }
        Commands::Bundle { pack } => {
            let pack = read_pack(pack)?;
            let bundle = LlmBundle::from_pack(&pack);
            println!("{}", serde_json::to_string_pretty(&bundle)?);
            Ok(ExitCode::SUCCESS)
        }
        Commands::Resolve { pack, plan } => {
            let pack = read_pack(pack)?;
            let plan = read_plan(plan)?;
            let scene = resolve_plan(&pack, &plan)?;
            println!("{}", serde_json::to_string_pretty(&scene)?);
            Ok(ExitCode::SUCCESS)
        }
    }
}

fn read_pack(path: PathBuf) -> Result<PackRecord, Box<dyn std::error::Error>> {
    let input = fs::read_to_string(path)?;
    Ok(serde_json::from_str(&input)?)
}

fn read_plan(path: PathBuf) -> Result<AssemblyPlan, Box<dyn std::error::Error>> {
    let input = fs::read_to_string(path)?;
    Ok(serde_json::from_str(&input)?)
}
```

- [ ] **Step 4: Run CLI tests**

Run:

```powershell
cargo test -p asset-mapper-cli --test cli
```

Expected: all CLI tests pass.

- [ ] **Step 5: Run full workspace tests**

Run:

```powershell
cargo test
```

Expected: all workspace tests pass.

- [ ] **Step 6: Commit CLI**

Run:

```powershell
git add crates/asset-mapper-cli/src/main.rs crates/asset-mapper-cli/tests/cli.rs
git commit -m "feat: add phase 0 cli"
```

Expected: commit succeeds.

---

### Task 7: Add Hash Helper Coverage And Final Quality Gates

**Files:**
- Create: `crates/asset-mapper-core/tests/hash.rs`
- Modify: no production file if `hash.rs` from Task 1 already matches

- [ ] **Step 1: Write hash helper test**

Write `crates/asset-mapper-core/tests/hash.rs`:

```rust
use std::io::Write;

use asset_mapper_core::hash::sha256_file;

#[test]
fn hashes_file_content_with_sha256() {
    let mut file = tempfile::NamedTempFile::new().expect("temp file is created");
    file.write_all(b"asset mapper\n")
        .expect("temp file can be written");

    let hash = sha256_file(file.path()).expect("hash succeeds");

    assert_eq!(
        hash,
        "fdf54baece5b0ff246dc1d2d5b85efc0c51dde8e41013c939648b8a2ac3426a2"
    );
}
```

- [ ] **Step 2: Run hash test**

Run:

```powershell
cargo test -p asset-mapper-core --test hash
```

Expected: hash test passes with the SHA-256 digest for `asset mapper` followed by a newline.

- [ ] **Step 3: Run formatter check**

Run:

```powershell
cargo fmt -- --check
```

Expected: command exits with code `0`.

- [ ] **Step 4: Run clippy**

Run:

```powershell
cargo clippy --all-targets --all-features -- -D warnings
```

Expected: command exits with code `0`.

- [ ] **Step 5: Run full test suite**

Run:

```powershell
cargo test
```

Expected: all tests pass.

- [ ] **Step 6: Run manual CLI smoke commands**

Run:

```powershell
cargo run -p asset-mapper-cli -- validate fixtures/phase0/simple_pack.assetmap.json
cargo run -p asset-mapper-cli -- bundle fixtures/phase0/simple_pack.assetmap.json
cargo run -p asset-mapper-cli -- resolve fixtures/phase0/simple_pack.assetmap.json fixtures/phase0/simple_plan.json
```

Expected:

- `validate` exits with code `0` and prints `"diagnostics": []`
- `bundle` exits with code `0` and prints `"pack_id": "phase0_corridor"`
- `resolve` exits with code `0` and prints placements for `corridor_a` and `corridor_b`

- [ ] **Step 7: Commit hash coverage and final verification**

Run:

```powershell
git add crates/asset-mapper-core/tests/hash.rs
git commit -m "test: cover asset content hashing"
```

Expected: commit succeeds.

---

## Final Verification Checklist

Run these commands from `C:\Users\dasbl\Documents\Asset Mapper`:

```powershell
cargo fmt -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test
cargo run -p asset-mapper-cli -- validate fixtures/phase0/simple_pack.assetmap.json
cargo run -p asset-mapper-cli -- bundle fixtures/phase0/simple_pack.assetmap.json
cargo run -p asset-mapper-cli -- resolve fixtures/phase0/simple_pack.assetmap.json fixtures/phase0/simple_plan.json
git status --short
```

Expected final state:

- formatting check passes
- clippy passes with warnings denied
- all tests pass
- valid fixture reports no diagnostics
- LLM bundle omits raw connector frames and quaternions
- resolver returns placements for both fixture assets
- `git status --short` is empty after the final commit

## Self-Review Notes

- Spec coverage: The plan covers the Phase 0 requirements from the approved design: schema draft, hand-authored fixture, validator, deterministic resolver, compact LLM bundle, and tests for valid and invalid connector operations.
- Scope control: The plan does not include editor, rendering, asset generation, world generation, or chat behavior.
- Type consistency: `PackRecord`, `AssemblyPlan`, `LlmBundle`, `ValidationReport`, `resolve_plan`, and `validate_pack` are introduced before later tasks use them.
- Dependency basis: Dependency choices were checked against crates.io or official crate docs on 2026-06-15 before writing the plan.
