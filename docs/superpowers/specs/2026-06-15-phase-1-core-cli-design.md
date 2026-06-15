# Phase 1 Core CLI Design

Date: 2026-06-15

## Summary

Phase 1 turns the Phase 0 proof harness into a usable headless workflow for maintaining asset pack metadata without a GUI. The CLI should be able to initialize a pack folder, index supported asset files, compute content hashes, preserve manual metadata across re-indexing, validate pack health, emit the LLM bundle, and run resolver plans from either a direct sidecar file or a pack folder.

Phase 1 also includes one hardening task from Phase 0: the corridor resolver fixture must prove a real non-overlapping modular attachment. The current proof shows deterministic placement, but it is too weak as a product proof because the fixture resolves both assets to the origin.

## Relationship To The Overall Spec

The overall Asset Mapper design defines Phase 1 as the Core CLI phase:

- index a folder
- compute hashes
- compute or record bounds where possible
- read and write sidecar metadata
- validate a pack
- emit LLM bundle
- run a resolver test plan

The Phase 1 exit criterion is:

> Pack metadata can be maintained and verified without a GUI.

This design keeps that boundary. It does not start editor work, connector placement UI, engine export, world generation, or chat behavior.

## Scope

Included:

- harden the corridor fixture and resolver expectations so valid attachment has meaningful spatial placement
- define a canonical sidecar file name and folder layout
- add a CLI command to initialize a pack folder
- add a CLI command to re-index a pack folder
- compute SHA-256 hashes for discovered assets
- preserve existing metadata when re-indexing
- create placeholder records for newly discovered assets
- detect missing source files and content-hash drift
- read and write canonical sidecar JSON
- keep validation, bundle export, and resolver commands working against sidecar files
- allow validation, bundle export, and resolver commands to accept pack folders
- add focused IO, validation, resolver, and CLI integration tests

Excluded:

- editor UI
- connector placement UI
- broad glTF geometry parsing
- engine, DCC, or format-specific export
- 2D resolver behavior
- LLM or chat workflow
- asset generation or scene generation

## Architecture

Phase 1 should add an IO crate rather than putting folder indexing into the CLI or core model.

Expected workspace shape:

```text
crates/
  asset-mapper-core/
  asset-mapper-io/
  asset-mapper-cli/
```

`asset-mapper-core` remains the graphics-free source of truth for schema types, validation, resolver behavior, and LLM bundle projection. It should not know how to walk folders or choose sidecar paths.

`asset-mapper-io` owns filesystem concerns:

- resolving a direct sidecar path versus a pack folder
- defining the canonical sidecar path
- scanning supported asset files
- computing file hashes
- loading and saving sidecar JSON
- reconciling indexed files against existing metadata

`asset-mapper-cli` stays thin:

- parse arguments
- call IO and core functions
- print structured JSON reports to stdout
- print operational errors to stderr
- return meaningful exit codes

This keeps core behavior testable without filesystem setup, while still giving Phase 1 a practical command-line workflow.

## Canonical Sidecar Layout

The canonical folder sidecar should be:

```text
<pack-folder>/.asset-mapper/pack.assetmap.json
```

Reasons:

- the pack remains portable as one folder
- generated metadata does not mix with source asset files
- the sidecar path is predictable for humans and tools
- later generated reports or caches can live under `.asset-mapper/` without changing the source asset layout

Commands that take a pack input should accept either:

- a direct `.assetmap.json` path
- a folder containing `.asset-mapper/pack.assetmap.json`

Direct file support preserves the Phase 0 fixture workflow. Folder support provides the Phase 1 maintenance workflow.

## CLI Workflow

The normal Phase 1 workflow should be:

```powershell
asset-mapper init <pack-folder> --name "My Pack"
asset-mapper index <pack-folder>
asset-mapper validate <pack-folder>
asset-mapper bundle <pack-folder>
asset-mapper resolve <pack-folder> <plan-json>
```

### `init`

`init` creates `.asset-mapper/pack.assetmap.json`.

It should:

- create the `.asset-mapper/` metadata directory if needed
- fail if the sidecar already exists unless an explicit overwrite flag is introduced later
- scan supported asset files
- compute hashes
- create placeholder asset records
- use default pack-level coordinate convention and units
- write an empty connector class list and compatibility rule list
- write the sidecar using stable pretty JSON

The initial default convention should match the Phase 0 fixture unless the user passes options:

- right-handed
- positive Y up
- positive Z forward
- meters

### `index`

`index` reloads the existing sidecar, scans the folder, recomputes hashes, and writes an updated sidecar.

It should:

- preserve existing records for unchanged files
- preserve manual metadata for changed files
- update observed content hashes only when the workflow explicitly decides to accept the new file state
- create placeholder records for newly discovered files
- report missing source files without silently deleting their records
- report content-hash drift
- keep connector, semantic, affordance, placement, and compatibility metadata intact unless the corresponding asset record is newly created

The first implementation can keep drifted records unchanged and report drift, rather than automatically accepting the new hash. That is safer because connector placement may no longer be valid after an asset changes.

### `validate`

`validate` should work for both sidecar files and pack folders.

When given a folder, it should load `.asset-mapper/pack.assetmap.json` and run core validation plus Phase 1 maintenance validation.

### `bundle`

`bundle` should work for both sidecar files and pack folders. It should keep the Phase 0 rule that the LLM bundle omits raw connector transforms and quaternions.

### `resolve`

`resolve` should work for both sidecar files and pack folders. It should keep returning a resolved scene on success and a concise error on failure.

The Phase 1 resolver regression should prove that the simple corridor fixture produces a non-overlapping placement, not just two asset IDs in the output.

## Supported Asset Discovery

Initial indexing should be deliberately narrow and boring.

Supported extensions:

- `.glb`
- `.gltf`
- `.obj`
- `.fbx`
- `.png`
- `.jpg`
- `.jpeg`
- `.webp`

The scanner should ignore the `.asset-mapper/` directory. It should also avoid indexing hidden metadata files and generated test output.

The indexer should store source paths relative to the pack folder using stable forward slashes. This avoids absolute paths in portable sidecars and keeps Windows path differences from leaking into the canonical metadata.

## Placeholder Metadata

Newly discovered assets need valid records, but Phase 1 should not pretend it knows facts it did not measure.

For newly indexed assets, placeholder fields should be explicit and validation should surface review warnings.

Recommended defaults:

- `asset_id`: stable slug derived from relative path, with collision handling
- `source_path`: relative path from the pack root
- `content_hash`: `sha256:<hex>`
- `display_name`: filename stem converted to readable text
- `asset_type`: inferred from extension where obvious
- `bounds`: placeholder unit bounds
- `dimensions`: placeholder unit dimensions
- `pivot`: `origin`
- `up_axis`: pack default up axis
- `forward_axis`: pack default forward axis
- `semantic_tags`: empty
- `affordances`: empty
- `placement_constraints`: empty
- `connectors`: empty

Placeholder bounds and orientation are acceptable in Phase 1 only if validation reports them clearly as needing author review.

## Validation And Diagnostics

Phase 1 validation should extend the current core validator with pack maintenance diagnostics.

New diagnostics should cover:

- sidecar source paths that no longer exist
- indexed files missing from metadata
- content-hash drift
- placeholder bounds that need author review
- placeholder orientation or pivot values that need author review
- duplicate source paths
- non-finite bounds
- non-finite dimensions
- non-finite quaternions
- non-finite or negative snap tolerances

Hash drift should be a warning, not an error. An asset can legitimately change, but its metadata should be reviewed before accepting the new hash.

Missing source files should also start as warnings for `validate` because the user may be cleaning up a pack. A command that needs a missing asset for a concrete operation may still fail.

Diagnostics should remain structured and machine-readable. Human-readable messages are useful, but stable codes matter more for future editor and CI usage.

## Error Handling

Report-style command output should go to stdout as JSON.

Examples:

- validation reports
- indexing reconciliation summaries
- resolved scenes
- LLM bundles

Operational failures should go to stderr and return nonzero:

- unreadable input
- missing sidecar
- invalid JSON
- unsupported command arguments
- inability to write the sidecar
- resolver failure

The CLI should avoid printing partial success-shaped JSON on resolver or IO failure.

## Testing Strategy

Phase 1 needs coverage at three levels.

Core tests:

- preserve existing schema, validation, resolver, bundle, and hash tests
- add a resolver regression proving the corridor fixture resolves to a non-overlapping placement

IO tests:

- canonical sidecar path resolution
- direct sidecar path support
- pack-folder sidecar support
- supported asset scanning
- `.asset-mapper/` exclusion
- relative source path normalization
- hash computation during indexing
- reconciliation for unchanged, changed, new, and missing files

CLI tests:

- `init` creates the sidecar
- `index` preserves manual metadata
- `index` reports new, missing, and drifted files
- `validate` accepts a folder input
- `bundle` accepts a folder input
- `resolve` accepts a folder input
- invalid inputs fail with stderr, not partial stdout JSON

The full Phase 1 verification gate should include:

```powershell
cargo fmt -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test
cargo run -p asset-mapper-cli -- init <fixture-pack-copy> --name "Fixture Pack"
cargo run -p asset-mapper-cli -- index <fixture-pack-copy>
cargo run -p asset-mapper-cli -- validate <fixture-pack-copy>
cargo run -p asset-mapper-cli -- bundle <fixture-pack-copy>
cargo run -p asset-mapper-cli -- resolve <fixture-pack-copy> <plan-json>
```

## Implementation Sequence

1. Harden the Phase 0 corridor fixture so `resolve` proves a non-overlapping attachment.
2. Add `asset-mapper-io` for sidecar path resolution, folder scanning, hashing, and read/write helpers.
3. Define `.asset-mapper/pack.assetmap.json` as the canonical folder sidecar.
4. Add `init` to create a sidecar from a folder.
5. Add `index` to reconcile folder contents with existing metadata.
6. Update `validate`, `bundle`, and `resolve` to accept either a sidecar file or pack folder.
7. Expand validation diagnostics for maintenance workflows.
8. Run full CLI and core verification.

## Exit Criteria

Phase 1 is complete when a user can point the CLI at a folder and:

- generate canonical sidecar metadata
- re-index after file changes
- see new-file, missing-file, hash-drift, and placeholder diagnostics
- validate the pack
- emit an LLM bundle
- run resolver plans
- keep manually authored metadata across indexing runs

The user should no longer need to hand-author the entire JSON sidecar from scratch, but connector placement and semantic labeling can still be manual JSON edits until the editor phase.

## Risks

### Placeholder Metadata Can Hide Missing Authoring Work

The indexer must not make placeholder bounds or orientation look authoritative. Validation warnings are required so the user knows what still needs review.

### Hash Drift Can Invalidate Connectors

Changing a mesh can invalidate connector positions without breaking JSON shape. Phase 1 should report drift and preserve old metadata until the user explicitly reviews the asset.

### Import Scope Can Expand Too Quickly

Phase 1 should not become a full geometry importer. Basic file discovery and hashes are enough for the exit criterion. Real bounds extraction can be added incrementally once the pack maintenance workflow is reliable.

### CLI Output Stability Matters

The editor and future automation will likely consume CLI JSON. Diagnostic codes and report shapes should be treated as intentional contracts once introduced.
