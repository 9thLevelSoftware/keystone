//! WebAssembly bindings for Keystone core operations.
//!
//! These APIs take and return JSON strings so JS/TS hosts do not need a
//! second schema implementation. Geometry math stays in Rust.

use asset_mapper_core::{
    AssemblyPlan, LlmBundle, PackRecord, resolve_plan, validate_pack, vibe_readiness,
};
use wasm_bindgen::prelude::*;

fn parse_pack(pack_json: &str) -> Result<PackRecord, JsValue> {
    serde_json::from_str(pack_json).map_err(|error| JsValue::from_str(&error.to_string()))
}

fn parse_plan(plan_json: &str) -> Result<AssemblyPlan, JsValue> {
    serde_json::from_str(plan_json).map_err(|error| JsValue::from_str(&error.to_string()))
}

/// Validate a pack JSON document; returns a [`ValidationReport`] JSON string.
#[wasm_bindgen]
pub fn validate_pack_json(pack_json: &str) -> Result<String, JsValue> {
    let pack = parse_pack(pack_json)?;
    let report = validate_pack(&pack);
    serde_json::to_string(&report).map_err(|error| JsValue::from_str(&error.to_string()))
}

/// Resolve an assembly plan against a pack; returns a [`ResolvedScene`] JSON string.
///
/// On failure, returns a `JsValue` error string containing a JSON
/// [`ResolveErrorReport`] (`code`, `fix_target`, `guidance`, …).
#[wasm_bindgen]
pub fn resolve_plan_json(pack_json: &str, plan_json: &str) -> Result<String, JsValue> {
    let pack = parse_pack(pack_json)?;
    let plan = parse_plan(plan_json)?;
    match resolve_plan(&pack, &plan) {
        Ok(scene) => {
            serde_json::to_string(&scene).map_err(|error| JsValue::from_str(&error.to_string()))
        }
        Err(error) => {
            let report = error.to_report();
            let body = serde_json::to_string(&report).unwrap_or_else(|_| {
                format!(r#"{{"code":"{}","message":"{}"}}"#, error.code(), error)
            });
            Err(JsValue::from_str(&body))
        }
    }
}

/// Build an LLM bundle from a pack JSON document.
#[wasm_bindgen]
pub fn bundle_pack_json(pack_json: &str) -> Result<String, JsValue> {
    let pack = parse_pack(pack_json)?;
    let bundle = LlmBundle::from_pack(&pack);
    serde_json::to_string(&bundle).map_err(|error| JsValue::from_str(&error.to_string()))
}

/// Vibe-builder readiness report for a pack JSON document.
#[wasm_bindgen]
pub fn vibe_ready_json(pack_json: &str) -> Result<String, JsValue> {
    let pack = parse_pack(pack_json)?;
    let report = vibe_readiness(&pack);
    serde_json::to_string(&report).map_err(|error| JsValue::from_str(&error.to_string()))
}

/// Current schema version supported by this build.
#[wasm_bindgen]
pub fn current_schema_version() -> u32 {
    asset_mapper_core::CURRENT_SCHEMA_VERSION
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_round_trip_via_json() {
        let pack = include_str!("../../../fixtures/phase0/simple_pack.assetmap.json");
        let plan = include_str!("../../../fixtures/phase0/llm_style_plan.json");
        let scene = resolve_plan_json(pack, plan).expect("resolves");
        assert!(scene.contains("corridor_b"));
        assert!(scene.contains("translation"));
    }

    #[test]
    fn validate_returns_report() {
        let pack = include_str!("../../../fixtures/phase0/simple_pack.assetmap.json");
        let report = validate_pack_json(pack).expect("validates");
        assert!(report.contains("diagnostics"));
    }

    #[test]
    fn bundle_includes_license() {
        let pack = include_str!("../../../fixtures/phase0/simple_pack.assetmap.json");
        let bundle = bundle_pack_json(pack).expect("bundles");
        assert!(bundle.contains("license_summary"));
        assert!(bundle.contains("vocabulary"));
        assert!(bundle.contains("how_to_plan"));
        assert!(bundle.contains("plan_contract"));
    }

    #[test]
    fn vibe_ready_returns_score() {
        let pack = include_str!("../../../fixtures/phase0/simple_pack.assetmap.json");
        let report = vibe_ready_json(pack).expect("vibe");
        assert!(report.contains("\"score\""));
        assert!(report.contains("checklist"));
    }
}
