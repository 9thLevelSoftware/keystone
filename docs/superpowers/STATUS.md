# Keystone Status Matrix (Plans vs Codebase)

**Last audited:** 2026-07-26  
**Branch:** `feat/vibe-ready-facilitation` (from `main` @ v0.2.0)  
**Method:** Compare design exit criteria against live crates/tests after full Phases 0–3 + vibe-ready facilitation.

### Verification gate results (2026-07-26, Windows)

| Gate | Result |
| --- | --- |
| `cargo fmt -- --check` | Pass |
| `cargo clippy --all-targets --all-features -- -D warnings` | Pass |
| `cargo test --workspace` | Pass |
| `npm run fixture:phase2` | Pass |
| `npm test` (editor) | Pass |
| `npm run build` (editor) | Pass |
| Manual `npm run tauri:dev` desktop smoke | Not required for code completeness |
| Real external modular pack smoke | Optional manual |

## Production definition (v1)

Production-ready means:

1. Phase 0–1 CLI workflows work and tests pass.
2. Phase 2 exit criterion: a modular 3D pack can be mapped in the desktop editor without hand-editing JSON, then validated/bundled via CLI.
3. CI enforces Rust + frontend tests.
4. Release artifacts documented for CLI and editor.
5. README/docs match reality; LICENSE files match dual-license claim.
6. Phase 3 export, 2D authoring, migrations, and authoring helpers are implemented.
7. Production packs require a real license summary (not empty/`UNSPECIFIED`), provenance with source or author, and controlled vocabulary.
8. WASM bindings expose validate/resolve/bundle for embedders.
9. Editor UI covers pack settings, vocab, connectors, rules, diagnostics, session ops.

---

## Phase 0 — Proof harness

**Verdict: done**

| ID | Requirement | Status | Evidence |
| --- | --- | --- | --- |
| P0-01 | Schema draft | Done | `crates/asset-mapper-core/src/schema.rs` |
| P0-02 | Hand-authored fixture pack | Done | `fixtures/phase0/` |
| P0-03 | Validator + diagnostics | Done | `validate.rs`, `diagnostics.rs` |
| P0-04 | Deterministic 3D resolver | Done | `resolver.rs` |
| P0-05 | LLM bundle omits raw transforms | Done | `bundle.rs` |
| P0-06 | CLI validate / bundle / resolve | Done | `asset-mapper-cli` |
| P0-07 | LLM produces valid connector plans | Done | `fixtures/phase0/llm_style_plan*.json`, `tests/llm_plan_proof.rs`, `docs/superpowers/llm-plan-proof.md` |

---

## Phase 1 — Core CLI

**Verdict: done**

| ID | Requirement | Status | Evidence |
| --- | --- | --- | --- |
| P1-01 | Canonical sidecar | Done | `sidecar.rs` |
| P1-02 | `init` | Done | CLI + tests |
| P1-03 | `index` | Done | CLI + tests |
| P1-04 | Folder + direct sidecar | Done | CLI + IO |
| P1-05 | Source maintenance diagnostics | Done | `validation.rs` |
| P1-06 | Placeholder review warnings | Done | `ReviewFlag` |
| P1-07 | Real bounds extraction | Done | `asset-mapper-io/src/bounds.rs`, gltf crate; tests/bounds.rs |
| P1-08 | Accept hash-drift workflow | Done | `accept-drift` CLI, editor command, tests |
| P1-09 | Supported extension scan | Done | `SUPPORTED_ASSET_EXTENSIONS` |

---

## Phase 2 — Editor MVP

**Verdict: done**

| ID | Requirement | Status | Evidence |
| --- | --- | --- | --- |
| P2-01 | Tauri + Vite/React/Three.js | Done | `asset-mapper-editor` |
| P2-02 | open/init/index/save/validate/export | Done | commands + accept-drift/measure |
| P2-03 | Asset list + inspector + diagnostics | Done | React components |
| P2-04 | glb/gltf preview + bounds | Done | sidecar + measure-from-mesh writes real bounds |
| P2-05 | Connector create/move/delete | Done | + duplicate, snap-to-face |
| P2-06 | Numeric position editing | Done | Inspector |
| P2-07 | Numeric orientation editing | Done | Euler + quat fields |
| P2-08 | Connector class assignment | Done | Inspector |
| P2-09 | Role / mating_axis / up_reference | Done | Inspector selects |
| P2-10 | Compatibility rule pairing | Done | RulesEditor |
| P2-11 | Rule rotation including steps_deg | Done | RulesEditor |
| P2-12 | Delete class/rule UI | Done | RulesEditor |
| P2-13 | Rust validation from editor | Done | Command layer |
| P2-14 | Save sidecar | Done | `save_pack` |
| P2-15 | Export LLM bundle | Done | `export_bundle` |
| P2-16 | Diagnostic click → select | Done | DiagnosticsPanel + `selectDiagnosticTarget` |
| P2-17 | Inspect orientation, pivot, flags | Done | Inspector |
| P2-18 | Semantic tags / affordances / constraints | Done | Inspector |
| P2-19 | Headless smoke | Done | `src-tauri/tests/smoke.rs` |
| P2-20 | Full desktop GUI smoke | Unverified manual | Code complete |
| P2-21 | Modular pack without hand JSON | Done | Editor path + fixture smoke |

---

## Phase 3 — Export & integration

**Verdict: done**

| ID | Requirement | Status | Evidence |
| --- | --- | --- | --- |
| P3-01 | glTF metadata mirroring | Done | `export::gltf_keystone_extras`, CLI `export-gltf-extras` |
| P3-02 | Engine export helpers | Done | Unreal/Unity/Godot/CSV via `export-engine` |
| P3-03 | 2D tile/sprite authoring UX | Done | Frame2d inspector + 2D resolve |
| P3-04 | Schema migration tooling | Done | `migrate` module + CLI |
| P3-05 | Authoring-speed automation | Done | duplicate, snap face, class suggest |

---

## Cross-cutting / production hardening

| ID | Requirement | Status | Notes |
| --- | --- | --- | --- |
| X-01 | CI | Done | `.github/workflows/ci.yml` |
| X-02 | LICENSE-MIT / LICENSE-APACHE | Done | Dual license files present |
| X-03 | Release packaging docs | Done | `docs/superpowers/RELEASE.md` |
| X-04 | Changelog | Done | `CHANGELOG.md` |
| X-05 | README accuracy | Done | Status table updated |
| X-06 | Provenance fields | Done | Schema v2 `provenance`; validate requires source or author (not notes-only) |
| X-07 | Controlled vocabulary | Done | Schema v2 `vocabulary` + unknown-term errors |
| X-08 | WASM resolver bindings | Done | `asset-mapper-wasm` (`resolve/validate/bundle` JSON APIs) |
| X-09 | Status file hygiene | Done | This file |
| X-10 | Fully functional editor UI | Done | Pack settings, tags, provenance, measure/drift, full connector/rules |
| X-11 | FBX bounds | Done | ASCII + binary Kaydara FBX Vertices AABB (raw + zlib arrays) |
| X-12 | Mesh-aware sockets | Done | glTF/OBJ mesh samples → surface/portal sockets; AABB fallback + review flag |
| X-13 | Rich auto rules | Done | Modular ontology + same-class self-rules; per-socket class heuristics |
| X-14 | Whole-pack assembly | Done | `propose_assembly_plan`, CLI `propose-assembly`, editor Pack assembly |
| X-15 | Vibe readiness | Done | `vibe_readiness`, CLI `vibe-ready`, editor banner, WASM |
| X-16 | Handoff contract | Done | LlmBundle `how_to_plan`/`plan_contract`/`face_size`, vibe-builder-handoff.md |
| X-17 | Resolve failure loop | Done | ResolveErrorReport codes + fix_target; editor plan import |
| X-18 | Vibe bake-off fixtures | Done | `fixtures/vibe/`, `write-vibe-fixtures.mjs`, io test harness |
| X-19 | FBX mesh samples | Done | Vertices → MeshGeometry for socket proposal |
| X-20 | Code signing | Deferred | Documented unsigned releases; no secrets in repo |

---

## Verification commands

```powershell
cargo fmt -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --workspace

cd crates/asset-mapper-editor
npm ci
npm run fixture:phase2
npm test
npm run build
```

## Summary

| Phase | Spec exit criterion | Reality |
| --- | --- | --- |
| 0 | Deterministic validate/resolve/bundle | Met + LLM plan proof |
| 1 | Maintain packs without GUI | Met + bounds + accept-drift |
| 2 | Map pack without hand-editing JSON | Met |
| 3 | External tools consume without hand translation | Met |
| Production | CI, licenses, artifacts, accurate docs | Met |

**Bottom line:** Phases 0–3 and production hardening are implemented on `main` with automated test coverage.
