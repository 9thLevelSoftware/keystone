# Auto-map bake-off

Prove that Keystone can auto-map a modular kit and assemble multiple pieces without hand-authored connectors.

**Product goal:** auto-map connectors so *other tools* can vibe-build — Keystone is not the vibe builder.

## Real pack: Modular SciFi MegaKit (local only)

Do **not** commit commercial kit meshes to the repo. Point the harness at your install:

```powershell
$env:KEYSTONE_BAKEOFF_PACK = "C:\Users\dasbl\Downloads\Modular SciFi MegaKit[Source]\glTF"
.\scripts\bakeoff-megakit.ps1
```

Recommended Analyze recipe (excludes decals/aliens/textures):

```powershell
asset-mapper analyze $env:KEYSTONE_BAKEOFF_PACK --replace `
  --exclude-glob "Decals/**" `
  --exclude-glob "Aliens/**" `
  --exclude-glob "*.png"
```

### Metrics that matter

| Metric | Target |
| --- | --- |
| `doorway_fraction` | **&lt; 25%** of connectors (walls must not all be doorway) |
| Stratified sample plausible | ≥ 60% (walls→`wall_edge`, doors→`doorway`, platforms→`floor_edge`) |
| propose + resolve | multi-piece plan resolves |
| vibe-ready | not “ready” under class monopoly |

Reports land under `target/bakeoff/megakit-*.json` and `*.md`.

### Known MegaKit failure mode (fixed in classification pass)

Early auto-map labeled almost every wall connector as `doorway` because weak mesh “portals” promoted class. Walls under `Walls/` now default to `wall_edge`; doorway only for door-named assets or **strong** portals.

## Generate fixtures

```powershell
node scripts/write-vibe-fixtures.mjs
```

Writes glTF/GLB pieces under `fixtures/vibe/modular_kit/`:

| File | Intent |
| --- | --- |
| `wall_box.glb` | Solid box wall |
| `wall_door.glb` | Wall with off-center door opening (portal) |
| `corridor_l.glb` | L-shaped corridor |
| `floor_tile.glb` | Floor slab |
| `door_piece.glb` | Door leaf / plug-sized piece |

## Automated harness

```powershell
cargo test -p asset-mapper-io vibe_kit_analyze_assemble_resolve -- --nocapture
```

This test:

1. Ensures fixtures exist (runs the node script if needed)
2. `init` + `measure` + `analyze` in a temp pack
3. Asserts connectors were proposed
4. `vibe_readiness` coverage / orphan checks
5. `propose_assembly_plan` places ≥2 pieces
6. `resolve_plan` succeeds

## Manual CLI bake-off

```powershell
$kit = ".\fixtures\vibe\modular_kit"
# if not already a pack:
asset-mapper init $kit --name "Vibe Kit" --license MIT --author "Studio"
asset-mapper analyze $kit --replace
asset-mapper vibe-ready $kit
asset-mapper propose-assembly $kit --max-pieces 5 -o .\plan.json
asset-mapper resolve $kit .\plan.json
asset-mapper bundle $kit > .\bundle.json
```

Expect: mesh sockets on glTF pieces where geometry allows, multi-piece resolve output, vibe score trending ready after analyze.
