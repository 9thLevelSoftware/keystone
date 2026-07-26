# Vibe-builder handoff

Keystone does **not** vibe-build levels. It makes modular kits **machine-mappable** so other tools can plan assemblies honestly. This document is the contract for those tools (LLMs, procedural builders, engine plugins).

## Preferred pipeline

```text
init / index → measure-bounds → analyze → vibe-ready → bundle → (external plan) → resolve
```

1. **glTF/GLB preferred** for auto-map (mesh surface + portal sockets). OBJ works. FBX now contributes `Vertices` point clouds for sockets, but lacks scene transforms/indices — glTF remains best quality.
2. Run **`asset-mapper analyze`** (or editor **Analyze**) so connectors, classes, rules, `face_size`, and vocabulary-gated semantics are proposed.
3. Run **`asset-mapper vibe-ready <pack>`** until `ready: true` (or accept a known partial kit).
4. Export **`asset-mapper bundle <pack>`** and hand the JSON to a planner.
5. Planner emits an **AssemblyPlan** (no raw transforms).
6. **`asset-mapper resolve <pack> <plan>`** returns world placements, or a structured error report.

## Plan JSON contract

```json
{
  "root_asset_id": "wall_box",
  "operations": [
    {
      "placed_asset_id": "wall_door",
      "placed_connector_id": "wall_door_pos_x",
      "anchor_asset_id": "wall_box",
      "anchor_connector_id": "wall_box_neg_x",
      "rotation_choice_deg": 0
    }
  ]
}
```

| Field | Rules |
| --- | --- |
| `root_asset_id` | Must exist in the pack |
| `operations[]` | Each attaches a **new** asset onto an already placed anchor (root counts as placed) |
| `*_connector_id` | Must exist on that asset; never invent ids |
| `rotation_choice_deg` | Must be allowed by the matching compatibility rule (`locked` / `steps_deg` / `free`) |

**Unique asset rule:** each `asset_id` appears at most once in a resolved plan. The resolver keys placements by `asset_id`. For tile floors, external tools place **N copies** of the same mesh using the same connector metadata — Keystone does not multi-instance inside one plan.

### Bundle fields for planners

`LlmBundle` includes:

- `assets[].dimensions`
- `assets[].connectors[]` with `class`, `role`, optional **`face_size: [width, height]`**
- `compatibility_rules`
- `how_to_plan` — short natural-language contract
- `plan_contract` — field names + notes
- Controlled `vocabulary` (do not invent out-of-vocab tags)

Raw connector transforms / quaternions are **omitted** from the bundle on purpose.

## Resolve usage

### CLI

```powershell
asset-mapper bundle .\my-pack > bundle.json
# ... produce plan.json from bundle ...
asset-mapper resolve .\my-pack .\plan.json
```

On failure, stderr is a JSON **`ResolveErrorReport`**:

```json
{
  "code": "incompatible_connector_classes",
  "message": "...",
  "fix_target": "fix_pack",
  "guidance": "Add a compatibility rule pairing ...",
  "asset_id": "wall_door",
  "connector_id": "wall_door_pos_x",
  "secondary_asset_id": "wall_box",
  "secondary_connector_id": "wall_box_neg_x"
}
```

| `fix_target` | Meaning |
| --- | --- |
| `fix_plan` | Wrong ids, order, rotation, or mixed 2D/3D in the plan |
| `fix_pack` | Rules, connector frames, axes, or orientations in the pack |

### Failure loop (recommended)

1. Resolve fails → read `code` + `fix_target`.
2. If `fix_plan`: adjust plan JSON (ids / order / rotation).
3. If `fix_pack`: open pack in editor, select implicated asset/connector (editor **Import plan** does this), fix class/rule/geometry, re-bundle.
4. Re-run resolve.

Editor path: **Assembly preview → Import plan → Resolve plan**. Failures select the implicated asset/connector and show guidance.

## WASM

Crate `asset-mapper-wasm` exposes JSON string APIs:

| Function | Role |
| --- | --- |
| `validate_pack_json` | ValidationReport |
| `bundle_pack_json` | LlmBundle (with handoff fields) |
| `resolve_plan_json` | ResolvedScene; errors are JSON ResolveErrorReport strings |
| `vibe_ready_json` | VibeReadinessReport |
| `current_schema_version` | u32 |

## Engine consumer notes

### Unity

- Import pack folder + sidecar or bundle for tooling.
- On resolve success, instantiate prefabs at `placements[].transform.translation` / `rotation_quat_xyzw` (xyzw).
- Treat connectors as authoring metadata; runtime snap can re-use classes + `face_size`.

### Unreal

- Same placement fields; map quat xyzw → FQuat carefully (Unreal often wxyz).
- Export helpers: `asset-mapper export-engine --target unreal`.

### Godot

- Apply transforms to Node3D; quat is xyzw matching Godot `Quaternion(x,y,z,w)`.
- Export helpers: `asset-mapper export-engine --target godot`.

### glTF extras

- `asset-mapper export-gltf-extras` writes companion `*.keystone.json` for DCC pipelines.

## Bake-off (prove auto-map)

Generate real-kit-ish meshes and run the harness:

```powershell
# from repo root
node scripts/write-vibe-fixtures.mjs
cargo test -p asset-mapper-io vibe_kit_analyze_assemble_resolve -- --nocapture
```

Fixtures land in `fixtures/vibe/modular_kit/` (box wall, wall+door opening, L-corridor, floor tile, door piece). The test runs measure → analyze → propose_assembly → resolve and asserts multi-piece connectivity.

CLI equivalent on a pack folder:

```powershell
asset-mapper init .\fixtures\vibe\modular_kit --name "Vibe Kit" --license MIT --author "Studio"
asset-mapper analyze .\fixtures\vibe\modular_kit --replace
asset-mapper vibe-ready .\fixtures\vibe\modular_kit
asset-mapper propose-assembly .\fixtures\vibe\modular_kit --max-pieces 5 -o plan.json
asset-mapper resolve .\fixtures\vibe\modular_kit .\plan.json
```

## Tile / instance kits

- `propose-assembly` places **unique** assets only.
- Flags `--allow-asset-reuse` / `--max-instances-per-asset` document intent in report **notes** but do **not** change resolve semantics.
- External tools should instance `floor_tile` / `tile_edge` classes N times using the same connector ids from the pack.

## Schema note

`ConnectorRecord.face_size` is **optional**. Existing packs load without a schema version bump; analyze fills it on new proposals.
