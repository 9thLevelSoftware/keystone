# Getting started with Keystone (Asset Mapper)

This guide is for **production use**: install from a GitHub Release, map a modular pack, and produce trusted metadata without reading the source tree.

## 1. Install

### CLI (Windows)

1. Open the latest [GitHub Release](https://github.com/9thLevelSoftware/keystone/releases).
2. Download `asset-mapper-windows-x64.zip`.
3. Extract `asset-mapper.exe` somewhere on your PATH (or keep the folder handy).

```powershell
.\asset-mapper.exe --help
```

### Desktop editor (Windows)

1. From the same Release, download the **NSIS** or **MSI** installer when available.
2. Install and launch **Asset Mapper**.
3. Windows SmartScreen may warn on **unsigned** builds — use “More info → Run anyway” only if you trust the release source.

> **From source (developers):**  
> `cargo build --release -p asset-mapper-cli`  
> Editor: `cd crates/asset-mapper-editor && npm ci && npm run tauri:dev`

## 2. Create a pack

Put your `.glb` / `.gltf` (and other assets) in a folder, then:

```powershell
asset-mapper init .\my-pack --name "My Modular Kit" --license "MIT" --author "Your Studio"
asset-mapper measure-bounds .\my-pack
asset-mapper validate .\my-pack
```

**Required for production:**

| Field | Rule |
| --- | --- |
| `--license` | Real summary (not empty, not `UNSPECIFIED…`) |
| `--author` or `--source` | At least one non-empty |

`validate` may still report **warnings** (e.g. placeholder orientation until you review). **Errors** block export.

### In the editor

1. **Init** → choose folder → enter pack name, license, and author.  
2. If the yellow **Pack incomplete** banner appears, fill **Pack settings** (license + provenance).  
3. Prefer **Analyze** (below) over hand-placing every connector.

## 3. Auto-map then tweak (default path)

**What the product is for:** load a modular kit → the tool proposes sockets and rules → you tweak → you **see pieces snap together**.

### Editor

1. Open or Init the pack (license + author).  
2. Click **Analyze** — measures bounds, loads mesh samples (glTF/OBJ), and **proposes mating sockets** on mesh surfaces / openings (AABB face centers only when mesh is unavailable). Also proposes connector classes and compatibility rules.  
3. Review green connector markers. Drag/tweak positions and classes if needed.  
4. **Assembly preview** (right panel):
   - **Pack assembly → Auto layout pack** — multi-piece connected layout via the real resolver (no hand-written plan).  
   - **Two-piece mate** — pick two connectors for precise debug mates.  
5. **Validate** → fix errors → **Save**.

### CLI

```powershell
asset-mapper analyze .\my-pack
# force replace existing connectors:
asset-mapper analyze .\my-pack --replace
# AABB-only (skip mesh sockets):
asset-mapper analyze .\my-pack --aabb-only

# Multi-piece assembly plan (unique assets, greedy graph):
asset-mapper propose-assembly .\my-pack --max-pieces 8 -o plan.json
asset-mapper resolve .\my-pack .\plan.json

# Vibe-builder readiness (coverage, orphans, connectivity score):
asset-mapper vibe-ready .\my-pack
```

**Vibe handoff:** export `bundle`, let an external planner emit plan JSON, then `resolve`. See [vibe-builder-handoff.md](./vibe-builder-handoff.md). Prove auto-map with the [bake-off](./bake-off.md).

**Tile kits:** each `asset_id` is unique per resolve. Place N floor tiles outside Keystone using the same connector metadata.

**Manual path (optional):** Add connectors yourself, set classes/rules, then validate/save as before.

## 4. Export & resolve

```powershell
asset-mapper bundle .\my-pack > llm-bundle.json
asset-mapper export-engine .\my-pack --target unity
asset-mapper export-gltf-extras .\my-pack
```

Assembly plans (JSON) list root asset + connector attach operations:

```powershell
asset-mapper resolve .\my-pack .\my-plan.json
```

Invalid plans fail with a structured error (no silent wrong transforms).

## 5. Typical production loop

```text
init (license + author) → Analyze → tweak connectors/rules
  → Pack assembly preview → Validate → Save
  → bundle / export-engine → resolve plans from LLM or propose-assembly
  → re-index when files change → accept-drift after review
```

## Edge cases (not product ceilings)

| Topic | Behavior |
| --- | --- |
| Mesh sockets | glTF/GLB/OBJ primary; FBX and unreadable meshes fall back to AABB faces with a review flag |
| Auto layout | Uses each asset once (unique kit pieces); not an infinite tile world builder |
| Organic sculpture | Modular kit geometry is the target; not CAD socket AI for every mesh |

## Next

- [Desktop QA checklist](../superpowers/QA-DESKTOP.md)  
- [Release packaging](../superpowers/RELEASE.md)  
- [Status / feature matrix](../superpowers/STATUS.md)  
- [LLM plan proof](../superpowers/llm-plan-proof.md)  
