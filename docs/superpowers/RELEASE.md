# Release packaging

## Prerequisites

- Rust stable (MSRV 1.85+)
- Node.js 20+
- On Windows: WebView2 (preinstalled on modern Windows 10/11)
- For Tauri installers: see [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/)

## Version bump

1. Update crate versions in:
   - `crates/asset-mapper-core/Cargo.toml`
   - `crates/asset-mapper-io/Cargo.toml`
   - `crates/asset-mapper-cli/Cargo.toml`
   - `crates/asset-mapper-editor/src-tauri/Cargo.toml`
   - `crates/asset-mapper-editor/package.json`
   - `crates/asset-mapper-editor/src-tauri/tauri.conf.json` (`version`)
2. Update `CHANGELOG.md`
3. Commit and tag: `git tag v0.1.0`

## CLI release binary

```powershell
cargo build --release -p asset-mapper-cli
# Windows: target\release\asset-mapper.exe
```

Smoke:

```powershell
.\target\release\asset-mapper.exe --help
.\target\release\asset-mapper.exe validate fixtures\phase0\simple_pack.assetmap.json
```

## Editor installer (Tauri)

```powershell
cd crates\asset-mapper-editor
npm ci
npm run fixture:phase2
npm run tauri:build
```

Artifacts are written under `crates/asset-mapper-editor/src-tauri/target/release/bundle/`
(NSIS/MSI on Windows when bundling is enabled).

`tauri.conf.json` is release-ready:

- `productName`: Asset Mapper
- `identifier`: `software.9thlevel.asset-mapper`
- `bundle.active`: true
- `bundle.targets`: all

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

## Engine / glTF export formats

| Command | Output |
| --- | --- |
| `asset-mapper export-engine --target unreal` | JSON with flat connector + rule rows for DataTables |
| `asset-mapper export-engine --target unity` | ScriptableObject-friendly nested JSON |
| `asset-mapper export-engine --target godot` | Dictionary/resource-friendly JSON (`resource_type: KeystonePack`) |
| `asset-mapper export-engine --target csv` | Connector CSV for spreadsheets |
| `asset-mapper export-gltf-extras` | Companion `*.keystone.json` (or embed under glTF `extras.keystone`) |

Canonical data remains the sidecar; engine/glTF outputs are mirrors.
