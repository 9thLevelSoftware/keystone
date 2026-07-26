pub mod analyze;
pub mod assembly_propose;
pub mod bundle;
pub mod diagnostics;
pub mod export;
pub mod hash;
pub mod mesh_geometry;
pub mod mesh_sockets;
pub mod migrate;
pub mod resolver;
pub mod schema;
pub mod suggest;
pub mod validate;
pub mod vibe_readiness;

pub use analyze::{AnalyzeOptions, AnalyzeReport, analyze_pack, analyze_pack_with_meshes};
pub use assembly_propose::{
    ProposeAssemblyOptions, ProposeAssemblyReport, propose_assembly_plan, rule_partner_map,
};
pub use bundle::{BundleAsset, BundleConnector, HOW_TO_PLAN, LlmBundle, PlanContract};
pub use diagnostics::{Diagnostic, Severity, ValidationReport};
pub use export::{
    GltfKeystoneExtras, GodotExport, UnityExport, UnrealExport, export_connectors_csv,
    export_godot, export_unity, export_unreal, gltf_keystone_extras,
};
pub use mesh_geometry::MeshGeometry;
pub use mesh_sockets::{
    ProposedSocket, SocketProposeOptions, SocketSource, propose_sockets_from_bounds,
    propose_sockets_from_mesh,
};
pub use migrate::{MigrationError, MigrationReport, migrate_pack, pack_from_legacy_json};
pub use resolver::{
    AssetPlacement, MateEndpoints, ResolveError, ResolveErrorReport, ResolveFixTarget,
    ResolvedScene, resolve_plan,
};
pub use schema::*;
pub use suggest::{
    FaceSnap, SuggestedSemantics, bounds_face_snaps, connector_on_face, duplicate_connector,
    snap_connector_to_nearest_face, suggest_class_from_asset, suggest_class_from_name,
    suggest_semantics_for_asset,
};
pub use validate::validate_pack;
pub use vibe_readiness::{VibeChecklistItem, VibeReadinessReport, vibe_readiness};
