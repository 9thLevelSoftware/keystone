# LLM-style assembly plan proof (P0-07)

This documents that a model-style assembly plan — the same shape an LLM would emit from an `LlmBundle` — is accepted by `resolve_plan` against the Phase 0 corridor fixture, and that invalid variants are rejected.

## Fixtures

| File | Purpose |
| --- | --- |
| `fixtures/phase0/simple_pack.assetmap.json` | Valid pack with two corridor segments and `corridor_end` connectors |
| `fixtures/phase0/llm_style_plan.json` | Valid plan attaching `corridor_b.back` → `corridor_a.front` at 0° |
| `fixtures/phase0/llm_style_plan_invalid_class.json` | Same attachment but `rotation_choice_deg: 90` under a locked rule |

## Automated proof

```powershell
cargo test -p asset-mapper-core --test llm_plan_proof
```

The test:

1. Loads the pack fixture and the valid LLM-style plan.
2. Asserts `resolve_plan` succeeds and places `corridor_b` at `z ≈ 2`.
3. Asserts the invalid rotation-choice plan fails with `RotationChoiceNotAllowed`.
4. Asserts a plan referencing an unknown asset fails with `UnknownPlacedAsset`.

## Manual CLI reproduction

```powershell
cargo run -p asset-mapper-cli -- resolve `
  fixtures/phase0/simple_pack.assetmap.json `
  fixtures/phase0/llm_style_plan.json

cargo run -p asset-mapper-cli -- resolve `
  fixtures/phase0/simple_pack.assetmap.json `
  fixtures/phase0/llm_style_plan_invalid_class.json
# expect non-zero exit and structured ResolveError
```

## Bundle → plan contract

An LLM is expected to:

1. Consume `asset-mapper bundle <pack>` (`LlmBundle`: assets, connectors by class, dimensions — no raw transforms).
2. Emit an `AssemblyPlan` with `root_asset_id` and ordered `operations` using connector ids/classes from the bundle.
3. Rely on Keystone to validate geometry via `resolve`.
