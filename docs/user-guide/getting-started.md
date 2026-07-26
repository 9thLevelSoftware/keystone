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
3. **Measure** bounds, then author connectors and rules.

## 3. Auto-map then tweak (editor)

**Default path (what the product is for):**

1. Open or Init the pack (license + author).  
2. Click **Analyze** — measures bounds and **proposes connectors on mesh bounds faces**, connector classes, and compatibility rules.  
3. Review proposed connectors in the viewport (green markers). Tweak positions/classes if needed.  
4. Use **Assembly preview** (right panel): pick two connectors → **Preview mate** to see the resolver attach them in 3D.  
5. **Validate** → fix errors → **Save**.

**CLI equivalent:**

```powershell
asset-mapper analyze .\my-pack
# force replace existing connectors:
asset-mapper analyze .\my-pack --replace
```

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
init (license + author) → measure-bounds → author in editor → validate → save
  → bundle / export-engine → resolve plans from LLM or tools
  → re-index when files change → accept-drift after review
```

## Next

- [Desktop QA checklist](../superpowers/QA-DESKTOP.md)  
- [Release packaging](../superpowers/RELEASE.md)  
- [Status / feature matrix](../superpowers/STATUS.md)  
- [LLM plan proof](../superpowers/llm-plan-proof.md)  
