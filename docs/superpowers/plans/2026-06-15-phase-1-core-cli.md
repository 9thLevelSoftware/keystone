# Phase 1 Core CLI Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the Phase 1 headless CLI workflow so pack metadata can be initialized, indexed, validated, bundled, and resolved from a pack folder without a GUI.

**Architecture:** Keep `asset-mapper-core` responsible for schema, validation, resolver, and LLM bundle projection. Add `asset-mapper-io` for sidecar path resolution, pack-folder scanning, hashing, sidecar read/write, and source-file validation. Keep `asset-mapper-cli` thin: parse commands, call IO/core functions, print JSON reports to stdout, and print operational failures to stderr.

**Tech Stack:** Rust 2024 edition, existing workspace crates, `serde`, `serde_json`, `schemars`, `thiserror`, `sha2`, `clap`, `assert_cmd`, `predicates`, `tempfile`, `proptest`, `insta`.

---

## File Structure

- `Cargo.toml`  
  Add `crates/asset-mapper-io` to the workspace members.

- `crates/asset-mapper-core/src/schema.rs`  
  Add review flags used by the indexer to mark placeholder metadata created from file discovery.

- `crates/asset-mapper-core/src/diagnostics.rs`  
  Add a small helper for merging validation reports.

- `crates/asset-mapper-core/src/validate.rs`  
  Extend schema validation for duplicate source paths, non-finite numeric metadata, and placeholder review flags.

- `crates/asset-mapper-core/tests/resolver.rs`  
  Strengthen the Phase 0 resolver proof so the corridor fixture must resolve to non-overlapping placement.

- `crates/asset-mapper-core/tests/validation.rs`  
  Add validation coverage for duplicate source paths, non-finite metadata, and placeholder review warnings.

- `fixtures/phase0/simple_pack.assetmap.json`  
  Correct the placed corridor connector mating axis so the existing fixture proves spatial attachment.

- `crates/asset-mapper-io/Cargo.toml`  
  New IO crate manifest.

- `crates/asset-mapper-io/src/lib.rs`  
  Public IO crate surface and re-exports.

- `crates/asset-mapper-io/src/error.rs`  
  IO-specific errors for sidecar, indexing, and validation operations.

- `crates/asset-mapper-io/src/sidecar.rs`  
  Canonical sidecar path resolution plus sidecar read/write helpers.

- `crates/asset-mapper-io/src/index.rs`  
  Supported asset scanning, placeholder asset creation, `init`, and `index` reconciliation.

- `crates/asset-mapper-io/src/validation.rs`  
  Source-file validation that needs filesystem access.

- `crates/asset-mapper-io/tests/sidecar.rs`  
  IO tests for canonical sidecar path resolution and direct sidecar support.

- `crates/asset-mapper-io/tests/index.rs`  
  IO tests for scanning, initialization, and re-index reconciliation.

- `crates/asset-mapper-io/tests/source_validation.rs`  
  IO tests for missing, drifted, and untracked source-file diagnostics.

- `crates/asset-mapper-cli/Cargo.toml`  
  Add `asset-mapper-io` as a dependency.

- `crates/asset-mapper-cli/src/main.rs`  
  Add `init` and `index`; update `validate`, `bundle`, and `resolve` to accept direct sidecar files or pack folders.

- `crates/asset-mapper-cli/tests/cli.rs`  
  Add CLI integration tests for folder workflows while preserving Phase 0 command coverage.

---

### Task 1: Harden The Corridor Resolver Fixture

**Files:**
- Modify: `fixtures/phase0/simple_pack.assetmap.json`
- Modify: `crates/asset-mapper-core/tests/resolver.rs`

- [ ] **Step 1: Strengthen the resolver test expectation**

In `crates/asset-mapper-core/tests/resolver.rs`, update `resolves_simple_corridor_attachment` so the second asset must move to `z = 2` and must keep identity rotation:

```rust
#[test]
fn resolves_simple_corridor_attachment() {
    let pack = load_pack();
    let plan = load_plan();

    let scene = resolve_plan(&pack, &plan).expect("plan resolves");

    assert_eq!(scene.placements.len(), 2);
    assert_eq!(scene.placements[0].asset_id, "corridor_a");
    assert_eq!(scene.placements[0].transform.translation, [0.0, 0.0, 0.0]);
    assert_eq!(
        scene.placements[0].transform.rotation_quat_xyzw,
        [0.0, 0.0, 0.0, 1.0]
    );
    assert_eq!(scene.placements[1].asset_id, "corridor_b");
    assert_close(scene.placements[1].transform.translation[0], 0.0);
    assert_close(scene.placements[1].transform.translation[1], 0.0);
    assert_close(scene.placements[1].transform.translation[2], 2.0);
    assert_vec3_close(
        Vec3::from_array(scene.placements[1].transform.translation),
        Vec3::new(0.0, 0.0, 2.0),
    );
    assert_eq!(
        scene.placements[1].transform.rotation_quat_xyzw,
        [0.0, 0.0, 0.0, 1.0]
    );
}
```

- [ ] **Step 2: Run the resolver test to verify it fails**

Run:

```powershell
cargo test -p asset-mapper-core --test resolver resolves_simple_corridor_attachment
```

Expected: the test fails because `corridor_b` still resolves at `translation[2] == 0.0`.

- [ ] **Step 3: Correct the placed connector mating axis**

In `fixtures/phase0/simple_pack.assetmap.json`, change only `corridor_b` connector `back` from:

```json
"mating_axis": "pos_z",
```

to:

```json
"mating_axis": "neg_z",
```

- [ ] **Step 4: Run focused resolver verification**

Run:

```powershell
cargo test -p asset-mapper-core --test resolver resolves_simple_corridor_attachment
```

Expected: the test passes.

- [ ] **Step 5: Run all existing tests**

Run:

```powershell
cargo test
```

Expected: all tests pass.

- [ ] **Step 6: Commit the fixture hardening**

Run:

```powershell
git add -- fixtures/phase0/simple_pack.assetmap.json crates/asset-mapper-core/tests/resolver.rs
git commit -m "test: prove non-overlapping connector resolution"
```

Expected: commit succeeds.

---

### Task 2: Add The IO Crate And Sidecar Helpers

**Files:**
- Modify: `Cargo.toml`
- Create: `crates/asset-mapper-io/Cargo.toml`
- Create: `crates/asset-mapper-io/src/lib.rs`
- Create: `crates/asset-mapper-io/src/error.rs`
- Create: `crates/asset-mapper-io/src/sidecar.rs`
- Create: `crates/asset-mapper-io/tests/sidecar.rs`

- [ ] **Step 1: Add failing sidecar tests**

Create `crates/asset-mapper-io/tests/sidecar.rs`:

```rust
use asset_mapper_core::{
    Axis3, CoordinateConvention, Handedness, PackRecord, Unit, CURRENT_SCHEMA_VERSION,
};
use asset_mapper_io::{
    canonical_sidecar_path, read_pack_from_input, resolve_pack_input_path, write_pack_sidecar,
    PackInputKind, SIDECAR_FILE,
};

fn minimal_pack() -> PackRecord {
    PackRecord {
        schema_version: CURRENT_SCHEMA_VERSION,
        pack_id: "test_pack".to_owned(),
        display_name: "Test Pack".to_owned(),
        coordinate_convention: CoordinateConvention {
            handedness: Handedness::Right,
            up_axis: Axis3::PosY,
            forward_axis: Axis3::PosZ,
        },
        default_units: Unit::Meters,
        connector_classes: Vec::new(),
        compatibility_rules: Vec::new(),
        assets: Vec::new(),
    }
}

#[test]
fn canonical_sidecar_path_lives_under_metadata_directory() {
    let temp = tempfile::tempdir().expect("temp dir is created");
    let sidecar = canonical_sidecar_path(temp.path());

    assert_eq!(sidecar.file_name().and_then(|name| name.to_str()), Some(SIDECAR_FILE));
    assert_eq!(
        sidecar
            .parent()
            .and_then(|parent| parent.file_name())
            .and_then(|name| name.to_str()),
        Some(".asset-mapper")
    );
}

#[test]
fn resolves_pack_folder_to_canonical_sidecar() {
    let temp = tempfile::tempdir().expect("temp dir is created");
    let resolved = resolve_pack_input_path(temp.path()).expect("folder input resolves");

    assert_eq!(resolved.kind, PackInputKind::PackFolder);
    assert_eq!(resolved.sidecar_path, canonical_sidecar_path(temp.path()));
    assert_eq!(resolved.pack_root.as_deref(), Some(temp.path()));
}

#[test]
fn resolves_direct_sidecar_file() {
    let temp = tempfile::tempdir().expect("temp dir is created");
    let sidecar = temp.path().join("custom.assetmap.json");
    std::fs::write(&sidecar, "{}").expect("sidecar file can be written");

    let resolved = resolve_pack_input_path(&sidecar).expect("file input resolves");

    assert_eq!(resolved.kind, PackInputKind::DirectSidecar);
    assert_eq!(resolved.sidecar_path, sidecar);
    assert_eq!(resolved.pack_root, None);
}

#[test]
fn reads_and_writes_pack_sidecar() {
    let temp = tempfile::tempdir().expect("temp dir is created");
    let pack = minimal_pack();

    let sidecar = write_pack_sidecar(temp.path(), &pack).expect("sidecar writes");
    assert_eq!(sidecar, canonical_sidecar_path(temp.path()));

    let loaded = read_pack_from_input(temp.path()).expect("pack reloads from folder");
    assert_eq!(loaded.pack.pack_id, "test_pack");
    assert_eq!(loaded.resolved.kind, PackInputKind::PackFolder);
}
```

- [ ] **Step 2: Run the sidecar tests to verify they fail**

Run:

```powershell
cargo test -p asset-mapper-io --test sidecar
```

Expected: Cargo fails because `asset-mapper-io` does not exist yet.

- [ ] **Step 3: Add the IO crate to the workspace**

Modify the workspace members in `Cargo.toml`:

```toml
[workspace]
members = [
    "crates/asset-mapper-core",
    "crates/asset-mapper-io",
    "crates/asset-mapper-cli",
]
resolver = "2"
```

- [ ] **Step 4: Create the IO crate manifest**

Create `crates/asset-mapper-io/Cargo.toml`:

```toml
[package]
name = "asset-mapper-io"
version = "0.1.0"
edition.workspace = true
license.workspace = true
rust-version.workspace = true

[dependencies]
asset-mapper-core = { path = "../asset-mapper-core" }
serde.workspace = true
serde_json.workspace = true
thiserror.workspace = true

[dev-dependencies]
tempfile.workspace = true
```

- [ ] **Step 5: Add the IO public module surface**

Create `crates/asset-mapper-io/src/lib.rs`:

```rust
pub mod error;
pub mod sidecar;

pub use error::IoError;
pub use sidecar::{
    canonical_sidecar_path, read_pack_from_input, resolve_pack_input_path, write_pack_sidecar,
    LoadedPack, PackInputKind, ResolvedPackInput, METADATA_DIR, SIDECAR_FILE,
};
```

- [ ] **Step 6: Add IO error types**

Create `crates/asset-mapper-io/src/error.rs`:

```rust
use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum IoError {
    #[error("pack sidecar already exists at `{path}`")]
    SidecarAlreadyExists { path: PathBuf },

    #[error("pack sidecar does not exist at `{path}`")]
    MissingSidecar { path: PathBuf },

    #[error("path `{path}` is neither a file nor a directory")]
    InvalidPackInput { path: PathBuf },

    #[error("failed to read `{path}`: {source}")]
    ReadFile {
        path: PathBuf,
        source: std::io::Error,
    },

    #[error("failed to write `{path}`: {source}")]
    WriteFile {
        path: PathBuf,
        source: std::io::Error,
    },

    #[error("failed to create directory `{path}`: {source}")]
    CreateDir {
        path: PathBuf,
        source: std::io::Error,
    },

    #[error("failed to parse JSON `{path}`: {source}")]
    ParseJson {
        path: PathBuf,
        source: serde_json::Error,
    },

    #[error("failed to serialize pack JSON: {0}")]
    SerializeJson(serde_json::Error),
}
```

- [ ] **Step 7: Add sidecar path resolution and read/write helpers**

Create `crates/asset-mapper-io/src/sidecar.rs`:

```rust
use std::path::{Path, PathBuf};

use asset_mapper_core::PackRecord;

use crate::error::IoError;

pub const METADATA_DIR: &str = ".asset-mapper";
pub const SIDECAR_FILE: &str = "pack.assetmap.json";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PackInputKind {
    PackFolder,
    DirectSidecar,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedPackInput {
    pub kind: PackInputKind,
    pub sidecar_path: PathBuf,
    pub pack_root: Option<PathBuf>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LoadedPack {
    pub pack: PackRecord,
    pub resolved: ResolvedPackInput,
}

pub fn canonical_sidecar_path(pack_root: impl AsRef<Path>) -> PathBuf {
    pack_root.as_ref().join(METADATA_DIR).join(SIDECAR_FILE)
}

pub fn resolve_pack_input_path(input: impl AsRef<Path>) -> Result<ResolvedPackInput, IoError> {
    let input = input.as_ref();
    if input.is_dir() {
        return Ok(ResolvedPackInput {
            kind: PackInputKind::PackFolder,
            sidecar_path: canonical_sidecar_path(input),
            pack_root: Some(input.to_path_buf()),
        });
    }

    if input.is_file() {
        return Ok(ResolvedPackInput {
            kind: PackInputKind::DirectSidecar,
            sidecar_path: input.to_path_buf(),
            pack_root: infer_pack_root_from_sidecar(input),
        });
    }

    Err(IoError::InvalidPackInput {
        path: input.to_path_buf(),
    })
}

pub fn read_pack_from_input(input: impl AsRef<Path>) -> Result<LoadedPack, IoError> {
    let resolved = resolve_pack_input_path(input)?;
    if !resolved.sidecar_path.is_file() {
        return Err(IoError::MissingSidecar {
            path: resolved.sidecar_path,
        });
    }

    let input = std::fs::read_to_string(&resolved.sidecar_path).map_err(|source| {
        IoError::ReadFile {
            path: resolved.sidecar_path.clone(),
            source,
        }
    })?;
    let pack = serde_json::from_str(&input).map_err(|source| IoError::ParseJson {
        path: resolved.sidecar_path.clone(),
        source,
    })?;

    Ok(LoadedPack { pack, resolved })
}

pub fn write_pack_sidecar(
    pack_root: impl AsRef<Path>,
    pack: &PackRecord,
) -> Result<PathBuf, IoError> {
    let sidecar_path = canonical_sidecar_path(pack_root);
    let metadata_dir = sidecar_path
        .parent()
        .expect("canonical sidecar path always has a parent");

    std::fs::create_dir_all(metadata_dir).map_err(|source| IoError::CreateDir {
        path: metadata_dir.to_path_buf(),
        source,
    })?;

    let output = serde_json::to_string_pretty(pack).map_err(IoError::SerializeJson)?;
    std::fs::write(&sidecar_path, format!("{output}\n")).map_err(|source| IoError::WriteFile {
        path: sidecar_path.clone(),
        source,
    })?;

    Ok(sidecar_path)
}

fn infer_pack_root_from_sidecar(sidecar_path: &Path) -> Option<PathBuf> {
    let metadata_dir = sidecar_path.parent()?;
    if metadata_dir.file_name()?.to_str()? != METADATA_DIR {
        return None;
    }

    metadata_dir.parent().map(Path::to_path_buf)
}
```

- [ ] **Step 8: Run sidecar tests**

Run:

```powershell
cargo test -p asset-mapper-io --test sidecar
```

Expected: all sidecar tests pass.

- [ ] **Step 9: Run workspace tests**

Run:

```powershell
cargo test
```

Expected: all tests pass.

- [ ] **Step 10: Commit the IO sidecar foundation**

Run:

```powershell
git add -- Cargo.toml crates/asset-mapper-io
git commit -m "feat: add pack sidecar io"
```

Expected: commit succeeds.

---

### Task 3: Add Folder Scanning And Index Reconciliation

**Files:**
- Modify: `crates/asset-mapper-core/src/schema.rs`
- Modify: `crates/asset-mapper-io/Cargo.toml`
- Modify: `crates/asset-mapper-io/src/error.rs`
- Modify: `crates/asset-mapper-io/src/lib.rs`
- Create: `crates/asset-mapper-io/src/index.rs`
- Create: `crates/asset-mapper-io/tests/index.rs`

- [ ] **Step 1: Add failing index tests**

Create `crates/asset-mapper-io/tests/index.rs`:

```rust
use asset_mapper_core::{AssetType, ReviewFlag};
use asset_mapper_io::{
    canonical_sidecar_path, index_pack_folder, init_pack_folder, read_pack_from_input, scan_assets,
};

#[test]
fn scan_assets_ignores_metadata_directory_and_normalizes_paths() {
    let temp = tempfile::tempdir().expect("temp dir is created");
    std::fs::create_dir_all(temp.path().join("models")).expect("models dir is created");
    std::fs::create_dir_all(temp.path().join(".asset-mapper")).expect("metadata dir is created");
    std::fs::write(temp.path().join("models").join("Wall A.glb"), b"wall")
        .expect("asset is written");
    std::fs::write(temp.path().join(".asset-mapper").join("ignored.glb"), b"ignored")
        .expect("metadata asset is written");
    std::fs::write(temp.path().join("notes.txt"), b"notes").expect("notes are written");

    let indexed = scan_assets(temp.path()).expect("scan succeeds");

    assert_eq!(indexed.len(), 1);
    assert_eq!(indexed[0].source_path, "models/Wall A.glb");
    assert_eq!(indexed[0].asset_type, AssetType::Model3d);
    assert!(indexed[0].content_hash.starts_with("sha256:"));
}

#[test]
fn init_pack_folder_creates_sidecar_with_placeholder_records() {
    let temp = tempfile::tempdir().expect("temp dir is created");
    std::fs::write(temp.path().join("wall.glb"), b"wall").expect("asset is written");

    let report = init_pack_folder(temp.path(), "Dungeon Kit".to_owned()).expect("init succeeds");
    let sidecar_path = canonical_sidecar_path(temp.path());

    assert_eq!(report.sidecar_path, sidecar_path.to_string_lossy().into_owned());
    assert_eq!(report.new_assets, vec!["wall.glb"]);
    assert!(sidecar_path.is_file());

    let loaded = read_pack_from_input(temp.path()).expect("sidecar reloads");
    assert_eq!(loaded.pack.pack_id, "dungeon_kit");
    assert_eq!(loaded.pack.display_name, "Dungeon Kit");
    assert_eq!(loaded.pack.assets.len(), 1);
    assert_eq!(loaded.pack.assets[0].asset_id, "wall");
    assert_eq!(loaded.pack.assets[0].source_path, "wall.glb");
    assert_eq!(loaded.pack.assets[0].asset_type, AssetType::Model3d);
    assert!(loaded.pack.assets[0].content_hash.starts_with("sha256:"));
    assert!(loaded.pack.assets[0].review_flags.contains(&ReviewFlag::BoundsPlaceholder));
    assert!(loaded.pack.assets[0]
        .review_flags
        .contains(&ReviewFlag::OrientationPlaceholder));
    assert!(loaded.pack.assets[0].review_flags.contains(&ReviewFlag::PivotPlaceholder));
}

#[test]
fn index_preserves_manual_metadata_and_reports_changes() {
    let temp = tempfile::tempdir().expect("temp dir is created");
    std::fs::write(temp.path().join("wall.glb"), b"wall-v1").expect("wall is written");
    std::fs::write(temp.path().join("floor.glb"), b"floor-v1").expect("floor is written");
    init_pack_folder(temp.path(), "Dungeon Kit".to_owned()).expect("init succeeds");

    let mut loaded = read_pack_from_input(temp.path()).expect("sidecar reloads");
    loaded.pack.assets[0].semantic_tags.push("manual_tag".to_owned());
    loaded.pack.assets.retain(|asset| asset.source_path == "wall.glb");
    asset_mapper_io::write_pack_sidecar(temp.path(), &loaded.pack).expect("sidecar rewrites");

    std::fs::write(temp.path().join("wall.glb"), b"wall-v2").expect("wall changes");
    std::fs::write(temp.path().join("ceiling.glb"), b"ceiling").expect("new asset is written");
    std::fs::remove_file(temp.path().join("floor.glb")).expect("floor is removed");

    let report = index_pack_folder(temp.path()).expect("index succeeds");

    assert_eq!(report.drifted_assets, vec!["wall.glb"]);
    assert_eq!(report.new_assets, vec!["ceiling.glb"]);
    assert!(report.missing_assets.is_empty());

    let reloaded = read_pack_from_input(temp.path()).expect("sidecar reloads");
    let wall = reloaded
        .pack
        .assets
        .iter()
        .find(|asset| asset.source_path == "wall.glb")
        .expect("wall record remains");
    assert_eq!(wall.semantic_tags, vec!["manual_tag"]);
    assert!(wall.content_hash.ends_with(&loaded.pack.assets[0].content_hash[7..]));

    let ceiling = reloaded
        .pack
        .assets
        .iter()
        .find(|asset| asset.source_path == "ceiling.glb")
        .expect("new ceiling record exists");
    assert_eq!(ceiling.asset_id, "ceiling");
}
```

- [ ] **Step 2: Run index tests to verify they fail**

Run:

```powershell
cargo test -p asset-mapper-io --test index
```

Expected: tests fail because indexing APIs and review flags do not exist.

- [ ] **Step 3: Add review flags to the schema**

In `crates/asset-mapper-core/src/schema.rs`, add this field to `AssetRecord` immediately after `placement_constraints`:

```rust
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub review_flags: Vec<ReviewFlag>,
```

Add this enum after `Pivot`:

```rust
#[derive(
    Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum ReviewFlag {
    BoundsPlaceholder,
    OrientationPlaceholder,
    PivotPlaceholder,
}
```

- [ ] **Step 4: Add hash access to the IO crate**

Modify `crates/asset-mapper-io/Cargo.toml` dependencies:

```toml
[dependencies]
asset-mapper-core = { path = "../asset-mapper-core" }
serde.workspace = true
serde_json.workspace = true
thiserror.workspace = true
```

No new hash dependency is needed because `asset-mapper-core::hash::sha256_file` already exists.

- [ ] **Step 5: Extend IO errors for indexing**

Add these variants to `IoError` in `crates/asset-mapper-io/src/error.rs`:

```rust
    #[error("pack sidecar already exists at `{path}`")]
    SidecarAlreadyExists { path: PathBuf },

    #[error("failed to scan directory `{path}`: {source}")]
    ScanDir {
        path: PathBuf,
        source: std::io::Error,
    },

    #[error("failed to read directory entry under `{path}`: {source}")]
    ReadDirEntry {
        path: PathBuf,
        source: std::io::Error,
    },

    #[error("failed to hash `{path}`: {source}")]
    HashFile {
        path: PathBuf,
        source: std::io::Error,
    },

    #[error("failed to derive a relative source path for `{path}` under `{root}`")]
    StripPackRoot { path: PathBuf, root: PathBuf },
```

If `SidecarAlreadyExists` already exists from Task 2, keep the existing variant and add only the missing variants.

- [ ] **Step 6: Export index APIs from the IO crate**

Modify `crates/asset-mapper-io/src/lib.rs`:

```rust
pub mod error;
pub mod index;
pub mod sidecar;

pub use error::IoError;
pub use index::{
    index_pack_folder, init_pack_folder, scan_assets, IndexReport, IndexedAsset,
    SUPPORTED_ASSET_EXTENSIONS,
};
pub use sidecar::{
    canonical_sidecar_path, read_pack_from_input, resolve_pack_input_path, write_pack_sidecar,
    LoadedPack, PackInputKind, ResolvedPackInput, METADATA_DIR, SIDECAR_FILE,
};
```

- [ ] **Step 7: Implement indexing**

Create `crates/asset-mapper-io/src/index.rs`:

```rust
use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::path::{Path, PathBuf};

use asset_mapper_core::{
    hash::sha256_file, AssetRecord, AssetType, Axis3, Bounds3, CoordinateConvention, Handedness,
    PackRecord, Pivot, ReviewFlag, Unit, CURRENT_SCHEMA_VERSION,
};
use serde::{Deserialize, Serialize};

use crate::error::IoError;
use crate::sidecar::{canonical_sidecar_path, read_pack_from_input, write_pack_sidecar};

pub const SUPPORTED_ASSET_EXTENSIONS: &[&str] = &[
    "glb", "gltf", "obj", "fbx", "png", "jpg", "jpeg", "webp",
];

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

pub fn init_pack_folder(
    pack_root: impl AsRef<Path>,
    display_name: String,
) -> Result<IndexReport, IoError> {
    let pack_root = pack_root.as_ref();
    let sidecar_path = canonical_sidecar_path(pack_root);
    if sidecar_path.exists() {
        return Err(IoError::SidecarAlreadyExists { path: sidecar_path });
    }

    let indexed = scan_assets(pack_root)?;
    let pack_id = slug_from_text(&display_name);
    let mut used_asset_ids = HashSet::new();
    let assets = indexed
        .iter()
        .map(|asset| placeholder_asset(asset, &mut used_asset_ids))
        .collect::<Vec<_>>();

    let pack = PackRecord {
        schema_version: CURRENT_SCHEMA_VERSION,
        pack_id,
        display_name,
        coordinate_convention: CoordinateConvention {
            handedness: Handedness::Right,
            up_axis: Axis3::PosY,
            forward_axis: Axis3::PosZ,
        },
        default_units: Unit::Meters,
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
        sidecar_path: canonical_sidecar_path(pack_root).to_string_lossy().into_owned(),
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
            assets.push(IndexedAsset {
                source_path,
                absolute_path: path,
                content_hash: format!("sha256:{hash}"),
                asset_type: asset_type_from_path(relative),
            });
        }
    }

    Ok(())
}

fn placeholder_asset(indexed: &IndexedAsset, used_asset_ids: &mut HashSet<String>) -> AssetRecord {
    AssetRecord {
        asset_id: unique_asset_id(&indexed.source_path, used_asset_ids),
        source_path: indexed.source_path.clone(),
        content_hash: indexed.content_hash.clone(),
        display_name: display_name_from_source_path(&indexed.source_path),
        asset_type: indexed.asset_type.clone(),
        bounds: Bounds3 {
            min: [-0.5, -0.5, -0.5],
            max: [0.5, 0.5, 0.5],
        },
        dimensions: [1.0, 1.0, 1.0],
        pivot: Pivot::Origin,
        up_axis: Axis3::PosY,
        forward_axis: Axis3::PosZ,
        semantic_tags: Vec::new(),
        affordances: Vec::new(),
        placement_constraints: Vec::new(),
        review_flags: vec![
            ReviewFlag::BoundsPlaceholder,
            ReviewFlag::OrientationPlaceholder,
            ReviewFlag::PivotPlaceholder,
        ],
        connectors: Vec::new(),
    }
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
    sources.into_iter().collect::<BTreeSet<_>>().into_iter().collect()
}
```

- [ ] **Step 8: Run index tests**

Run:

```powershell
cargo test -p asset-mapper-io --test index
```

Expected: all index tests pass.

- [ ] **Step 9: Run schema round-trip tests**

Run:

```powershell
cargo test -p asset-mapper-core --test schema_roundtrip
```

Expected: tests pass. Existing fixtures still round-trip because empty `review_flags` are skipped during serialization.

- [ ] **Step 10: Run workspace tests**

Run:

```powershell
cargo test
```

Expected: all tests pass.

- [ ] **Step 11: Commit indexing**

Run:

```powershell
git add -- Cargo.toml crates/asset-mapper-core/src/schema.rs crates/asset-mapper-io
git commit -m "feat: index asset pack folders"
```

Expected: commit succeeds.

---

### Task 4: Add CLI Init, Index, And Folder Inputs

**Files:**
- Modify: `crates/asset-mapper-cli/Cargo.toml`
- Modify: `crates/asset-mapper-cli/src/main.rs`
- Modify: `crates/asset-mapper-cli/tests/cli.rs`

- [ ] **Step 1: Add failing CLI tests for folder workflows**

Append these tests to `crates/asset-mapper-cli/tests/cli.rs`:

```rust
#[test]
fn init_creates_sidecar_for_pack_folder() {
    let temp = tempfile::tempdir().expect("temp dir is created");
    std::fs::write(temp.path().join("wall.glb"), b"wall").expect("asset is written");

    let mut command = Command::cargo_bin("asset-mapper").expect("binary exists");
    command
        .args([
            "init",
            temp.path().to_str().expect("temp path is utf-8"),
            "--name",
            "Dungeon Kit",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"new_assets\""))
        .stdout(predicate::str::contains("wall.glb"));

    assert!(temp
        .path()
        .join(".asset-mapper")
        .join("pack.assetmap.json")
        .is_file());
}

#[test]
fn index_reports_drift_and_new_assets() {
    let temp = tempfile::tempdir().expect("temp dir is created");
    std::fs::write(temp.path().join("wall.glb"), b"wall-v1").expect("asset is written");

    let mut init = Command::cargo_bin("asset-mapper").expect("binary exists");
    init.args([
        "init",
        temp.path().to_str().expect("temp path is utf-8"),
        "--name",
        "Dungeon Kit",
    ])
    .assert()
    .success();

    std::fs::write(temp.path().join("wall.glb"), b"wall-v2").expect("asset changes");
    std::fs::write(temp.path().join("floor.glb"), b"floor").expect("new asset is written");

    let mut index = Command::cargo_bin("asset-mapper").expect("binary exists");
    index
        .args(["index", temp.path().to_str().expect("temp path is utf-8")])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"drifted_assets\""))
        .stdout(predicate::str::contains("wall.glb"))
        .stdout(predicate::str::contains("\"new_assets\""))
        .stdout(predicate::str::contains("floor.glb"));
}

#[test]
fn validate_bundle_and_resolve_accept_pack_folder() {
    let temp = tempfile::tempdir().expect("temp dir is created");
    let metadata_dir = temp.path().join(".asset-mapper");
    std::fs::create_dir_all(&metadata_dir).expect("metadata dir is created");
    std::fs::copy(
        fixture_path("fixtures/phase0/simple_pack.assetmap.json"),
        metadata_dir.join("pack.assetmap.json"),
    )
    .expect("fixture sidecar copies");

    let mut validate = Command::cargo_bin("asset-mapper").expect("binary exists");
    validate
        .args(["validate", temp.path().to_str().expect("temp path is utf-8")])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"diagnostics\": []"));

    let mut bundle = Command::cargo_bin("asset-mapper").expect("binary exists");
    bundle
        .args(["bundle", temp.path().to_str().expect("temp path is utf-8")])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"pack_id\": \"phase0_corridor\""))
        .stdout(predicate::str::contains("orientation_quat_xyzw").not());

    let mut resolve = Command::cargo_bin("asset-mapper").expect("binary exists");
    resolve
        .args([
            "resolve",
            temp.path().to_str().expect("temp path is utf-8"),
            &fixture_path("fixtures/phase0/simple_plan.json"),
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"asset_id\": \"corridor_b\""))
        .stdout(predicate::str::contains("2.0"));
}
```

- [ ] **Step 2: Run CLI tests to verify they fail**

Run:

```powershell
cargo test -p asset-mapper-cli --test cli
```

Expected: tests fail because the CLI does not have `init`, `index`, or folder input support yet.

- [ ] **Step 3: Add IO dependency to the CLI crate**

Modify `crates/asset-mapper-cli/Cargo.toml`:

```toml
[dependencies]
asset-mapper-core = { path = "../asset-mapper-core" }
asset-mapper-io = { path = "../asset-mapper-io" }
clap.workspace = true
serde_json.workspace = true
```

- [ ] **Step 4: Replace the CLI entrypoint**

Replace `crates/asset-mapper-cli/src/main.rs` with:

```rust
use std::fs;
use std::path::PathBuf;
use std::process::ExitCode;

use asset_mapper_core::{resolve_plan, validate_pack, AssemblyPlan, LlmBundle};
use asset_mapper_io::{index_pack_folder, init_pack_folder, read_pack_from_input};
use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(name = "asset-mapper")]
#[command(about = "Headless Asset Mapper metadata tooling")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    Init {
        folder: PathBuf,
        #[arg(long)]
        name: String,
    },
    Index {
        folder: PathBuf,
    },
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
        Commands::Init { folder, name } => {
            let report = init_pack_folder(folder, name)?;
            println!("{}", serde_json::to_string_pretty(&report)?);
            Ok(ExitCode::SUCCESS)
        }
        Commands::Index { folder } => {
            let report = index_pack_folder(folder)?;
            println!("{}", serde_json::to_string_pretty(&report)?);
            Ok(ExitCode::SUCCESS)
        }
        Commands::Validate { pack } => {
            let loaded = read_pack_from_input(pack)?;
            let report = validate_pack(&loaded.pack);
            println!("{}", serde_json::to_string_pretty(&report)?);
            if report.is_valid() {
                Ok(ExitCode::SUCCESS)
            } else {
                Ok(ExitCode::from(1))
            }
        }
        Commands::Bundle { pack } => {
            let loaded = read_pack_from_input(pack)?;
            let bundle = LlmBundle::from_pack(&loaded.pack);
            println!("{}", serde_json::to_string_pretty(&bundle)?);
            Ok(ExitCode::SUCCESS)
        }
        Commands::Resolve { pack, plan } => {
            let loaded = read_pack_from_input(pack)?;
            let plan = read_plan(plan)?;
            let scene = resolve_plan(&loaded.pack, &plan)?;
            println!("{}", serde_json::to_string_pretty(&scene)?);
            Ok(ExitCode::SUCCESS)
        }
    }
}

fn read_plan(path: PathBuf) -> Result<AssemblyPlan, Box<dyn std::error::Error>> {
    let input = fs::read_to_string(&path).map_err(|error| {
        std::io::Error::new(
            error.kind(),
            format!("failed to read plan {}: {error}", path.display()),
        )
    })?;
    Ok(serde_json::from_str(&input)?)
}
```

- [ ] **Step 5: Run CLI tests**

Run:

```powershell
cargo test -p asset-mapper-cli --test cli
```

Expected: all CLI tests pass.

- [ ] **Step 6: Run workspace tests**

Run:

```powershell
cargo test
```

Expected: all tests pass.

- [ ] **Step 7: Commit CLI folder workflow**

Run:

```powershell
git add -- crates/asset-mapper-cli
git commit -m "feat: add pack folder cli workflow"
```

Expected: commit succeeds.

---

### Task 5: Add Maintenance Validation

**Files:**
- Modify: `crates/asset-mapper-core/src/diagnostics.rs`
- Modify: `crates/asset-mapper-core/src/validate.rs`
- Modify: `crates/asset-mapper-core/tests/validation.rs`
- Modify: `crates/asset-mapper-io/src/lib.rs`
- Create: `crates/asset-mapper-io/src/validation.rs`
- Create: `crates/asset-mapper-io/tests/source_validation.rs`
- Modify: `crates/asset-mapper-cli/src/main.rs`
- Modify: `crates/asset-mapper-cli/tests/cli.rs`

- [ ] **Step 1: Add failing core validation tests**

Append these tests to `crates/asset-mapper-core/tests/validation.rs`:

```rust
#[test]
fn duplicate_source_paths_are_errors() {
    let mut pack = load_pack("fixtures/phase0/simple_pack.assetmap.json");
    pack.assets[1].source_path = pack.assets[0].source_path.clone();

    let report = validate_pack(&pack);

    assert!(!report.is_valid());
    let diagnostic = find_code(&report.diagnostics, "duplicate_source_path")
        .expect("duplicate source path diagnostic is present");
    assert_eq!(diagnostic.severity, Severity::Error);
}

#[test]
fn non_finite_dimensions_are_errors() {
    let mut pack = load_pack("fixtures/phase0/simple_pack.assetmap.json");
    pack.assets[0].dimensions[0] = f32::NAN;

    let report = validate_pack(&pack);

    assert!(!report.is_valid());
    let diagnostic = find_code(&report.diagnostics, "non_finite_dimensions")
        .expect("non-finite dimensions diagnostic is present");
    assert_eq!(diagnostic.severity, Severity::Error);
    assert_eq!(diagnostic.asset_id.as_deref(), Some("corridor_a"));
}

#[test]
fn placeholder_review_flags_are_warnings() {
    let mut pack = load_pack("fixtures/phase0/simple_pack.assetmap.json");
    pack.assets[0]
        .review_flags
        .push(asset_mapper_core::ReviewFlag::BoundsPlaceholder);
    pack.assets[0]
        .review_flags
        .push(asset_mapper_core::ReviewFlag::OrientationPlaceholder);
    pack.assets[0]
        .review_flags
        .push(asset_mapper_core::ReviewFlag::PivotPlaceholder);

    let report = validate_pack(&pack);

    let bounds = find_code(&report.diagnostics, "placeholder_bounds")
        .expect("placeholder bounds diagnostic is present");
    let orientation = find_code(&report.diagnostics, "placeholder_orientation")
        .expect("placeholder orientation diagnostic is present");
    let pivot = find_code(&report.diagnostics, "placeholder_pivot")
        .expect("placeholder pivot diagnostic is present");
    assert_eq!(bounds.severity, Severity::Warning);
    assert_eq!(orientation.severity, Severity::Warning);
    assert_eq!(pivot.severity, Severity::Warning);
}
```

- [ ] **Step 2: Run core validation tests to verify they fail**

Run:

```powershell
cargo test -p asset-mapper-core --test validation
```

Expected: tests fail because the new diagnostics are not implemented.

- [ ] **Step 3: Add validation report merging**

Add this method to `ValidationReport` in `crates/asset-mapper-core/src/diagnostics.rs`:

```rust
    pub fn extend(&mut self, diagnostics: impl IntoIterator<Item = Diagnostic>) {
        self.diagnostics.extend(diagnostics);
    }
```

The full `impl ValidationReport` should become:

```rust
impl ValidationReport {
    pub fn new(diagnostics: Vec<Diagnostic>) -> Self {
        Self { diagnostics }
    }

    pub fn extend(&mut self, diagnostics: impl IntoIterator<Item = Diagnostic>) {
        self.diagnostics.extend(diagnostics);
    }

    pub fn is_valid(&self) -> bool {
        !self
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.severity == Severity::Error)
    }
}
```

- [ ] **Step 4: Extend core validation**

Modify `crates/asset-mapper-core/src/validate.rs`.

Add `ReviewFlag` to the schema imports:

```rust
use crate::schema::{CURRENT_SCHEMA_VERSION, ConnectorFrame, PackRecord, ReviewFlag};
```

Add a source path set near the existing `asset_ids` set:

```rust
    let mut source_paths = HashSet::new();
```

Inside the asset loop, immediately after duplicate asset ID validation, add:

```rust
        if !source_paths.insert(asset.source_path.as_str()) {
            diagnostics.push(
                Diagnostic::error(
                    "duplicate_source_path",
                    format!("source_path `{}` is duplicated", asset.source_path),
                )
                .with_asset(asset.asset_id.clone()),
            );
        }

        validate_finite_vec3(
            &mut diagnostics,
            "non_finite_dimensions",
            "asset dimensions must contain only finite numbers",
            asset.asset_id.as_str(),
            asset.dimensions,
        );

        validate_finite_vec3(
            &mut diagnostics,
            "non_finite_bounds",
            "asset bounds min must contain only finite numbers",
            asset.asset_id.as_str(),
            asset.bounds.min,
        );
        validate_finite_vec3(
            &mut diagnostics,
            "non_finite_bounds",
            "asset bounds max must contain only finite numbers",
            asset.asset_id.as_str(),
            asset.bounds.max,
        );

        validate_review_flags(&mut diagnostics, asset.asset_id.as_str(), &asset.review_flags);
```

Inside the connector loop, before `validate_connector_frame`, add:

```rust
            if !connector.snap_tolerance.is_finite() {
                diagnostics.push(
                    Diagnostic::error(
                        "non_finite_snap_tolerance",
                        "snap_tolerance must be finite",
                    )
                    .with_asset(asset.asset_id.clone())
                    .with_connector(connector.connector_id.clone()),
                );
            }
```

Add these helper functions before `bounds_are_ordered`:

```rust
fn validate_finite_vec3(
    diagnostics: &mut Vec<Diagnostic>,
    code: &str,
    message: &str,
    asset_id: &str,
    values: [f32; 3],
) {
    if values.iter().any(|value| !value.is_finite()) {
        diagnostics.push(Diagnostic::error(code, message).with_asset(asset_id.to_owned()));
    }
}

fn validate_review_flags(
    diagnostics: &mut Vec<Diagnostic>,
    asset_id: &str,
    review_flags: &[ReviewFlag],
) {
    for flag in review_flags {
        match flag {
            ReviewFlag::BoundsPlaceholder => diagnostics.push(
                Diagnostic::warning("placeholder_bounds", "asset bounds need author review")
                    .with_asset(asset_id.to_owned()),
            ),
            ReviewFlag::OrientationPlaceholder => diagnostics.push(
                Diagnostic::warning(
                    "placeholder_orientation",
                    "asset orientation needs author review",
                )
                .with_asset(asset_id.to_owned()),
            ),
            ReviewFlag::PivotPlaceholder => diagnostics.push(
                Diagnostic::warning("placeholder_pivot", "asset pivot needs author review")
                    .with_asset(asset_id.to_owned()),
            ),
        }
    }
}
```

Inside `validate_connector_frame`, before quaternion length calculation, add:

```rust
            if orientation_quat_xyzw
                .iter()
                .any(|component| !component.is_finite())
            {
                diagnostics.push(
                    Diagnostic::error(
                        "non_finite_connector_quaternion",
                        "3D connector quaternion must contain only finite numbers",
                    )
                    .with_asset(asset_id.to_owned())
                    .with_connector(connector_id.to_owned()),
                );
                return;
            }
```

- [ ] **Step 5: Run core validation tests**

Run:

```powershell
cargo test -p asset-mapper-core --test validation
```

Expected: all core validation tests pass.

- [ ] **Step 6: Add failing source-file validation tests**

Create `crates/asset-mapper-io/tests/source_validation.rs`:

```rust
use asset_mapper_core::{Severity, validate_pack};
use asset_mapper_io::{
    init_pack_folder, read_pack_from_input, validate_pack_sources, write_pack_sidecar,
};

#[test]
fn source_validation_reports_missing_drifted_and_untracked_files() {
    let temp = tempfile::tempdir().expect("temp dir is created");
    std::fs::write(temp.path().join("wall.glb"), b"wall-v1").expect("wall is written");
    std::fs::write(temp.path().join("floor.glb"), b"floor-v1").expect("floor is written");
    init_pack_folder(temp.path(), "Dungeon Kit".to_owned()).expect("init succeeds");

    let mut loaded = read_pack_from_input(temp.path()).expect("sidecar reloads");
    loaded.pack.assets.retain(|asset| asset.source_path == "wall.glb");
    write_pack_sidecar(temp.path(), &loaded.pack).expect("sidecar rewrites");

    std::fs::write(temp.path().join("wall.glb"), b"wall-v2").expect("wall drifts");
    std::fs::remove_file(temp.path().join("floor.glb")).expect("floor is removed");
    std::fs::write(temp.path().join("ceiling.glb"), b"ceiling").expect("ceiling is written");

    let report = validate_pack_sources(temp.path(), &loaded.pack).expect("source validation runs");

    assert!(report
        .diagnostics
        .iter()
        .any(|diagnostic| diagnostic.code == "source_hash_drift"
            && diagnostic.severity == Severity::Warning
            && diagnostic.asset_id.as_deref() == Some("wall")));
    assert!(report
        .diagnostics
        .iter()
        .any(|diagnostic| diagnostic.code == "source_file_untracked"
            && diagnostic.severity == Severity::Warning));
    assert!(!report
        .diagnostics
        .iter()
        .any(|diagnostic| diagnostic.code == "source_file_missing"));

    let mut missing_pack = loaded.pack.clone();
    missing_pack.assets[0].source_path = "missing.glb".to_owned();
    let missing_report =
        validate_pack_sources(temp.path(), &missing_pack).expect("source validation runs");
    assert!(missing_report
        .diagnostics
        .iter()
        .any(|diagnostic| diagnostic.code == "source_file_missing"
            && diagnostic.severity == Severity::Warning));

    let mut combined = validate_pack(&missing_pack);
    combined.extend(missing_report.diagnostics);
    assert!(combined
        .diagnostics
        .iter()
        .any(|diagnostic| diagnostic.code == "source_file_missing"));
}
```

- [ ] **Step 7: Run source validation tests to verify they fail**

Run:

```powershell
cargo test -p asset-mapper-io --test source_validation
```

Expected: tests fail because source-file validation does not exist.

- [ ] **Step 8: Export validation APIs from IO**

Modify `crates/asset-mapper-io/src/lib.rs`:

```rust
pub mod error;
pub mod index;
pub mod sidecar;
pub mod validation;

pub use error::IoError;
pub use index::{
    index_pack_folder, init_pack_folder, scan_assets, IndexReport, IndexedAsset,
    SUPPORTED_ASSET_EXTENSIONS,
};
pub use sidecar::{
    canonical_sidecar_path, read_pack_from_input, resolve_pack_input_path, write_pack_sidecar,
    LoadedPack, PackInputKind, ResolvedPackInput, METADATA_DIR, SIDECAR_FILE,
};
pub use validation::validate_pack_sources;
```

- [ ] **Step 9: Implement source-file validation**

Create `crates/asset-mapper-io/src/validation.rs`:

```rust
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use asset_mapper_core::{Diagnostic, PackRecord, ValidationReport};

use crate::error::IoError;
use crate::index::scan_assets;

pub fn validate_pack_sources(
    pack_root: impl AsRef<Path>,
    pack: &PackRecord,
) -> Result<ValidationReport, IoError> {
    let pack_root = pack_root.as_ref();
    let indexed = scan_assets(pack_root)?;
    let indexed_by_source = indexed
        .iter()
        .map(|asset| (asset.source_path.as_str(), asset))
        .collect::<BTreeMap<_, _>>();
    let metadata_sources = pack
        .assets
        .iter()
        .map(|asset| asset.source_path.as_str())
        .collect::<BTreeSet<_>>();

    let mut diagnostics = Vec::new();

    for asset in &pack.assets {
        match indexed_by_source.get(asset.source_path.as_str()) {
            Some(indexed_asset) if indexed_asset.content_hash != asset.content_hash => {
                diagnostics.push(
                    Diagnostic::warning(
                        "source_hash_drift",
                        format!(
                            "source file `{}` hash changed from `{}` to `{}`",
                            asset.source_path, asset.content_hash, indexed_asset.content_hash
                        ),
                    )
                    .with_asset(asset.asset_id.clone()),
                );
            }
            Some(_) => {}
            None => diagnostics.push(
                Diagnostic::warning(
                    "source_file_missing",
                    format!("source file `{}` is missing", asset.source_path),
                )
                .with_asset(asset.asset_id.clone()),
            ),
        }
    }

    for indexed_asset in indexed {
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
```

- [ ] **Step 10: Update CLI validate to include source diagnostics for folder inputs**

In `crates/asset-mapper-cli/src/main.rs`, update the IO import:

```rust
use asset_mapper_io::{index_pack_folder, init_pack_folder, read_pack_from_input, validate_pack_sources};
```

Replace the `Commands::Validate` arm with:

```rust
        Commands::Validate { pack } => {
            let loaded = read_pack_from_input(pack)?;
            let mut report = validate_pack(&loaded.pack);
            if let Some(pack_root) = loaded.resolved.pack_root.as_deref() {
                let source_report = validate_pack_sources(pack_root, &loaded.pack)?;
                report.extend(source_report.diagnostics);
            }
            println!("{}", serde_json::to_string_pretty(&report)?);
            if report.is_valid() {
                Ok(ExitCode::SUCCESS)
            } else {
                Ok(ExitCode::from(1))
            }
        }
```

- [ ] **Step 11: Update CLI folder validation test expectation**

In `validate_bundle_and_resolve_accept_pack_folder`, remove the assertion that folder validation prints exactly empty diagnostics, because the copied Phase 0 fixture has source paths but no copied asset files. Replace:

```rust
        .stdout(predicate::str::contains("\"diagnostics\": []"));
```

with:

```rust
        .stdout(predicate::str::contains("source_file_missing"));
```

- [ ] **Step 12: Run validation and CLI tests**

Run:

```powershell
cargo test -p asset-mapper-core --test validation
cargo test -p asset-mapper-io --test source_validation
cargo test -p asset-mapper-cli --test cli
```

Expected: all listed tests pass.

- [ ] **Step 13: Run workspace tests**

Run:

```powershell
cargo test
```

Expected: all tests pass.

- [ ] **Step 14: Commit maintenance validation**

Run:

```powershell
git add -- crates/asset-mapper-core crates/asset-mapper-io crates/asset-mapper-cli
git commit -m "feat: validate pack source files"
```

Expected: commit succeeds.

---

### Task 6: Final Verification And Manual Smoke

**Files:**
- No planned production changes.

- [ ] **Step 1: Run formatting check**

Run:

```powershell
cargo fmt -- --check
```

Expected: command exits with code `0`.

- [ ] **Step 2: Run Clippy with warnings denied**

Run:

```powershell
cargo clippy --all-targets --all-features -- -D warnings
```

Expected: command exits with code `0`.

- [ ] **Step 3: Run the full test suite**

Run:

```powershell
cargo test
```

Expected: all tests pass.

- [ ] **Step 4: Run direct sidecar smoke commands**

Run:

```powershell
cargo run -p asset-mapper-cli -- validate fixtures/phase0/simple_pack.assetmap.json
cargo run -p asset-mapper-cli -- bundle fixtures/phase0/simple_pack.assetmap.json
cargo run -p asset-mapper-cli -- resolve fixtures/phase0/simple_pack.assetmap.json fixtures/phase0/simple_plan.json
```

Expected:

- `validate` exits with code `0` and prints no diagnostics
- `bundle` exits with code `0`, includes `"pack_id": "phase0_corridor"`, and omits `orientation_quat_xyzw`
- `resolve` exits with code `0` and places `corridor_b` at translation `[0.0, 0.0, 2.0]`

- [ ] **Step 5: Run pack-folder smoke commands in a disposable folder**

Run:

```powershell
$smoke = Join-Path $env:TEMP "asset-mapper-phase1-smoke"
if (Test-Path -LiteralPath $smoke) {
    Remove-Item -LiteralPath $smoke -Recurse -Force
}
New-Item -ItemType Directory -Path $smoke | Out-Null
Set-Content -LiteralPath (Join-Path $smoke "wall.glb") -Value "wall-v1"
cargo run -p asset-mapper-cli -- init $smoke --name "Smoke Pack"
cargo run -p asset-mapper-cli -- validate $smoke
cargo run -p asset-mapper-cli -- bundle $smoke
Set-Content -LiteralPath (Join-Path $smoke "wall.glb") -Value "wall-v2"
Set-Content -LiteralPath (Join-Path $smoke "floor.glb") -Value "floor"
cargo run -p asset-mapper-cli -- index $smoke
cargo run -p asset-mapper-cli -- validate $smoke
```

Expected:

- `init` creates `$smoke/.asset-mapper/pack.assetmap.json`
- first `validate` exits with code `0` and reports placeholder warnings only
- `bundle` exits with code `0`
- `index` reports `wall.glb` under `drifted_assets` and `floor.glb` under `new_assets`
- second `validate` exits with code `0` and reports placeholder/hash-drift warnings

- [ ] **Step 6: Check git status**

Run:

```powershell
git status --short
```

Expected: no uncommitted changes after the implementation commits.

## Self-Review Notes

- Spec coverage: The plan covers the approved Phase 1 design: resolver fixture hardening, canonical sidecar layout, pack folder initialization, indexing, hash reconciliation, folder input support, source-file diagnostics, tests, and final verification.
- Scope control: The plan does not include editor UI, connector placement UI, broad geometry import, engine export, scene generation, chat behavior, or 2D resolver work.
- Type consistency: `ReviewFlag`, `IndexReport`, `IndexedAsset`, `LoadedPack`, `ResolvedPackInput`, `validate_pack_sources`, `init_pack_folder`, and `index_pack_folder` are introduced before later tasks use them.
