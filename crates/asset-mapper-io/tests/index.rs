use asset_mapper_core::{AssetType, ReviewFlag};
use asset_mapper_io::{
    canonical_sidecar_path, index_pack_folder, init_pack_folder, read_pack_from_input, scan_assets,
};

#[test]
fn scan_assets_ignores_metadata_directory_and_normalizes_paths() {
    let temp = tempfile::tempdir().expect("temp dir is created");
    std::fs::create_dir_all(temp.path().join("models")).expect("models dir is created");
    std::fs::create_dir_all(temp.path().join(".asset-mapper")).expect("metadata dir is created");
    std::fs::write(temp.path().join("models").join("Wall A.glb"), b"wall")
        .expect("asset is written");
    std::fs::write(
        temp.path().join(".asset-mapper").join("ignored.glb"),
        b"ignored",
    )
    .expect("metadata asset is written");
    std::fs::write(temp.path().join("notes.txt"), b"notes").expect("notes are written");

    let indexed = scan_assets(temp.path()).expect("scan succeeds");

    assert_eq!(indexed.len(), 1);
    assert_eq!(indexed[0].source_path, "models/Wall A.glb");
    assert_eq!(indexed[0].asset_type, AssetType::Model3d);
    assert!(indexed[0].content_hash.starts_with("sha256:"));
}

#[test]
fn init_pack_folder_creates_sidecar_with_placeholder_records() {
    let temp = tempfile::tempdir().expect("temp dir is created");
    std::fs::write(temp.path().join("wall.glb"), b"wall").expect("asset is written");

    let report = init_pack_folder(temp.path(), "Dungeon Kit".to_owned()).expect("init succeeds");
    let sidecar_path = canonical_sidecar_path(temp.path());

    assert_eq!(
        report.sidecar_path,
        sidecar_path.to_string_lossy().into_owned()
    );
    assert_eq!(report.new_assets, vec!["wall.glb"]);
    assert!(sidecar_path.is_file());

    let loaded = read_pack_from_input(temp.path()).expect("sidecar reloads");
    assert_eq!(loaded.pack.pack_id, "dungeon_kit");
    assert_eq!(loaded.pack.display_name, "Dungeon Kit");
    assert_eq!(loaded.pack.assets.len(), 1);
    assert_eq!(loaded.pack.assets[0].asset_id, "wall");
    assert_eq!(loaded.pack.assets[0].source_path, "wall.glb");
    assert_eq!(loaded.pack.assets[0].asset_type, AssetType::Model3d);
    assert!(loaded.pack.assets[0].content_hash.starts_with("sha256:"));
    assert!(
        loaded.pack.assets[0]
            .review_flags
            .contains(&ReviewFlag::BoundsPlaceholder)
    );
    assert!(
        loaded.pack.assets[0]
            .review_flags
            .contains(&ReviewFlag::OrientationPlaceholder)
    );
    assert!(
        loaded.pack.assets[0]
            .review_flags
            .contains(&ReviewFlag::PivotPlaceholder)
    );
}

#[test]
fn index_preserves_manual_metadata_and_reports_changes() {
    let temp = tempfile::tempdir().expect("temp dir is created");
    std::fs::write(temp.path().join("wall.glb"), b"wall-v1").expect("wall is written");
    std::fs::write(temp.path().join("floor.glb"), b"floor-v1").expect("floor is written");
    init_pack_folder(temp.path(), "Dungeon Kit".to_owned()).expect("init succeeds");

    let mut loaded = read_pack_from_input(temp.path()).expect("sidecar reloads");
    loaded
        .pack
        .assets
        .iter_mut()
        .find(|asset| asset.source_path == "wall.glb")
        .expect("wall record exists")
        .semantic_tags
        .push("manual_tag".to_owned());
    loaded
        .pack
        .assets
        .retain(|asset| asset.source_path == "wall.glb");
    asset_mapper_io::write_pack_sidecar(temp.path(), &loaded.pack).expect("sidecar rewrites");

    std::fs::write(temp.path().join("wall.glb"), b"wall-v2").expect("wall changes");
    std::fs::write(temp.path().join("ceiling.glb"), b"ceiling").expect("new asset is written");
    std::fs::remove_file(temp.path().join("floor.glb")).expect("floor is removed");

    let report = index_pack_folder(temp.path()).expect("index succeeds");

    assert_eq!(report.drifted_assets, vec!["wall.glb"]);
    assert_eq!(report.new_assets, vec!["ceiling.glb"]);
    assert!(report.missing_assets.is_empty());

    let reloaded = read_pack_from_input(temp.path()).expect("sidecar reloads");
    let wall = reloaded
        .pack
        .assets
        .iter()
        .find(|asset| asset.source_path == "wall.glb")
        .expect("wall record remains");
    assert_eq!(wall.semantic_tags, vec!["manual_tag"]);
    assert!(
        wall.content_hash
            .ends_with(&loaded.pack.assets[0].content_hash[7..])
    );

    let ceiling = reloaded
        .pack
        .assets
        .iter()
        .find(|asset| asset.source_path == "ceiling.glb")
        .expect("new ceiling record exists");
    assert_eq!(ceiling.asset_id, "ceiling");
}
