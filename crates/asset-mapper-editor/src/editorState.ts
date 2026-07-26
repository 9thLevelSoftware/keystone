import type {
  AllowedRotation,
  AssetRecord,
  Axis3,
  CompatibilityRule,
  ConnectorClass,
  ConnectorFrame,
  ConnectorRecord,
  ConnectorRole,
  Diagnostic,
  EditorPackState,
  QuatXyzw,
  ReviewFlag,
  ValidationReport,
  Vec3,
} from "./types";

type Frame3d = Extract<ConnectorFrame, { kind: "frame3d" }>;
type Frame2d = Extract<ConnectorFrame, { kind: "frame2d" }>;
type Frame3dPatch = Omit<Frame3d, "kind">;

export type GroupedDiagnostics = Record<string, Diagnostic[]>;

const AXIS_OPTIONS: Axis3[] = [
  "pos_x",
  "neg_x",
  "pos_y",
  "neg_y",
  "pos_z",
  "neg_z",
];

export function axisOptions(): Axis3[] {
  return [...AXIS_OPTIONS];
}

export function selectAsset(
  state: EditorPackState,
  assetId: string,
): EditorPackState {
  if (!state.pack.assets.some((asset) => asset.asset_id === assetId)) {
    return state;
  }

  return {
    ...state,
    selectedAssetId: assetId,
    selectedConnectorId: null,
  };
}

export function selectConnector(
  state: EditorPackState,
  assetId: string,
  connectorId: string,
): EditorPackState {
  const asset = state.pack.assets.find((record) => record.asset_id === assetId);
  if (
    !asset ||
    !asset.connectors.some((connector) => connector.connector_id === connectorId)
  ) {
    return state;
  }

  return {
    ...state,
    selectedAssetId: assetId,
    selectedConnectorId: connectorId,
  };
}

export function selectDiagnosticTarget(
  state: EditorPackState,
  diagnostic: Diagnostic,
): EditorPackState {
  if (diagnostic.asset_id && diagnostic.connector_id) {
    return selectConnector(state, diagnostic.asset_id, diagnostic.connector_id);
  }
  if (diagnostic.asset_id) {
    return selectAsset(state, diagnostic.asset_id);
  }
  return state;
}

export function addConnector(
  state: EditorPackState,
  assetId: string,
  frameKind: "frame3d" | "frame2d" = "frame3d",
): EditorPackState {
  const asset = state.pack.assets.find((record) => record.asset_id === assetId);
  const connectorClass =
    suggestClassFromName(asset?.display_name ?? "") ??
    state.pack.connector_classes[0]?.class ??
    "";
  let connectorId = "";

  const nextState = updateAsset(state, assetId, (current) => {
    connectorId = nextConnectorId(current.connectors);
    const connector: ConnectorRecord =
      frameKind === "frame2d"
        ? {
            connector_id: connectorId,
            display_name: titleFromId(connectorId),
            class: connectorClass,
            role: "symmetric",
            frame: {
              kind: "frame2d",
              position: [0, 0],
              normal: [1, 0],
              grid_cell: null,
            },
            mating_axis: "pos_x",
            up_reference: "pos_y",
            snap_tolerance: 0.01,
          }
        : {
            connector_id: connectorId,
            display_name: titleFromId(connectorId),
            class: connectorClass,
            role: "symmetric",
            frame: {
              kind: "frame3d",
              position: [0, 0, 0],
              orientation_quat_xyzw: [0, 0, 0, 1],
            },
            mating_axis: "pos_z",
            up_reference: "pos_y",
            snap_tolerance: 0.01,
          };

    return {
      ...current,
      connectors: [...current.connectors, connector],
    };
  });

  if (nextState === state) {
    return state;
  }

  return {
    ...nextState,
    selectedAssetId: assetId,
    selectedConnectorId: connectorId,
    dirty: true,
  };
}

export function duplicateConnector(
  state: EditorPackState,
  assetId: string,
  connectorId: string,
): EditorPackState {
  let newId = "";
  const nextState = updateAsset(state, assetId, (asset) => {
    const source = asset.connectors.find((c) => c.connector_id === connectorId);
    if (!source) {
      return asset;
    }
    newId = nextConnectorId(asset.connectors);
    const clone: ConnectorRecord = {
      ...structuredClone(source),
      connector_id: newId,
      display_name: `${source.display_name} Copy`,
    };
    if (clone.frame.kind === "frame3d") {
      clone.frame = {
        ...clone.frame,
        position: [
          clone.frame.position[0] + 0.1,
          clone.frame.position[1],
          clone.frame.position[2],
        ],
      };
    } else {
      clone.frame = {
        ...clone.frame,
        position: [clone.frame.position[0] + 1, clone.frame.position[1]],
      };
    }
    return { ...asset, connectors: [...asset.connectors, clone] };
  });

  if (nextState === state || !newId) {
    return state;
  }

  return {
    ...nextState,
    selectedAssetId: assetId,
    selectedConnectorId: newId,
    dirty: true,
  };
}

export function snapConnectorToBoundsFace(
  state: EditorPackState,
  assetId: string,
  connectorId: string,
  face?: "pos_x" | "neg_x" | "pos_y" | "neg_y" | "pos_z" | "neg_z",
): EditorPackState {
  return updateConnectorRecord(state, assetId, connectorId, (connector, asset) => {
    if (connector.frame.kind !== "frame3d") {
      return connector;
    }
    const snaps = boundsFaceSnaps(asset.bounds);
    const target = face
      ? snaps.find((snap) => snap.name === face)
      : nearestFace(connector.frame.position, snaps);
    if (!target) {
      return connector;
    }
    return {
      ...connector,
      mating_axis: target.mating_axis,
      up_reference: target.up_reference,
      frame: {
        kind: "frame3d",
        position: target.position,
        orientation_quat_xyzw: target.orientation_quat_xyzw,
      },
    };
  });
}

export function updateConnectorFrame(
  state: EditorPackState,
  assetId: string,
  connectorId: string,
  frame: Frame3dPatch,
): EditorPackState {
  return updateConnectorRecord(state, assetId, connectorId, (connector) => ({
    ...connector,
    frame: {
      ...frame,
      kind: "frame3d",
    },
  }));
}

export function updateConnector(
  state: EditorPackState,
  assetId: string,
  connectorId: string,
  patch: Partial<ConnectorRecord>,
): EditorPackState {
  const nextState = updateConnectorRecord(
    state,
    assetId,
    connectorId,
    (connector) => ({
      ...connector,
      ...patch,
    }),
  );

  if (
    nextState !== state &&
    state.selectedAssetId === assetId &&
    state.selectedConnectorId === connectorId &&
    hasConnectorIdPatch(patch)
  ) {
    return {
      ...nextState,
      selectedConnectorId: patch.connector_id,
    };
  }

  return nextState;
}

function hasConnectorIdPatch(
  patch: Partial<ConnectorRecord>,
): patch is Partial<ConnectorRecord> & Pick<ConnectorRecord, "connector_id"> {
  return Object.prototype.hasOwnProperty.call(patch, "connector_id");
}

export function removeConnector(
  state: EditorPackState,
  assetId: string,
  connectorId: string,
): EditorPackState {
  const nextState = updateAsset(state, assetId, (asset) => {
    if (!asset.connectors.some((connector) => connector.connector_id === connectorId)) {
      return asset;
    }

    return {
      ...asset,
      connectors: asset.connectors.filter(
        (connector) => connector.connector_id !== connectorId,
      ),
    };
  });

  if (nextState === state) {
    return state;
  }

  return {
    ...nextState,
    selectedConnectorId: null,
    dirty: true,
  };
}

export function updateAssetMetadata(
  state: EditorPackState,
  assetId: string,
  patch: Partial<
    Pick<
      AssetRecord,
      | "display_name"
      | "pivot"
      | "up_axis"
      | "forward_axis"
      | "semantic_tags"
      | "affordances"
      | "placement_constraints"
      | "review_flags"
      | "bounds"
      | "dimensions"
    >
  >,
): EditorPackState {
  const nextState = updateAsset(state, assetId, (asset) => ({
    ...asset,
    ...patch,
  }));
  if (nextState === state) {
    return state;
  }
  return { ...nextState, dirty: true };
}

export function setReviewFlag(
  state: EditorPackState,
  assetId: string,
  flag: ReviewFlag,
  enabled: boolean,
): EditorPackState {
  return updateAssetMetadata(state, assetId, {
    review_flags: (() => {
      const asset = state.pack.assets.find((a) => a.asset_id === assetId);
      if (!asset) {
        return [];
      }
      const set = new Set(asset.review_flags);
      if (enabled) {
        set.add(flag);
      } else {
        set.delete(flag);
      }
      return [...set];
    })(),
  });
}

export function clearAllReviewFlags(
  state: EditorPackState,
  assetId: string,
): EditorPackState {
  return updateAssetMetadata(state, assetId, { review_flags: [] });
}

export function applyMeasuredBoundsToAsset(
  state: EditorPackState,
  assetId: string,
  bounds: { min: Vec3; max: Vec3 },
  dimensions: Vec3,
): EditorPackState {
  const asset = state.pack.assets.find((a) => a.asset_id === assetId);
  if (!asset) {
    return state;
  }
  const flags = asset.review_flags.filter((flag) => flag !== "bounds_placeholder");
  return updateAssetMetadata(state, assetId, {
    bounds,
    dimensions,
    review_flags: flags,
  });
}

export function addConnectorClass(
  state: EditorPackState,
  className: string,
  displayName: string,
): EditorPackState {
  if (
    state.pack.connector_classes.some(
      (connectorClass) => connectorClass.class === className,
    )
  ) {
    return state;
  }

  return {
    ...state,
    dirty: true,
    pack: {
      ...state.pack,
      connector_classes: [
        ...state.pack.connector_classes,
        { class: className, display_name: displayName },
      ],
    },
  };
}

export function updateConnectorClass(
  state: EditorPackState,
  index: number,
  patch: ConnectorClass,
): EditorPackState {
  const previous = state.pack.connector_classes[index];
  if (!previous) {
    return state;
  }

  return {
    ...state,
    dirty: true,
    pack: {
      ...state.pack,
      connector_classes: state.pack.connector_classes.map(
        (connectorClass, currentIndex) =>
          currentIndex === index ? patch : connectorClass,
      ),
      compatibility_rules: state.pack.compatibility_rules.map((rule) => ({
        ...rule,
        a_class: rule.a_class === previous.class ? patch.class : rule.a_class,
        b_class: rule.b_class === previous.class ? patch.class : rule.b_class,
      })),
      assets: state.pack.assets.map((asset) => ({
        ...asset,
        connectors: asset.connectors.map((connector) =>
          connector.class === previous.class
            ? { ...connector, class: patch.class }
            : connector,
        ),
      })),
    },
  };
}

export function removeConnectorClass(
  state: EditorPackState,
  index: number,
): EditorPackState {
  const removed = state.pack.connector_classes[index];
  if (!removed) {
    return state;
  }

  return {
    ...state,
    dirty: true,
    pack: {
      ...state.pack,
      connector_classes: state.pack.connector_classes.filter(
        (_, currentIndex) => currentIndex !== index,
      ),
      compatibility_rules: state.pack.compatibility_rules.filter(
        (rule) => rule.a_class !== removed.class && rule.b_class !== removed.class,
      ),
      assets: state.pack.assets.map((asset) => ({
        ...asset,
        connectors: asset.connectors.map((connector) =>
          connector.class === removed.class ? { ...connector, class: "" } : connector,
        ),
      })),
    },
  };
}

export function addCompatibilityRule(
  state: EditorPackState,
  aClass: string,
  bClass: string,
): EditorPackState {
  const rule: CompatibilityRule = {
    a_class: aClass,
    b_class: bClass,
    rotation: { kind: "locked" },
  };

  return {
    ...state,
    dirty: true,
    pack: {
      ...state.pack,
      compatibility_rules: [...state.pack.compatibility_rules, rule],
    },
  };
}

export function updateCompatibilityRule(
  state: EditorPackState,
  index: number,
  patch: CompatibilityRule,
): EditorPackState {
  if (!state.pack.compatibility_rules[index]) {
    return state;
  }

  return {
    ...state,
    dirty: true,
    pack: {
      ...state.pack,
      compatibility_rules: state.pack.compatibility_rules.map((rule, currentIndex) =>
        currentIndex === index ? patch : rule,
      ),
    },
  };
}

export function removeCompatibilityRule(
  state: EditorPackState,
  index: number,
): EditorPackState {
  if (!state.pack.compatibility_rules[index]) {
    return state;
  }

  return {
    ...state,
    dirty: true,
    pack: {
      ...state.pack,
      compatibility_rules: state.pack.compatibility_rules.filter(
        (_, currentIndex) => currentIndex !== index,
      ),
    },
  };
}

export function rotationFromKind(
  kind: AllowedRotation["kind"],
  previous?: AllowedRotation,
): AllowedRotation {
  if (kind === "locked") {
    return { kind: "locked" };
  }
  if (kind === "free") {
    return { kind: "free" };
  }
  const values =
    previous && previous.kind === "steps_deg" && previous.values.length > 0
      ? previous.values
      : [0, 90, 180, 270];
  return { kind: "steps_deg", values };
}

export function eulerDegToQuat(eulerDeg: Vec3): QuatXyzw {
  const [x, y, z] = eulerDeg.map((deg) => (deg * Math.PI) / 180) as Vec3;
  const cx = Math.cos(x / 2);
  const sx = Math.sin(x / 2);
  const cy = Math.cos(y / 2);
  const sy = Math.sin(y / 2);
  const cz = Math.cos(z / 2);
  const sz = Math.sin(z / 2);
  // q = q_yaw * q_pitch * q_roll  (matches quatToEulerDeg Tait–Bryan extraction)
  return [
    sx * cy * cz - cx * sy * sz,
    cx * sy * cz + sx * cy * sz,
    cx * cy * sz - sx * sy * cz,
    cx * cy * cz + sx * sy * sz,
  ];
}

export function quatToEulerDeg(quat: QuatXyzw): Vec3 {
  const [x, y, z, w] = quat;
  const sinrCosp = 2 * (w * x + y * z);
  const cosrCosp = 1 - 2 * (x * x + y * y);
  const roll = Math.atan2(sinrCosp, cosrCosp);

  const sinp = 2 * (w * y - z * x);
  const pitch = Math.abs(sinp) >= 1 ? (Math.sign(sinp) * Math.PI) / 2 : Math.asin(sinp);

  const sinyCosp = 2 * (w * z + x * y);
  const cosyCosp = 1 - 2 * (y * y + z * z);
  const yaw = Math.atan2(sinyCosp, cosyCosp);

  return [
    (roll * 180) / Math.PI,
    (pitch * 180) / Math.PI,
    (yaw * 180) / Math.PI,
  ];
}

export function suggestClassFromName(name: string): string | null {
  const lower = name.toLowerCase();
  const patterns: [string, string][] = [
    ["door", "doorway"],
    ["arch", "archway"],
    ["window", "window_frame"],
    ["floor", "floor_edge"],
    ["ceiling", "ceiling_edge"],
    ["wall", "wall_edge"],
    ["corridor", "corridor_end"],
    ["tile", "tile_edge"],
  ];
  for (const [needle, className] of patterns) {
    if (lower.includes(needle)) {
      return className;
    }
  }
  return null;
}

export function groupDiagnostics(
  report: ValidationReport,
): GroupedDiagnostics {
  return report.diagnostics.reduce<GroupedDiagnostics>((groups, diagnostic) => {
    const key = diagnosticGroupKey(diagnostic);
    const diagnostics = groups[key] ?? [];

    return {
      ...groups,
      [key]: [...diagnostics, diagnostic],
    };
  }, {});
}

export function parseStringList(value: string): string[] {
  return value
    .split(/[,;\n]/)
    .map((part) => part.trim())
    .filter((part) => part.length > 0);
}

export function formatStringList(values: string[]): string {
  return values.join(", ");
}

function updateConnectorRecord(
  state: EditorPackState,
  assetId: string,
  connectorId: string,
  update: (connector: ConnectorRecord, asset: AssetRecord) => ConnectorRecord,
): EditorPackState {
  const nextState = updateAsset(state, assetId, (asset) => {
    if (!asset.connectors.some((connector) => connector.connector_id === connectorId)) {
      return asset;
    }

    return {
      ...asset,
      connectors: asset.connectors.map((connector) =>
        connector.connector_id === connectorId ? update(connector, asset) : connector,
      ),
    };
  });

  if (nextState === state) {
    return state;
  }

  return {
    ...nextState,
    dirty: true,
  };
}

function updateAsset(
  state: EditorPackState,
  assetId: string,
  update: (asset: AssetRecord) => AssetRecord,
): EditorPackState {
  const asset = state.pack.assets.find((record) => record.asset_id === assetId);
  if (!asset) {
    return state;
  }

  const nextAsset = update(asset);
  if (nextAsset === asset) {
    return state;
  }

  return {
    ...state,
    pack: {
      ...state.pack,
      assets: state.pack.assets.map((record) =>
        record.asset_id === assetId ? nextAsset : record,
      ),
    },
  };
}

function nextConnectorId(connectors: ConnectorRecord[]): string {
  const existingIds = new Set(
    connectors.map((connector) => connector.connector_id),
  );
  let index = 1;

  while (existingIds.has(`connector_${index}`)) {
    index += 1;
  }

  return `connector_${index}`;
}

function titleFromId(id: string): string {
  return id
    .split("_")
    .filter((part) => part.length > 0)
    .map((part) => part.charAt(0).toUpperCase() + part.slice(1))
    .join(" ");
}

function diagnosticGroupKey(diagnostic: Diagnostic): string {
  if (diagnostic.asset_id && diagnostic.connector_id) {
    return `asset:${diagnostic.asset_id}/connector:${diagnostic.connector_id}`;
  }

  if (diagnostic.asset_id) {
    return `asset:${diagnostic.asset_id}`;
  }

  return "pack";
}

type FaceSnap = {
  name: "pos_x" | "neg_x" | "pos_y" | "neg_y" | "pos_z" | "neg_z";
  position: Vec3;
  orientation_quat_xyzw: QuatXyzw;
  mating_axis: Axis3;
  up_reference: Axis3;
};

function boundsFaceSnaps(bounds: { min: Vec3; max: Vec3 }): FaceSnap[] {
  const cx = (bounds.min[0] + bounds.max[0]) * 0.5;
  const cy = (bounds.min[1] + bounds.max[1]) * 0.5;
  const cz = (bounds.min[2] + bounds.max[2]) * 0.5;
  // Orientation maps local +Z to outward normal; mating_axis is always pos_z.
  return [
    faceSnap("pos_x", [bounds.max[0], cy, cz], [1, 0, 0], [0, 1, 0]),
    faceSnap("neg_x", [bounds.min[0], cy, cz], [-1, 0, 0], [0, 1, 0]),
    faceSnap("pos_y", [cx, bounds.max[1], cz], [0, 1, 0], [0, 0, 1]),
    faceSnap("neg_y", [cx, bounds.min[1], cz], [0, -1, 0], [0, 0, 1]),
    faceSnap("pos_z", [cx, cy, bounds.max[2]], [0, 0, 1], [0, 1, 0]),
    faceSnap("neg_z", [cx, cy, bounds.min[2]], [0, 0, -1], [0, 1, 0]),
  ];
}

function faceSnap(
  name: FaceSnap["name"],
  position: Vec3,
  outward: Vec3,
  upHint: Vec3,
): FaceSnap {
  return {
    name,
    position,
    orientation_quat_xyzw: orientationFacing(outward, upHint),
    mating_axis: "pos_z",
    up_reference: "pos_y",
  };
}

function orientationFacing(outward: Vec3, upHint: Vec3): QuatXyzw {
  const z = normalize3(outward);
  let y = sub3(upHint, scale3(z, dot3(upHint, z)));
  if (lengthSq3(y) < 1e-8) {
    const alt: Vec3 = Math.abs(z[0]) < 0.9 ? [1, 0, 0] : [0, 1, 0];
    y = sub3(alt, scale3(z, dot3(alt, z)));
  }
  y = normalize3(y);
  let x = normalize3(cross3(y, z));
  y = normalize3(cross3(z, x));
  // Mat3 columns x,y,z → quaternion (xyzw)
  const m00 = x[0];
  const m01 = y[0];
  const m02 = z[0];
  const m10 = x[1];
  const m11 = y[1];
  const m12 = z[1];
  const m20 = x[2];
  const m21 = y[2];
  const m22 = z[2];
  const trace = m00 + m11 + m22;
  let qx: number;
  let qy: number;
  let qz: number;
  let qw: number;
  if (trace > 0) {
    const s = Math.sqrt(trace + 1) * 2;
    qw = 0.25 * s;
    qx = (m21 - m12) / s;
    qy = (m02 - m20) / s;
    qz = (m10 - m01) / s;
  } else if (m00 > m11 && m00 > m22) {
    const s = Math.sqrt(1 + m00 - m11 - m22) * 2;
    qw = (m21 - m12) / s;
    qx = 0.25 * s;
    qy = (m01 + m10) / s;
    qz = (m02 + m20) / s;
  } else if (m11 > m22) {
    const s = Math.sqrt(1 + m11 - m00 - m22) * 2;
    qw = (m02 - m20) / s;
    qx = (m01 + m10) / s;
    qy = 0.25 * s;
    qz = (m12 + m21) / s;
  } else {
    const s = Math.sqrt(1 + m22 - m00 - m11) * 2;
    qw = (m10 - m01) / s;
    qx = (m02 + m20) / s;
    qy = (m12 + m21) / s;
    qz = 0.25 * s;
  }
  const len = Math.hypot(qx, qy, qz, qw) || 1;
  return [qx / len, qy / len, qz / len, qw / len];
}

function dot3(a: Vec3, b: Vec3): number {
  return a[0] * b[0] + a[1] * b[1] + a[2] * b[2];
}
function cross3(a: Vec3, b: Vec3): Vec3 {
  return [
    a[1] * b[2] - a[2] * b[1],
    a[2] * b[0] - a[0] * b[2],
    a[0] * b[1] - a[1] * b[0],
  ];
}
function sub3(a: Vec3, b: Vec3): Vec3 {
  return [a[0] - b[0], a[1] - b[1], a[2] - b[2]];
}
function scale3(a: Vec3, s: number): Vec3 {
  return [a[0] * s, a[1] * s, a[2] * s];
}
function lengthSq3(a: Vec3): number {
  return dot3(a, a);
}
function normalize3(a: Vec3): Vec3 {
  const len = Math.sqrt(lengthSq3(a)) || 1;
  return [a[0] / len, a[1] / len, a[2] / len];
}

function nearestFace(position: Vec3, snaps: FaceSnap[]): FaceSnap | undefined {
  let best: FaceSnap | undefined;
  let bestDist = Number.POSITIVE_INFINITY;
  for (const snap of snaps) {
    const dx = snap.position[0] - position[0];
    const dy = snap.position[1] - position[1];
    const dz = snap.position[2] - position[2];
    const dist = dx * dx + dy * dy + dz * dz;
    if (dist < bestDist) {
      bestDist = dist;
      best = snap;
    }
  }
  return best;
}

export type { Frame2d, ConnectorRole };
