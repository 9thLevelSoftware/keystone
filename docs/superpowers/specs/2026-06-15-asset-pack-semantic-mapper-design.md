# Asset Pack Semantic Mapper Design

Date: 2026-06-15

## Summary

Asset Mapper is a semantic mapping tool for prebuilt 2D and 3D asset packs. It does not generate assets, generate worlds, or provide the "vibe building" chat experience itself. Its job is to produce trustworthy metadata that lets an external LLM or downstream tool assemble the pack later.

The app turns a folder of assets into a portable sidecar record describing each asset's dimensions, orientation, pivot, connector locations, valid connector relationships, and semantic labels. A compact LLM bundle gives a thinking model the "jigsaw puzzle" view of the pack, while deterministic validation and resolver logic keep geometry math out of the LLM.

## Product Boundary

Asset Mapper is responsible for:

- importing or indexing a prebuilt asset pack
- measuring or recording asset dimensions and bounds
- declaring coordinate conventions, units, up axes, forward axes, and pivot semantics
- authoring connector locations as precise local-space frames or anchors
- classifying connectors into reusable connector classes
- defining valid class-to-class connection rules
- adding semantic tags, affordances, and placement constraints
- validating completeness and consistency of metadata
- exporting canonical sidecar metadata next to the asset pack
- exporting a compact LLM-readable context bundle
- optionally exporting lossy engine, DCC, or format-specific metadata later

Asset Mapper is not responsible for:

- generating new assets
- generating scenes or worlds
- hosting a chat interface
- choosing what to build from a prompt
- replacing game-engine placement, rendering, navmesh, lighting, or gameplay systems
- guaranteeing that every LLM will produce a good design

The core product contract is:

> The LLM chooses which pieces and connectors should attach. Asset Mapper metadata plus resolver logic makes that choice geometrically valid or rejects it with a structured reason.

## Recommended Approach

Use the "metadata mapper plus validation/resolver contract" architecture.

A metadata-only mapper would be faster to build, but would leave downstream tools to interpret transforms, connector orientation, and compatibility rules on their own. That would recreate the same failure mode the product is meant to avoid: LLMs or scripts guessing geometry.

A plugin-first approach for Blender, Unreal, Unity, or Godot may become valuable later, but it would tie the canonical data model to one ecosystem too early.

The right first product is a standalone, portable metadata system with deterministic validation and resolver behavior. The editor is a human-friendly view over that data, not the source of truth.

## Architecture

The system has three layers.

### 1. Core Metadata Model

The core model is the canonical schema for packs, assets, connectors, and compatibility rules. It must not depend on an editor, game engine, rendering framework, or specific asset format.

The core model defines:

- pack identity and schema version
- coordinate convention and units
- asset records
- bounds and dimensions
- orientation metadata
- pivot/origin metadata
- connector definitions
- connector classes
- compatibility rules
- semantic tags and affordances
- placement constraints
- provenance and license fields where useful
- migrations between schema versions

This layer is the source of truth.

### 2. Validator And Resolver

The validator checks whether metadata is complete and internally consistent.

Validation should detect:

- missing connector classes
- connector classes that participate in no valid rule
- invalid or degenerate connector frames
- missing bounds or dimensions
- missing coordinate convention
- non-normalized 3D orientation data
- ambiguous or contradictory orientation metadata
- content-hash drift when an asset changes silently
- invalid compatibility rules

The resolver is a deterministic geometry helper, not a worldbuilder. Given a proposed operation such as:

```text
attach(asset_a.connector_x, asset_b.connector_y, rotation_choice)
```

it either computes the required transform or rejects the operation with a structured error.

The editor and downstream SDKs must call the resolver instead of reimplementing transform math.

### 3. Authoring App

The authoring app is the human-facing tool for mapping an asset pack.

It provides:

- pack import or indexing
- asset preview
- dimension and bounds inspection
- orientation and pivot review
- connector placement
- connector class assignment
- compatibility-rule editing
- semantic tagging
- validation report
- canonical sidecar export
- LLM bundle export

The editor stores data through the core model. It should not invent editor-only metadata that cannot be exported or validated.

## Data Model

The metadata has three primary scopes: pack, asset, and connector.

### Pack Record

The pack record contains global identity and rules.

Expected fields:

- `schema_version`
- `pack_id`
- `display_name`
- `coordinate_convention`
- `default_units`
- `assets`
- `connector_classes`
- `compatibility_rules`
- optional `provenance`
- optional `license_summary`

Pack-level compatibility rules must be class-based, not pairwise between individual assets.

Example class rules:

```text
wall_edge connects to wall_edge
doorway connects to door_frame
floor_edge connects to floor_edge
roof_socket connects to wall_top
road_end connects to road_end
```

Pairwise rules such as `wall_001_left connects to wall_002_right` should be avoided except for rare, intentional one-off constraints.

### Asset Record

Each asset gets a stable record.

Expected fields:

- `asset_id`
- `source_path`
- `content_hash`
- `display_name`
- `asset_type`
- `bounds`
- `dimensions`
- `pivot`
- `up_axis`
- `forward_axis`
- `semantic_tags`
- `affordances`
- `placement_constraints`
- `connectors`

The `asset_id` should not depend only on a filename. It should include or reference a content hash so silent mesh or sprite changes can be detected.

### Connector Record

A connector is a local-space attachment point.

Expected fields:

- `connector_id`
- `display_name`
- `class`
- `role`
- `frame`
- `mating_axis`
- `up_reference`
- `allowed_rotation`
- `snap_tolerance`
- optional `notes`

For 3D assets, `frame` should be a position plus quaternion or explicit basis. Euler angles should not be used in the canonical record because rotation order and gimbal ambiguity create avoidable bugs.

For 2D assets, connector frames should be adapted to 2D concepts:

- grid cell coordinates
- edge side
- pixel-local or asset-local anchor position
- normal direction
- allowed rotations
- allowed flips
- tile, sprite, collision, or render-layer constraints

2D and 3D should share high-level concepts, but the schema should allow distinct frame types rather than forcing both through a single 3D quaternion model.

## Compatibility Rules

Compatibility rules define which connector classes can attach and under what constraints.

Rules should support:

- symmetric connections
- plug/receptacle connections
- locked rotation
- stepped rotation
- allowed flips for 2D assets
- allowed scale class or module size
- tolerance for snap validation

Rules should be human-readable enough for authoring, but strict enough for deterministic validation and resolver behavior.

## Main Workflow

### 1. Create Or Open Pack

The user selects a folder of existing assets. The app indexes files, computes hashes, detects basic dimensions and bounds where possible, and creates or loads the pack record.

### 2. Inspect Assets

The user reviews each asset's preview, dimensions, pivot, orientation, up axis, forward axis, and detected bounds. The app should highlight suspicious scale, orientation, or pivot issues.

### 3. Place Connectors

The user adds connectors directly on the asset preview.

For 3D assets, this means placing local-space connector frames on or near the mesh.

For 2D assets, this means placing anchors, edges, grid positions, or sprite-local connector points.

### 4. Classify Connectors

The user assigns each connector a reusable class such as:

- `wall_edge`
- `floor_edge`
- `door_frame`
- `road_end`
- `pipe_socket`
- `roof_mount`
- `tile_north_edge`
- `sprite_anchor_bottom`

### 5. Define Valid Connections

The user defines class-to-class rules once at the pack level. This is the jigsaw-puzzle layer: it tells a future LLM which edges can mate.

### 6. Tag Semantics

The user labels assets with compact, useful terms such as:

- `wall`
- `floor`
- `corner`
- `door`
- `window`
- `walkable`
- `cover`
- `decorative`
- `hazard`
- `lootable`
- `entry`
- `exit`

The initial vocabulary should be small and controlled, with namespaced extensions allowed for project-specific tags.

### 7. Validate

The app reports missing connector classes, orphan connector types, invalid frames, missing bounds, inconsistent units, possible orientation problems, and geometry/hash drift.

Validation should produce both human-readable findings and machine-readable error codes.

### 8. Export

The app writes:

- canonical sidecar metadata
- validation report
- compact LLM context bundle
- optional engine, DCC, or format-specific exports in later phases

The sidecar remains canonical. Format-specific embedding is a lossy mirror.

## LLM Bundle

The LLM bundle is a compact derivative of the canonical metadata.

It should include:

- asset IDs
- short descriptions
- semantic tags
- dimensions or coarse bounds
- connector IDs, labels, classes, and roles
- placement constraints
- compatibility rule summary

It should not include raw quaternions, full matrices, or implementation-heavy geometry data unless a downstream consumer explicitly needs them. The LLM should reason over names, classes, roles, and constraints. The resolver should handle math.

## Technical Direction

The recommended implementation is a local-first app with a headless core.

Initial workspace shape:

```text
asset-mapper/
  crates/
    asset-mapper-core/
    asset-mapper-cli/
    asset-mapper-io/
    asset-mapper-editor/
  fixtures/
  docs/
```

### Core

Use Rust for the core model, validation, resolver, import/export helpers, and CLI.

Likely responsibilities:

- `asset-mapper-core`: schema types, connector frames, compatibility rules, resolver, validator
- `asset-mapper-cli`: validate, inspect, bundle, and resolve test plans
- `asset-mapper-io`: asset indexing, hashing, sidecar read/write, basic glTF inspection
- `asset-mapper-editor`: GUI after the core is proven

The core should be fully testable without graphics.

### Editor

Two editor options remain viable:

- Rust native editor with Bevy plus egui or bevy_egui
- local web or Tauri editor with Three.js

The editor choice should follow the proof harness and early asset-loading experiments. If precise 3D picking and socket-frame authoring are easier in Bevy, use Bevy. If distribution and UI iteration matter more, a web/Tauri editor may be better.

Do not start with editor implementation before the metadata model and resolver are proven.

## MVP Recommendation

The MVP should focus on rigid modular 3D asset packs first.

Reason:

- connector frames and compatibility rules are central to the value proposition
- rigid modular kits expose the transform/resolver problem clearly
- the validation and LLM-bundle workflow can be tested without a full editor
- supporting 2D and 3D authoring UX at the same time would slow the MVP

2D should remain in the schema direction, but 2D authoring workflows can follow after the 3D core proves the model.

## Roadmap

### Phase 0: Proof Harness

Scope:

- define schema draft
- hand-author a tiny fixture pack metadata file
- implement validation
- implement resolver
- export compact LLM bundle
- test whether an LLM can produce valid connector-level assembly plans

Exit criterion:

The fixture pack can be described to an LLM, the LLM can choose connector pairings, and the resolver accepts valid operations while rejecting invalid ones deterministically.

### Phase 1: Core CLI

Scope:

- index a folder
- compute hashes
- compute or record bounds where possible
- read and write sidecar metadata
- validate a pack
- emit LLM bundle
- run a resolver test plan

Exit criterion:

Pack metadata can be maintained and verified without a GUI.

### Phase 2: Editor MVP

Scope:

- open indexed pack
- preview assets
- inspect bounds and orientation
- manually place connectors
- assign connector classes
- edit compatibility rules
- run validation
- export sidecar and LLM bundle

Exit criterion:

A real modular asset pack can be mapped without hand-editing JSON.

### Phase 3: Export And Integration

Scope:

- glTF metadata mirroring
- Unreal, Unity, or Godot export helpers if useful
- 2D tile and sprite support if still desired
- schema migration and versioning
- authoring-speed improvements

Exit criterion:

External tools can consume a mapped pack without custom hand translation.

## Testing And Verification Strategy

The resolver and schema need stronger testing than the editor.

Recommended tests:

- schema serialization snapshots
- validation fixtures for good and bad packs
- resolver golden tests with known transforms
- property tests for resolver invariants
- LLM-bundle snapshot tests
- hash-drift tests
- CLI end-to-end tests over fixture packs

Resolver bugs are high-impact because they silently produce wrong assemblies. Treat resolver behavior as the most important correctness surface.

## Risks

### Authoring Time May Not Scale

Manual connector authoring can become tedious for large packs. The MVP should measure authoring time per asset so future automation work is justified by evidence.

### 2D And 3D UX Can Diverge

The schema can support both, but the editor interactions are different. Building both at once risks a weaker MVP.

### Format Imports Are Product Risk

glTF is a reasonable first target. FBX and USD should be treated as later import/export work, not MVP requirements.

### LLMs May Still Make Bad Design Choices

The metadata can make geometry valid. It cannot guarantee tasteful, playable, or coherent world design by itself. The app's success criterion is reliable assembly affordance, not perfect creative output.

### Metadata Standards May Evolve

The canonical sidecar should remain independent from any single engine or format. glTF, USD, and engine exports should be mirrors that can evolve without breaking the source record.

## Open Decisions

- Exact sidecar file name and folder layout
- Rust native editor versus web/Tauri editor
- First supported 3D import format beyond basic glTF inspection
- Initial controlled semantic vocabulary
- Whether the resolver ships only as CLI/library or also as WASM bindings in the first public release
- How much provenance/licensing metadata is mandatory in v1

## Completion Criteria For The First Implementation Plan

The first implementation plan should target Phase 0 only.

It should not include the editor.

It should produce:

- a schema draft
- a hand-authored fixture pack
- a validator
- a deterministic resolver
- a compact LLM bundle exporter
- tests proving valid and invalid connector operations

Only after Phase 0 proves the core contract should the project move into CLI ergonomics and editor UX.
