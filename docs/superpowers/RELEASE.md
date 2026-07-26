# Release packaging

## Tag-driven release (preferred)

1. Ensure `main` is green (CI).
2. Confirm versions in crates / `package.json` / `tauri.conf.json` match the tag.
3. Update `CHANGELOG.md` for the version.
4. Tag and push:

```powershell
git tag v0.2.0
git push origin v0.2.0
```

5. GitHub Actions workflow **Release** (`.github/workflows/release.yml`) builds:
   - `asset-mapper-windows-x64.zip` (CLI)
   - Editor NSIS/MSI under the release assets (when Tauri bundle succeeds)
6. Verify the [GitHub Releases](https://github.com/9thLevelSoftware/keystone/releases) page and download smoke on a clean machine.

### Signing

v0.2.x ships **unsigned**. Windows SmartScreen may warn. Code signing is a later trust milestone (P2). Do **not** commit certificates or signing secrets to this repo.

### Multi-OS

Primary release path is **Windows x64** CLI zip + editor installers. macOS/Linux CLI builds and a multi-OS CI matrix are future work unless already trivial in workflow files; keep docs accurate rather than claiming unsupported platforms.

## Manual local build (fallback)

### Prerequisites

- Rust stable (MSRV 1.85+)
- Node.js 20+
- Windows: WebView2; [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/) for installers

### CLI

```powershell
cargo build --release -p asset-mapper-cli
# target\release\asset-mapper.exe
.\target\release\asset-mapper.exe init .\demo --name "Demo" --license "MIT" --author "Studio"
.\target\release\asset-mapper.exe validate .\demo
```

### Editor installer

```powershell
cd crates\asset-mapper-editor
npm ci
npm run fixture:phase2
npm run tauri:build
# bundles under src-tauri\target\release\bundle\
```

## Verification gate before tagging

```powershell
cargo fmt -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --workspace

cd crates\asset-mapper-editor
npm ci
npm run fixture:phase2
npm test
npm run build
```

Also complete [QA-DESKTOP.md](./QA-DESKTOP.md) once per release train.

## Engine / glTF export formats

| Command | Output |
| --- | --- |
| `asset-mapper export-engine --target unreal` | JSON with flat connector + rule rows for DataTables |
| `asset-mapper export-engine --target unity` | ScriptableObject-friendly nested JSON |
| `asset-mapper export-engine --target godot` | Dictionary/resource-friendly JSON (`resource_type: KeystonePack`) |
| `asset-mapper export-engine --target csv` | Connector CSV for spreadsheets |
| `asset-mapper export-gltf-extras` | Companion `*.keystone.json` |

Canonical data remains the sidecar; engine/glTF outputs are mirrors.
