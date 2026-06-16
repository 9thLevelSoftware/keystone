# Phase 2 Editor MVP Design

Date: 2026-06-16

## Summary

Phase 2 turns the Phase 1 headless asset-pack workflow into a usable desktop editor. The editor should let a user open a real modular asset pack, inspect indexed assets, preview 3D assets, place and classify connectors, edit compatibility rules, validate the pack, save the canonical sidecar, and export the LLM bundle without hand-editing JSON.

The selected direction is a Tauri desktop app with a Three.js frontend and direct Rust command handlers over the existing `asset-mapper-core` and `asset-mapper-io` crates. Rust remains the authority for schema, validation, sidecar IO, indexing, and bundle generation. The frontend owns interaction, visualization, and temporary editor state.

## Relationship To The Overall Spec

The overall Asset Mapper design defines Phase 2 as the Editor MVP phase:

- open indexed pack
- preview assets
- inspect bounds and orientation
- manually place connectors
- assign connector classes
- edit compatibility rules
- run validation
- export sidecar and LLM bundle

The Phase 2 exit criterion is:

> A real modular asset pack can be mapped without hand-editing JSON.

This design keeps that boundary. It does not introduce procedural generation, chat behavior, cloud sync, marketplace workflow, engine export, or broad 2D authoring. The editor is a local desktop authoring tool for the metadata model proven by Phase 0 and made maintainable by Phase 1.

## Scope

Included:

- add a Tauri desktop editor workspace member
- add a Three.js-based 3D preview surface
- open an existing pack folder containing `.asset-mapper/pack.assetmap.json`
- initialize and index a pack folder when needed
- show indexed assets with source path, hash status, review flags, bounds, dimensions, and connector counts
- preview selected `.glb` and `.gltf` assets
- show unsupported preview states for formats that are indexed but not renderable in the MVP
- display origin axes, asset bounds, and connector markers
- create, move, rotate, rename, and delete 3D connectors
- assign connector classes and connector metadata
- edit compatibility rules
- provide numeric editing fallback for connector transforms
- run Rust validation from inside the editor
- show diagnostics grouped by asset, connector, or pack-level target where possible
- save the canonical sidecar through Rust IO
- export the LLM bundle through the same Rust bundle path used by the CLI
- add Rust command tests, frontend state tests, and a desktop smoke workflow

Excluded:

- 2D asset authoring
- automatic connector detection
- advanced mesh analysis
- procedural generation or scene assembly UI
- engine, DCC, or format-specific export
- cloud sync, collaboration, or remote pack storage
- marketplace-style browsing or presentation UI
- LLM/chat workflow
- asset generation

## Architecture

Phase 2 should add an editor crate rather than embedding desktop behavior into the CLI.

Expected workspace shape:

```text
crates/
  asset-mapper-core/
  asset-mapper-io/
  asset-mapper-cli/
  asset-mapper-editor/
```

`asset-mapper-core` remains the graphics-free source of truth for schema types, validation, resolver behavior, and LLM bundle projection.

`asset-mapper-io` remains responsible for filesystem concerns:

- resolving pack folders and sidecar paths
- loading and saving sidecar JSON
- scanning supported asset files
- computing source hashes
- reconciling indexed files against existing metadata
- validating source file maintenance state

`asset-mapper-editor` owns the desktop app:

- Tauri shell and command registration
- frontend app bundle
- local file/folder dialog integration
- command DTOs used by the frontend
- desktop smoke entry points where practical

The editor should call Rust crates directly from Tauri commands. It should not shell out to `asset-mapper-cli` for normal app behavior, because CLI output shape, binary paths, and packaged desktop environments are the wrong contract for editor internals.

## Backend Command Layer

Tauri commands should be narrow and testable. Most command behavior should live in ordinary Rust functions so it can be covered without launching a desktop shell.

Initial command surface:

- `open_pack_folder(path) -> EditorPackState`
- `init_pack_folder(path, options) -> EditorPackState`
- `index_pack_folder(path) -> IndexEditorResult`
- `save_pack(state) -> SaveEditorResult`
- `validate_pack(state) -> ValidationEditorResult`
- `export_bundle(state, output_path) -> ExportEditorResult`

The command layer should:

- load and save the canonical sidecar through `asset-mapper-io`
- use `asset-mapper-core` validation before save and export
- use the same bundle projection as the CLI
- return structured errors that the frontend can display without parsing stderr text
- reject export when validation reports errors
- allow save when validation reports warnings
- keep path handling Windows-safe and avoid absolute paths inside persisted sidecars

`PackRecord` remains the canonical persisted model. `EditorPackState` wraps it with editor-only derived data:

- pack folder path
- source file status
- selected asset id
- selected connector id
- unsaved changes flag
- validation diagnostics grouped by target

Editor-only state must not be written to `.asset-mapper/pack.assetmap.json`.

## Frontend

The frontend should be a working editor surface, not a landing page.

Recommended stack:

- Vite
- TypeScript
- React
- Three.js
- a small local state layer, chosen only after the first implementation pass shows whether React state is enough

Frontend responsibilities:

- maintain the current editor session state
- render asset previews with Three.js
- provide asset, connector, class, and compatibility-rule editing panels
- convert visual connector placement into schema-compatible transforms
- display validation output and save/export status
- preserve unsaved local edits until save, reload, or explicit discard
- avoid duplicating Rust validation rules in TypeScript except for immediate form-level affordances

The frontend may use generated or hand-maintained TypeScript DTOs for command payloads, but the Rust model remains authoritative before disk writes and exports.

## UI And Interaction Design

Primary layout:

- left sidebar: pack actions and asset list
- center: Three.js preview viewport for the selected asset
- right inspector: selected asset or connector details
- bottom panel: validation diagnostics and save/export status

Core interactions:

- open, initialize, and index pack folders through desktop dialogs
- select an asset from the indexed asset list
- orbit, pan, and zoom the preview camera
- toggle connector placement mode
- click or drag in the viewport to place a connector
- use transform controls for connector position and rotation
- edit connector id, class, tags, snap tolerance, and review flags in the inspector
- edit connector classes and compatibility rules in a dedicated rules view
- save sidecar
- validate
- export LLM bundle

Viewport expectations:

- load `.glb` and `.gltf` first
- show unsupported preview states for `.obj`, `.fbx`, and 2D image files in the MVP
- display asset bounds, origin axes, and connector markers
- encode connector class and validation status visually
- keep numeric transform editing available so imperfect picking does not block authoring
- assume rigid 3D modular assets and local files

Diagnostic behavior:

- validation errors and warnings appear in the bottom panel
- selecting a diagnostic selects the relevant asset or connector when target metadata is available
- diagnostics remain visible during save and export workflows
- export is blocked when Rust validation reports errors
- save is allowed with warnings so users can preserve incomplete authoring work

## Data Flow

The normal editor loop should be:

1. User opens or initializes a pack folder.
2. Tauri command loads or creates `.asset-mapper/pack.assetmap.json`.
3. Backend returns `EditorPackState` with the canonical pack plus derived file status.
4. Frontend renders the asset list and selected asset preview.
5. User edits connectors, connector classes, compatibility rules, and metadata.
6. Frontend updates local editor state and marks the session dirty.
7. User runs validation.
8. Backend validates the submitted pack state and returns structured diagnostics.
9. User saves.
10. Backend writes the canonical sidecar after normalizing and validating the submitted pack state.
11. User exports bundle.
12. Backend blocks on validation errors or writes the LLM bundle using the core bundle projection.

This keeps the desktop app responsive while preserving Rust as the final authority for persisted metadata.

## Testing Strategy

Phase 2 needs coverage at three levels.

Rust backend tests:

- opening a fixture pack folder
- initializing and indexing a temporary pack folder
- save/load round trip preserving canonical sidecar shape
- validation diagnostics returned through editor command logic
- export bundle using the same core bundle path as the CLI
- structured command errors for missing folders, invalid sidecars, failed writes, and validation-blocked exports

Frontend tests:

- connector create, update, delete state transitions
- connector class assignment state transitions
- compatibility-rule editing state transitions
- validation result grouping
- save and export error handling
- dirty-state behavior around save, reload, and validation

Desktop smoke verification:

- launch the Tauri app against a fixture pack
- open or initialize a pack folder
- select a `.glb` asset
- add a connector
- assign a connector class
- add a compatibility rule
- save the sidecar
- run validation
- export the LLM bundle
- assert the sidecar and bundle exist and contain expected connector and rule data

Manual verification should use at least one real or fixture modular `.glb` asset pack, not only JSON fixtures. Phase 2 is not complete if the editor only validates pre-authored JSON or still requires hand-editing the sidecar to place connectors.

The full Phase 2 verification gate should include:

```powershell
cargo fmt -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test
# frontend unit test command, once the editor package exists
# Tauri desktop smoke command, once the editor package exists
```

The exact frontend and Tauri commands should be finalized in the implementation plan after the package manager and Tauri template are selected.

## Implementation Sequence

1. Add `asset-mapper-editor` as a workspace member with Tauri, Vite, TypeScript, React, and Three.js.
2. Add a Rust editor command layer over `asset-mapper-core` and `asset-mapper-io`.
3. Build open, initialize, index, save, validate, and export flows.
4. Build the main editor shell with asset list, viewport, inspector, and diagnostics panel.
5. Add `.glb` and `.gltf` preview with camera controls, axes, bounds, and unsupported-format states.
6. Add connector create, update, delete behavior with transform controls and numeric inspector fallback.
7. Add connector class and compatibility-rule editing.
8. Add validation diagnostic grouping and target selection.
9. Add regression tests for Rust command logic and frontend state helpers.
10. Add desktop smoke verification using a real fixture pack.

The first implementation milestone should be:

> Open a fixture pack in Tauri, preview one `.glb`, save, and validate without editing JSON.

Connector authoring should come immediately after that milestone.

## Exit Criteria

Phase 2 is complete when a user can use the desktop editor to:

- open or initialize a local modular asset pack
- index source assets
- preview at least `.glb` and `.gltf` assets
- inspect asset bounds, dimensions, orientation metadata, hash status, review flags, and connector counts
- create and edit 3D connectors without hand-editing JSON
- assign connector classes
- edit compatibility rules
- run validation and understand blocking errors
- save `.asset-mapper/pack.assetmap.json`
- export an LLM bundle
- verify the resulting sidecar and bundle with the existing Rust validation and CLI paths

The editor does not need to be polished as a distributable product yet, but the end-to-end desktop authoring loop must work on a real modular asset pack.

## Risks

### Tauri Setup Can Consume The Phase

Desktop packaging can distract from the authoring workflow. The first milestone should prove Tauri can open a pack, preview a `.glb`, save, and validate before deeper editor controls are built.

### Frontend Logic Can Drift From Rust Validation

The frontend should not duplicate schema rules as a second source of truth. Rust must validate before save and export, while TypeScript provides only immediate form affordances and display grouping.

### 3D Picking May Be Imprecise

Connector placement depends on usable picking and transforms. Numeric transform editing is required as a fallback so the MVP remains useful even when viewport placement is not perfect.

### Asset Loading Scope Can Expand Too Quickly

The previewer should support `.glb` and `.gltf` first. Other indexed formats can remain visible but preview-unavailable until the core authoring loop is reliable.

### Validation Targeting May Need Better Metadata

Existing diagnostics may not always identify an exact asset or connector target. Phase 2 should group diagnostics when possible and may need small diagnostic-shape improvements, but it should avoid a broad validator rewrite unless the editor needs it.
