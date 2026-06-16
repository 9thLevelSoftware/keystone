import type {
  AssetRecord,
  CompatibilityRule,
  ConnectorFrame,
  ConnectorRecord,
  Diagnostic,
  EditorPackState,
  ValidationReport,
} from "./types";

type Frame3d = Extract<ConnectorFrame, { kind: "frame3d" }>;
type Frame3dPatch = Omit<Frame3d, "kind">;

export type GroupedDiagnostics = Record<string, Diagnostic[]>;

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

export function addConnector(
  state: EditorPackState,
  assetId: string,
): EditorPackState {
  const connectorClass = state.pack.connector_classes[0]?.class ?? "";
  let connectorId = "";

  const nextState = updateAsset(state, assetId, (asset) => {
    connectorId = nextConnectorId(asset.connectors);
    const connector: ConnectorRecord = {
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
      ...asset,
      connectors: [...asset.connectors, connector],
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

function updateConnectorRecord(
  state: EditorPackState,
  assetId: string,
  connectorId: string,
  update: (connector: ConnectorRecord) => ConnectorRecord,
): EditorPackState {
  const nextState = updateAsset(state, assetId, (asset) => {
    if (!asset.connectors.some((connector) => connector.connector_id === connectorId)) {
      return asset;
    }

    return {
      ...asset,
      connectors: asset.connectors.map((connector) =>
        connector.connector_id === connectorId ? update(connector) : connector,
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
