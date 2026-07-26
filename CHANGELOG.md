# Changelog

All notable changes to Keystone (Asset Mapper) are documented in this file.

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
