//! Engine / DCC export helpers and glTF metadata mirrors.
//!
//! Canonical data remains the sidecar. Exports are lossy mirrors for engines
//! and DCC tools that cannot read `.assetmap.json` directly.

use crate::schema::{
    AllowedRotation, AssetRecord, ConnectorFrame, ConnectorRecord, PackRecord, Transform3d,
};

/// Companion JSON document intended to sit beside a glTF/GLB as
/// `<name>.keystone.json`, or to be embedded under `extras.keystone`.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct GltfKeystoneExtras {
    pub schema: &'static str,
    pub schema_version: u32,
    pub pack_id: String,
    pub display_name: String,
    pub assets: Vec<GltfAssetExtras>,
    pub connector_classes: Vec<crate::schema::ConnectorClass>,
    pub compatibility_rules: Vec<crate::schema::CompatibilityRule>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct GltfAssetExtras {
    pub asset_id: String,
    pub source_path: String,
    pub dimensions: [f32; 3],
    pub bounds: crate::schema::Bounds3,
    pub semantic_tags: Vec<String>,
    pub affordances: Vec<String>,
    pub placement_constraints: Vec<String>,
    pub connectors: Vec<GltfConnectorExtras>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct GltfConnectorExtras {
    pub connector_id: String,
    pub display_name: String,
    pub class: String,
    pub role: crate::schema::ConnectorRole,
    pub frame: ConnectorFrame,
    pub mating_axis: crate::schema::Axis3,
    pub up_reference: crate::schema::Axis3,
    pub snap_tolerance: f32,
}

pub fn gltf_keystone_extras(pack: &PackRecord) -> GltfKeystoneExtras {
    GltfKeystoneExtras {
        schema: "keystone.gltf.extras/v1",
        schema_version: pack.schema_version,
        pack_id: pack.pack_id.clone(),
        display_name: pack.display_name.clone(),
        assets: pack.assets.iter().map(gltf_asset_extras).collect(),
        connector_classes: pack.connector_classes.clone(),
        compatibility_rules: pack.compatibility_rules.clone(),
    }
}

fn gltf_asset_extras(asset: &AssetRecord) -> GltfAssetExtras {
    GltfAssetExtras {
        asset_id: asset.asset_id.clone(),
        source_path: asset.source_path.clone(),
        dimensions: asset.dimensions,
        bounds: asset.bounds,
        semantic_tags: asset.semantic_tags.clone(),
        affordances: asset.affordances.clone(),
        placement_constraints: asset.placement_constraints.clone(),
        connectors: asset.connectors.iter().map(gltf_connector_extras).collect(),
    }
}

fn gltf_connector_extras(connector: &ConnectorRecord) -> GltfConnectorExtras {
    GltfConnectorExtras {
        connector_id: connector.connector_id.clone(),
        display_name: connector.display_name.clone(),
        class: connector.class.clone(),
        role: connector.role.clone(),
        frame: connector.frame.clone(),
        mating_axis: connector.mating_axis,
        up_reference: connector.up_reference,
        snap_tolerance: connector.snap_tolerance,
    }
}

/// Flat connector table for Unreal DataTable / Blueprint import (JSON array).
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct UnrealConnectorRow {
    pub pack_id: String,
    pub asset_id: String,
    pub source_path: String,
    pub connector_id: String,
    pub display_name: String,
    pub class: String,
    pub role: String,
    pub location_x: f32,
    pub location_y: f32,
    pub location_z: f32,
    pub quat_x: f32,
    pub quat_y: f32,
    pub quat_z: f32,
    pub quat_w: f32,
    pub mating_axis: String,
    pub up_reference: String,
    pub snap_tolerance: f32,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct UnrealRuleRow {
    pub pack_id: String,
    pub a_class: String,
    pub b_class: String,
    pub rotation_kind: String,
    pub rotation_steps_deg: Vec<f32>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct UnrealExport {
    pub connectors: Vec<UnrealConnectorRow>,
    pub rules: Vec<UnrealRuleRow>,
}

pub fn export_unreal(pack: &PackRecord) -> UnrealExport {
    let mut connectors = Vec::new();
    for asset in &pack.assets {
        for connector in &asset.connectors {
            let (loc, quat) = frame_as_xyz_quat(&connector.frame);
            connectors.push(UnrealConnectorRow {
                pack_id: pack.pack_id.clone(),
                asset_id: asset.asset_id.clone(),
                source_path: asset.source_path.clone(),
                connector_id: connector.connector_id.clone(),
                display_name: connector.display_name.clone(),
                class: connector.class.clone(),
                role: role_str(&connector.role).to_owned(),
                location_x: loc[0],
                location_y: loc[1],
                location_z: loc[2],
                quat_x: quat[0],
                quat_y: quat[1],
                quat_z: quat[2],
                quat_w: quat[3],
                mating_axis: axis_str(connector.mating_axis).to_owned(),
                up_reference: axis_str(connector.up_reference).to_owned(),
                snap_tolerance: connector.snap_tolerance,
            });
        }
    }

    let rules = pack
        .compatibility_rules
        .iter()
        .map(|rule| {
            let (kind, steps) = rotation_parts(&rule.rotation);
            UnrealRuleRow {
                pack_id: pack.pack_id.clone(),
                a_class: rule.a_class.clone(),
                b_class: rule.b_class.clone(),
                rotation_kind: kind.to_owned(),
                rotation_steps_deg: steps,
            }
        })
        .collect();

    UnrealExport { connectors, rules }
}

/// Unity-friendly ScriptableObject JSON shape.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct UnityExport {
    pub pack_id: String,
    pub display_name: String,
    pub assets: Vec<UnityAsset>,
    pub rules: Vec<UnityRule>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct UnityAsset {
    pub asset_id: String,
    pub source_path: String,
    pub dimensions: [f32; 3],
    pub connectors: Vec<UnityConnector>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct UnityConnector {
    pub connector_id: String,
    pub class_name: String,
    pub role: String,
    pub local_position: [f32; 3],
    pub local_rotation: [f32; 4],
    pub mating_axis: String,
    pub up_reference: String,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct UnityRule {
    pub a_class: String,
    pub b_class: String,
    pub rotation: String,
    pub steps_deg: Vec<f32>,
}

pub fn export_unity(pack: &PackRecord) -> UnityExport {
    UnityExport {
        pack_id: pack.pack_id.clone(),
        display_name: pack.display_name.clone(),
        assets: pack
            .assets
            .iter()
            .map(|asset| UnityAsset {
                asset_id: asset.asset_id.clone(),
                source_path: asset.source_path.clone(),
                dimensions: asset.dimensions,
                connectors: asset
                    .connectors
                    .iter()
                    .map(|connector| {
                        let (pos, quat) = frame_as_xyz_quat(&connector.frame);
                        UnityConnector {
                            connector_id: connector.connector_id.clone(),
                            class_name: connector.class.clone(),
                            role: role_str(&connector.role).to_owned(),
                            local_position: pos,
                            local_rotation: quat,
                            mating_axis: axis_str(connector.mating_axis).to_owned(),
                            up_reference: axis_str(connector.up_reference).to_owned(),
                        }
                    })
                    .collect(),
            })
            .collect(),
        rules: pack
            .compatibility_rules
            .iter()
            .map(|rule| {
                let (kind, steps) = rotation_parts(&rule.rotation);
                UnityRule {
                    a_class: rule.a_class.clone(),
                    b_class: rule.b_class.clone(),
                    rotation: kind.to_owned(),
                    steps_deg: steps,
                }
            })
            .collect(),
    }
}

/// Godot resource-friendly JSON (import as Dictionary / custom Resource).
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct GodotExport {
    pub resource_type: &'static str,
    pub pack_id: String,
    pub display_name: String,
    pub connectors: Vec<GodotConnector>,
    pub rules: Vec<GodotRule>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct GodotConnector {
    pub asset_id: String,
    pub connector_id: String,
    pub class_name: String,
    pub role: String,
    pub position: [f32; 3],
    pub quaternion: [f32; 4],
    pub mating_axis: String,
    pub up_reference: String,
    pub snap_tolerance: f32,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct GodotRule {
    pub a_class: String,
    pub b_class: String,
    pub rotation_policy: String,
    pub steps_deg: Vec<f32>,
}

pub fn export_godot(pack: &PackRecord) -> GodotExport {
    let mut connectors = Vec::new();
    for asset in &pack.assets {
        for connector in &asset.connectors {
            let (pos, quat) = frame_as_xyz_quat(&connector.frame);
            connectors.push(GodotConnector {
                asset_id: asset.asset_id.clone(),
                connector_id: connector.connector_id.clone(),
                class_name: connector.class.clone(),
                role: role_str(&connector.role).to_owned(),
                position: pos,
                quaternion: quat,
                mating_axis: axis_str(connector.mating_axis).to_owned(),
                up_reference: axis_str(connector.up_reference).to_owned(),
                snap_tolerance: connector.snap_tolerance,
            });
        }
    }
    GodotExport {
        resource_type: "KeystonePack",
        pack_id: pack.pack_id.clone(),
        display_name: pack.display_name.clone(),
        connectors,
        rules: pack
            .compatibility_rules
            .iter()
            .map(|rule| {
                let (kind, steps) = rotation_parts(&rule.rotation);
                GodotRule {
                    a_class: rule.a_class.clone(),
                    b_class: rule.b_class.clone(),
                    rotation_policy: kind.to_owned(),
                    steps_deg: steps,
                }
            })
            .collect(),
    }
}

/// CSV rows for spreadsheet / DataTable import of connectors.
pub fn export_connectors_csv(pack: &PackRecord) -> String {
    let mut out = String::from(
        "pack_id,asset_id,connector_id,class,role,x,y,z,qx,qy,qz,qw,mating_axis,up_reference,snap_tolerance\n",
    );
    for asset in &pack.assets {
        for connector in &asset.connectors {
            let (pos, quat) = frame_as_xyz_quat(&connector.frame);
            out.push_str(&format!(
                "{},{},{},{},{},{},{},{},{},{},{},{},{},{},{}\n",
                csv_escape(&pack.pack_id),
                csv_escape(&asset.asset_id),
                csv_escape(&connector.connector_id),
                csv_escape(&connector.class),
                role_str(&connector.role),
                pos[0],
                pos[1],
                pos[2],
                quat[0],
                quat[1],
                quat[2],
                quat[3],
                axis_str(connector.mating_axis),
                axis_str(connector.up_reference),
                connector.snap_tolerance,
            ));
        }
    }
    out
}

fn frame_as_xyz_quat(frame: &ConnectorFrame) -> ([f32; 3], [f32; 4]) {
    match frame {
        ConnectorFrame::Frame3d {
            position,
            orientation_quat_xyzw,
        } => (*position, *orientation_quat_xyzw),
        ConnectorFrame::Frame2d {
            position, normal, ..
        } => {
            // Embed 2D frame in XY plane; orientation faces along normal in XY.
            let angle = normal[1].atan2(normal[0]);
            let half = angle * 0.5;
            let quat = [0.0, 0.0, half.sin(), half.cos()];
            ([position[0], position[1], 0.0], quat)
        }
    }
}

fn rotation_parts(rotation: &AllowedRotation) -> (&'static str, Vec<f32>) {
    match rotation {
        AllowedRotation::Locked => ("locked", vec![]),
        AllowedRotation::Free => ("free", vec![]),
        AllowedRotation::StepsDeg { values } => ("steps_deg", values.clone()),
    }
}

fn role_str(role: &crate::schema::ConnectorRole) -> &'static str {
    match role {
        crate::schema::ConnectorRole::Symmetric => "symmetric",
        crate::schema::ConnectorRole::Plug => "plug",
        crate::schema::ConnectorRole::Receptacle => "receptacle",
    }
}

fn axis_str(axis: crate::schema::Axis3) -> &'static str {
    match axis {
        crate::schema::Axis3::PosX => "pos_x",
        crate::schema::Axis3::NegX => "neg_x",
        crate::schema::Axis3::PosY => "pos_y",
        crate::schema::Axis3::NegY => "neg_y",
        crate::schema::Axis3::PosZ => "pos_z",
        crate::schema::Axis3::NegZ => "neg_z",
    }
}

fn csv_escape(value: &str) -> String {
    if value.contains(',') || value.contains('"') || value.contains('\n') {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_owned()
    }
}

/// Identity transform helper re-export for engine docs examples.
pub fn identity_transform() -> Transform3d {
    Transform3d::identity()
}
