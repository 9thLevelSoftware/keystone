//! Bake-off: procedural vibe fixtures → analyze → propose_assembly → resolve.
//!
//! Proves multi-piece connectivity on real-kit-ish glTF geometry.

use std::path::{Path, PathBuf};
use std::process::Command;

use asset_mapper_core::{
    ProposeAssemblyOptions, propose_assembly_plan, resolve_plan, validate_pack, vibe_readiness,
};
use asset_mapper_io::{
    InitPackOptions, analyze_pack_folder, init_pack_folder, measure_pack_bounds,
    read_pack_from_input,
};

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn ensure_vibe_fixtures() {
    let fixture = repo_root().join("fixtures/vibe/modular_kit/wall_box.glb");
    if fixture.is_file() {
        return;
    }
    let script = repo_root().join("scripts/write-vibe-fixtures.mjs");
    let status = Command::new("node")
        .arg(&script)
        .current_dir(repo_root())
        .status()
        .expect("node available to generate vibe fixtures");
    assert!(status.success(), "write-vibe-fixtures.mjs failed");
    assert!(
        fixture.is_file(),
        "fixture not written: {}",
        fixture.display()
    );
}

fn copy_glbs(src: &Path, dst: &Path) {
    std::fs::create_dir_all(dst).expect("dst");
    for entry in std::fs::read_dir(src).expect("src") {
        let entry = entry.expect("entry");
        if !entry.file_type().expect("ty").is_file() {
            continue;
        }
        let name = entry.file_name();
        let name_str = name.to_string_lossy();
        if name_str.ends_with(".glb") || name_str.ends_with(".gltf") {
            std::fs::copy(entry.path(), dst.join(&name)).expect("copy");
        }
    }
}

#[test]
fn vibe_kit_analyze_assemble_resolve() {
    ensure_vibe_fixtures();
    let temp = tempfile::tempdir().expect("temp");
    let pack_root = temp.path().join("kit");
    copy_glbs(&repo_root().join("fixtures/vibe/modular_kit"), &pack_root);

    init_pack_folder(
        &pack_root,
        InitPackOptions {
            display_name: "Vibe Bakeoff Kit".to_owned(),
            license_summary: "MIT".to_owned(),
            author: Some("Keystone Tests".to_owned()),
            source: Some("fixtures/vibe".to_owned()),
        },
    )
    .expect("init");

    measure_pack_bounds(&pack_root).expect("measure");
    let report = analyze_pack_folder(
        &pack_root,
        asset_mapper_core::AnalyzeOptions {
            replace_existing_connectors: true,
            ..Default::default()
        },
    )
    .expect("analyze");

    assert!(
        report.connectors_added >= 4,
        "expected several connectors, got {}",
        report.connectors_added
    );
    assert!(
        report.mesh_socket_assets + report.bounds_fallback_assets >= 2,
        "expected mesh or bounds sockets on assets"
    );

    let loaded = read_pack_from_input(&pack_root).expect("read pack");
    let validation = validate_pack(&loaded.pack);
    assert!(
        validation.is_valid(),
        "pack invalid: {:?}",
        validation.diagnostics
    );

    let readiness = vibe_readiness(&loaded.pack);
    assert!(
        readiness.coverage > 0.5,
        "coverage too low: {}",
        readiness.coverage
    );
    assert!(
        readiness.orphan_classes.is_empty(),
        "orphan classes: {:?}",
        readiness.orphan_classes
    );

    let assembly = propose_assembly_plan(
        &loaded.pack,
        &ProposeAssemblyOptions {
            max_pieces: 5,
            ..Default::default()
        },
    );
    assert!(
        !assembly.plan.root_asset_id.is_empty(),
        "expected a root: {:?}",
        assembly.notes
    );
    assert!(
        assembly.placed_asset_ids.len() >= 2,
        "expected multi-piece connectivity, placed={:?} notes={:?}",
        assembly.placed_asset_ids,
        assembly.notes
    );

    let scene = resolve_plan(&loaded.pack, &assembly.plan).expect("resolve assembly plan");
    assert_eq!(scene.placements.len(), assembly.placed_asset_ids.len());
}
