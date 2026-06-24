# Keystone

Headless Rust tooling for **semantic mapping of prebuilt 2D and 3D asset packs**.

Keystone is the canonical metadata layer for jigsaw-style asset packs: it captures each asset's dimensions, orientation, pivot, connector locations, connector classes, and valid class-to-class compatibility rules, and exposes that metadata through a deterministic validator and a connector resolver. It does not generate assets, build worlds, or host a chat experience — its job is to make downstream assembly (by an LLM or another tool) geometrically trustworthy.

> The LLM chooses which pieces and connectors should attach. Keystone's metadata plus resolver logic makes that choice geometrically valid or rejects it with a structured reason.

## Status

**Pre-release / Phase 0–1 CLI.** The `asset-mapper` binary is functional and ships with `init`, `index`, `validate`, `bundle`, and `resolve` subcommands. An interactive editor (Phase 2) is in progress on the `codex/phase-2-editor-mvp` branch but is **not** on `main` yet.

| Phase | Scope | Status |
| --- | --- | --- |
| 0 | Core schema, validator, LLM bundle, connector resolver | Shipped on `main` |
| 1 | Headless CLI for pack folder workflow | Shipped on `main` |
| 2 | Interactive editor MVP | Branched (`codex/phase-2-editor-mvp`) |

## Workspace

Cargo workspace, edition 2024, MSRV `1.85`, dual-licensed MIT OR Apache-2.0:

- **`asset-mapper-core`** — canonical schema (`Pack`, `Asset`, `Connector`, classes, compatibility rules), `validate_pack`, content hashing, the `LlmBundle` exporter, and the deterministic `resolve_plan` connector resolver. No I/O dependencies.
- **`asset-mapper-io`** — pack folder indexing, sidecar `*.assetmap.json` read/write, pack source validation.
- **`asset-mapper-cli`** — the `asset-mapper` binary, built on `clap` with derive subcommands.

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
  init      Initialize a new pack folder
  index     Index an existing pack folder
  validate  Validate a pack (sidecar JSON or pack folder)
  bundle    Export a compact LLM-readable context bundle
  resolve   Resolve an assembly plan into a placed scene
```

### `init` — create a new pack folder

```bash
asset-mapper init --name "My Pack" ./my-pack
```

Writes a starter pack layout, including the sidecar metadata file, into the target folder.

### `index` — index an existing pack folder

```bash
asset-mapper index ./my-pack
```

Walks the pack folder and prints a JSON reconciliation report (asset presence, content hashes, sidecar freshness).

### `validate` — check a pack for completeness and consistency

```bash
asset-mapper validate ./my-pack
# or against a sidecar file directly:
asset-mapper validate ./my-pack/pack.assetmap.json
```

Detects: missing connector classes, classes with no valid rule, degenerate connector frames, missing bounds or coordinate convention, non-normalized 3D orientation, contradictory orientation metadata, content-hash drift, and invalid compatibility rules. Exits non-zero when any error-severity diagnostic is reported. JSON output.

### `bundle` — export a compact LLM context bundle

```bash
asset-mapper bundle ./my-pack
```

Emits a `LlmBundle` JSON document summarizing each asset's dimensions, semantic tags, affordances, placement constraints, and connectors. Designed to be the "jigsaw puzzle view" of the pack that a thinking model can consume to plan an assembly.

### `resolve` — resolve an assembly plan into a placed scene

```bash
asset-mapper resolve --plan ./my-plan.json ./my-pack
```

Given an `AssemblyPlan` describing which connectors should attach to which, computes the required transforms deterministically. Fails with a structured `ResolveError` (e.g. incompatible classes, conflicting constraints) rather than producing a silently-wrong placement.

## Pack format

A pack is a folder containing assets plus an `*.assetmap.json` sidecar that records:

- pack identity, schema version, coordinate convention, default units
- per-asset records: dimensions, bounds, orientation, pivot, content hash
- connector definitions as precise local-space frames, tagged with a `class`
- connector classes and **class-based** compatibility rules (e.g. `wall_edge` ↔ `wall_edge`, `doorway` ↔ `door_frame`)
- semantic tags, affordances, and placement constraints
- optional provenance / license summary

A working example lives in [`fixtures/phase0/simple_pack.assetmap.json`](fixtures/phase0/simple_pack.assetmap.json). A negative test case (unknown connector class) is at [`fixtures/phase0/invalid_pack_unknown_class.assetmap.json`](fixtures/phase0/invalid_pack_unknown_class.assetmap.json).

## Design contract

- **Metadata is the source of truth.** The editor and downstream SDKs call the resolver — they do not reimplement transform math.
- **Validation is deterministic and reportable.** Diagnostics are structured JSON with a `Severity` (`Error` / `Warning` / `Info`).
- **Resolver is deterministic.** Given a valid plan, output is reproducible across platforms; given an invalid one, it returns a structured `ResolveError`.
- **Compatibility is class-based, not pairwise.** Rules apply to all members of a connector class so packs scale.

The full design rationale is in [`docs/superpowers/specs/2026-06-15-asset-pack-semantic-mapper-design.md`](docs/superpowers/specs/2026-06-15-asset-pack-semantic-mapper-design.md).

## Repository layout

```
crates/
  asset-mapper-core/      schema + validator + resolver + LLM bundle
  asset-mapper-io/        pack folder I/O and sidecar read/write
  asset-mapper-cli/       `asset-mapper` binary
docs/superpowers/
  specs/                  design specs
  plans/                  phased implementation plans
fixtures/phase0/          example packs and assembly plans used in tests
```

## License

Dual-licensed under either of:

- [MIT License](https://opensource.org/licenses/MIT)
- [Apache License, Version 2.0](https://www.apache.org/licenses/LICENSE-2.0)

at your option.
