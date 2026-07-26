# Changelog

All notable changes to Keystone (Asset Mapper) are documented in this file.

## [Unreleased]

### Vibe-ready facilitation

- **Vibe readiness:** `vibe_readiness(pack)` + CLI `asset-mapper vibe-ready` + editor readiness banner (coverage, orphan/dead-end classes, connectivity, score 0–100)
- **Auto-map quality:** stronger portal/occupancy scoring, junk face filters, lower socket caps, optional `ConnectorRecord.face_size` (no schema version bump), conservative plug/receptacle inset roles
- **Semantics:** analyze suggests vocabulary-gated semantic tags / affordances / placement constraints from name + class + shape
- **LLM handoff:** bundle includes `face_size`, `how_to_plan`, `plan_contract`; docs in `docs/user-guide/vibe-builder-handoff.md`
- **Resolve feedback:** stable error codes + `fix_pack` / `fix_plan` guidance; CLI stderr JSON; editor plan import + highlight on failure
- **FBX sockets:** load mesh samples from FBX `Vertices` (ASCII/binary) for proposals; glTF still preferred
- **Fixtures / bake-off:** `scripts/write-vibe-fixtures.mjs`, `fixtures/vibe/`, harness test + `docs/user-guide/bake-off.md`
- **Tile reuse:** documented unique-asset resolve limit; `propose-assembly --allow-asset-reuse` / `--max-instances-per-asset` emit guidance notes only
- **WASM:** `vibe_ready_json`; resolve errors return JSON reports

## [Unreleased]

### MegaKit auto-map tune

- **Geometry-first classes** (AABB shape family + strong portal openings); filenames optional soft boost only
- Portal class promotion only for **strong** portals with solid occupancy (stops doorway monopoly on sci-fi walls)
- Soft path/name hints when they agree with shape (not required for wall/floor/door)
- Analyze `--exclude-glob` / skip images by default; modular kit recipe for large packs
- Vibe-ready: fail on class monopoly; class diversity checklist
- Assembly root prefers full wall straights; structural mate scoring
- Bake-off harness: `scripts/bakeoff-megakit.ps1` + docs

## [0.2.0] — 2026-07-26

Honest-limits completion: load → auto-map → tweak → see the pack connect.

### Product vision

- **Mesh-aware sockets:** Analyze loads glTF/OBJ mesh samples and proposes mating frames on surface centroids / portal openings; AABB face centers remain fallback (review flag `auto_from_bounds_fallback`)
- **Richer auto rules:** Modular ontology (doorway↔wall_edge, corridor_end, floor_edge, archway, …) plus same-class self-rules; per-socket class heuristics; scale-based snap tolerances
- **Whole-pack assembly:** `propose_assembly_plan` greedy multi-piece synthesizer; CLI `propose-assembly`; editor **Pack assembly → Auto layout pack** (still supports two-piece mate)
- Analyze report fields: `mesh_socket_assets`, `bounds_fallback_assets`
- CLI: `analyze --aabb-only`; `propose-assembly --max-pieces --root -o`
- Working 3D previews (blob load) and two-piece mate from v0.1.1 lineage
- Docs: getting-started default path is Analyze + pack assembly; edge cases only (not open product gaps)

## [0.1.1] — 2026-07-26

CLI zip + editor installers; release workflow fixes after v0.1.0.

## [0.1.0] — 2026-07-26

First production-complete cut covering Phases 0–3.

### Phase 0 — Core proof harness
- Canonical pack schema, validator, LLM bundle, deterministic 3D resolver
- Fixture packs and plans under `fixtures/phase0/`
- Recorded LLM-style plan proof (`docs/superpowers/llm-plan-proof.md`, `tests/llm_plan_proof.rs`)

### Phase 1 — CLI
- Pack folder workflow: `init`, `index`, `validate`, `bundle`, `resolve`
- Canonical sidecar `.asset-mapper/pack.assetmap.json`
- Real bounds extraction from glTF/GLB, OBJ, and common image formats
- `accept-drift` to accept content-hash drift after review
- `measure-bounds` to re-measure and clear `BoundsPlaceholder`

### Phase 2 — Desktop editor
- Tauri + React + Three.js authoring UI on `main`
- Connector role / mating_axis / up_reference, numeric orientation (Euler + quat)
- Rule rotation policies including `steps_deg`
- Delete class/rule, semantic tags / affordances / placement constraints
- Review flag management, diagnostic click-to-select
- Session UX: dirty confirm, reload, discard
- Measure-from-mesh and accept-drift actions

### Phase 3 — Export & integration
- Schema migration (`migrate` CLI; v0 → v1 framework)
- glTF Keystone extras companion export (`export-gltf-extras`)
- Engine export helpers: Unreal / Unity / Godot JSON + connectors CSV
- Frame2d resolve path (2D attachments on the XY plane)
- 2D connector authoring in the editor
- Authoring helpers: duplicate connector, snap to bounds faces, class-from-name suggestions

### Production
- Schema v2 production gates: non-placeholder `license_summary` (rejects empty/`UNSPECIFIED`), provenance requiring source or author, controlled vocabulary
- Full editor pack settings UI (license, provenance, vocabulary)
- WASM bindings: `validate_pack_json` / `resolve_plan_json` / `bundle_pack_json`
- FBX bounds: ASCII + binary Kaydara Vertices AABB (raw/zlib, f/d arrays, v7400 + v7500 headers; array/depth caps)
- Dual license files: `LICENSE-MIT`, `LICENSE-APACHE`
- GitHub Actions CI (fmt, clippy, cargo test, npm test/build)
- Release documentation (`docs/superpowers/RELEASE.md`)
