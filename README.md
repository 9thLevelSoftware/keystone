# Keystone

Headless Rust tooling for **semantic mapping of prebuilt 2D and 3D asset packs**.

Keystone is the canonical metadata layer for jigsaw-style asset packs: it captures each asset's dimensions, orientation, pivot, connector locations, connector classes, and valid class-to-class compatibility rules, and exposes that metadata through a deterministic validator and a connector resolver. It does not generate assets, build worlds, or host a chat experience — its job is to make downstream assembly (by an LLM or another tool) geometrically trustworthy.

> The LLM chooses which pieces and connectors should attach. Keystone's metadata plus resolver logic makes that choice geometrically valid or rejects it with a structured reason.

## Status

**Product path:** load a modular kit → **Analyze** (mesh sockets + rules) → **vibe-ready** → tweak → **Pack assembly** preview → bundle handoff → resolve.  
**Current release:** v0.2.0 + unreleased vibe-ready facilitation on `feat/vibe-ready-facilitation`.

**User docs:** [Getting started](docs/user-guide/getting-started.md) · [Vibe-builder handoff](docs/user-guide/vibe-builder-handoff.md) · [Bake-off](docs/user-guide/bake-off.md) · [Desktop QA](docs/superpowers/QA-DESKTOP.md) · [Release](docs/superpowers/RELEASE.md) · [Status matrix](docs/superpowers/STATUS.md)

Install from [GitHub Releases](https://github.com/9thLevelSoftware/keystone/releases) (Windows CLI zip + editor installer).

| Phase | Scope | Status |
| --- | --- | --- |
| 0 | Core schema, validator, LLM bundle, connector resolver | Done |
| 1 | Headless CLI for pack folder workflow | Done |
| 2 | Interactive editor MVP | Done |
| 3 | Engine/export integration, 2D authoring, migrations | Done |
| Prod gates | Provenance, vocabulary, WASM, full editor UI, FBX bounds | Done |
| Honest limits | Mesh sockets, rich auto-rules, pack assembly preview | Done |
| Vibe-ready | Readiness report, handoff contract, bake-off, resolve feedback | In progress (this branch) |

## Workspace

Cargo workspace, edition 2024, MSRV `1.85`, dual-licensed MIT OR Apache-2.0:

- **`asset-mapper-core`** — canonical schema, `validate_pack`, content hashing, `LlmBundle`, `resolve_plan` (3D + Frame2d), migrations, engine/glTF export helpers, authoring suggestions. No I/O dependencies.
- **`asset-mapper-io`** — pack folder indexing, bounds measurement (glTF/OBJ/images/ASCII+binary FBX), mesh samples for sockets (glTF/OBJ/FBX Vertices), sidecar read/write, accept-drift, migration IO.
- **`asset-mapper-cli`** — the `asset-mapper` binary (`clap` derive subcommands).
- **`asset-mapper-editor`** — Tauri v2 desktop editor (React + Three.js) over the same core/IO crates.
- **`asset-mapper-wasm`** — WASM JSON APIs: `validate_pack_json`, `resolve_plan_json`, `bundle_pack_json`, `vibe_ready_json`.

## Install / Build

```bash
git clone https://github.com/9thLevelSoftware/keystone.git
cd keystone
cargo build --release
# binary lives at ./target/release/asset-mapper
```

Run the test suite:

```bash
cargo test --workspace
```

## CLI

```text
asset-mapper <COMMAND>

Commands:
  init                 Initialize a new pack folder
  index                Index an existing pack folder
  validate             Validate a pack (sidecar JSON or pack folder)
  bundle               Export a compact LLM-readable context bundle
  resolve              Resolve an assembly plan into a placed scene
  propose-assembly     Auto multi-piece plan from connectors + rules
  analyze              Measure bounds + propose mesh sockets/rules
  vibe-ready           Report pack readiness for vibe builders
  accept-drift         Accept content-hash drift after review
  measure-bounds       Re-measure mesh/image bounds
  migrate              Migrate pack sidecar to current schema version
  export-engine        Export Unreal/Unity/Godot/CSV connector tables
  export-gltf-extras   Write glTF Keystone extras companion JSON
```

### `init` — create a new pack folder

```bash
asset-mapper init ./my-pack --name "My Pack" --license "MIT" --author "Your Studio"
```

Writes a starter pack layout, including the sidecar metadata file, into the target folder.  
**Production requires** `--license` (not `UNSPECIFIED`) and at least one of `--author` / `--source`. When glTF/image sources are readable, real AABB/pixel bounds are stored and `BoundsPlaceholder` is omitted.

### `index` — index an existing pack folder

```bash
asset-mapper index ./my-pack
```

Walks the pack folder and prints a JSON reconciliation report (asset presence, content hashes, sidecar freshness). Preserves manual metadata.

### `accept-drift` — accept hash drift after review

```bash
asset-mapper accept-drift ./my-pack
asset-mapper accept-drift ./my-pack --asset wall
asset-mapper accept-drift ./my-pack --clear-connectors
```

Updates `content_hash` from disk. Keeps connectors by default; use `--clear-connectors` to drop them.

### `measure-bounds` — re-measure geometry

```bash
asset-mapper measure-bounds ./my-pack
```

### `validate` — check a pack for completeness and consistency

```bash
asset-mapper validate ./my-pack
asset-mapper validate ./my-pack/.asset-mapper/pack.assetmap.json
```

### `bundle` — export a compact LLM context bundle

```bash
asset-mapper bundle ./my-pack
```

### `resolve` — resolve an assembly plan into a placed scene

```bash
asset-mapper resolve ./my-pack ./my-plan.json
```

Supports Frame3d (full 3D mating) and Frame2d (XY-plane attachments). Mixed 2D/3D pairs are rejected.

### `migrate` — schema migration

```bash
asset-mapper migrate ./my-pack
asset-mapper migrate ./legacy.assetmap.json
```

### `export-engine` / `export-gltf-extras`

```bash
asset-mapper export-engine ./my-pack --target unity
asset-mapper export-engine ./my-pack --target unreal --output unreal.json
asset-mapper export-engine ./my-pack --target godot
asset-mapper export-engine ./my-pack --target csv
asset-mapper export-gltf-extras ./my-pack --output my-pack.keystone.json
```

See [`docs/superpowers/RELEASE.md`](docs/superpowers/RELEASE.md) for format notes and release packaging.

## Editor (Phase 2)

Desktop authoring UI for the same sidecar model:

```bash
cd crates/asset-mapper-editor
npm install
npm run fixture:phase2   # generates fixtures/phase2 modular .glb if needed
npm run tauri:dev
```

Open or initialize a pack folder, preview `.glb`/`.gltf`, place 3D/2D connectors, edit classes/rules (including `steps_deg`), semantic tags, review flags, measure bounds from mesh, accept hash drift, validate, save `.asset-mapper/pack.assetmap.json`, and export an LLM bundle. Session UX includes dirty confirmation, reload, and discard.

Frontend unit tests and production build:

```bash
cd crates/asset-mapper-editor
npm test
npm run build
```

## Pack format

A pack is a folder containing assets plus an `*.assetmap.json` sidecar that records:

- pack identity, schema version, coordinate convention, default units
- per-asset records: dimensions, bounds, orientation, pivot, content hash
- connector definitions as precise local-space frames (`Frame3d` or `Frame2d`), tagged with a `class`
- connector classes and **class-based** compatibility rules
- semantic tags, affordances, and placement constraints
- production metadata: non-placeholder `license_summary`, provenance (`source` and/or `author`), controlled vocabulary

Working examples:

- [`fixtures/phase0/simple_pack.assetmap.json`](fixtures/phase0/simple_pack.assetmap.json)
- [`fixtures/phase0/llm_style_plan.json`](fixtures/phase0/llm_style_plan.json) (LLM-style assembly plan proof)
- Negative case: [`fixtures/phase0/invalid_pack_unknown_class.assetmap.json`](fixtures/phase0/invalid_pack_unknown_class.assetmap.json)

LLM plan proof docs: [`docs/superpowers/llm-plan-proof.md`](docs/superpowers/llm-plan-proof.md).

## Design contract

- **Metadata is the source of truth.** The editor and downstream SDKs call the resolver — they do not reimplement transform math.
- **Validation is deterministic and reportable.** Diagnostics are structured JSON with a `Severity` (`Error` / `Warning`).
- **Resolver is deterministic.** Given a valid plan, output is reproducible; given an invalid one, it returns a structured `ResolveError`.
- **Compatibility is class-based, not pairwise.** Rules apply to all members of a connector class so packs scale.

The full design rationale is in [`docs/superpowers/specs/2026-06-15-asset-pack-semantic-mapper-design.md`](docs/superpowers/specs/2026-06-15-asset-pack-semantic-mapper-design.md).

## Repository layout

```
crates/
  asset-mapper-core/      schema + validator + resolver + LLM bundle + export/migrate
  asset-mapper-io/        pack folder I/O, bounds, sidecar
  asset-mapper-cli/       `asset-mapper` binary
  asset-mapper-editor/    Tauri desktop editor (frontend + src-tauri)
docs/superpowers/         status, specs, release notes, LLM proof
fixtures/                 phase0/phase2 fixtures
LICENSE-MIT
LICENSE-APACHE
CHANGELOG.md
```

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT license ([LICENSE-MIT](LICENSE-MIT))

at your option.
