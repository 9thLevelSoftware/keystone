import type { AssetRecord, ConnectorRecord, EditorPackState } from "../types";

interface InspectorProps {
  state: EditorPackState | null;
  selectedAsset: AssetRecord | null;
  selectedConnector: ConnectorRecord | null;
  onSelectConnector: (assetId: string, connectorId: string) => void;
}

export default function Inspector({
  state,
  selectedAsset,
  selectedConnector,
  onSelectConnector,
}: InspectorProps) {
  if (!state || !selectedAsset) {
    return (
      <aside className="inspector-panel">
        <h1>Asset Mapper</h1>
        <p className="muted">Open a pack folder to inspect assets.</p>
      </aside>
    );
  }

  const sourceStatus = state.assets.find(
    (asset) => asset.assetId === selectedAsset.asset_id,
  );

  return (
    <aside className="inspector-panel">
      <h1>{selectedAsset.display_name}</h1>
      <dl className="property-list">
        <dt>Source</dt>
        <dd>{selectedAsset.source_path}</dd>
        <dt>Hash</dt>
        <dd>{sourceStatus?.hashMatches === false ? "Drifted" : "Current"}</dd>
        <dt>Dimensions</dt>
        <dd>{selectedAsset.dimensions.join(" x ")}</dd>
        <dt>Bounds</dt>
        <dd>
          {selectedAsset.bounds.min.join(", ")} to{" "}
          {selectedAsset.bounds.max.join(", ")}
        </dd>
      </dl>

      <h2>Connectors</h2>
      {selectedAsset.connectors.length === 0 ? (
        <p className="muted">No connectors.</p>
      ) : (
        <ul className="connector-list">
          {selectedAsset.connectors.map((connector) => (
            <li key={connector.connector_id}>
              <button
                type="button"
                className={
                  connector.connector_id === selectedConnector?.connector_id
                    ? "connector-list-item selected"
                    : "connector-list-item"
                }
                onClick={() =>
                  onSelectConnector(selectedAsset.asset_id, connector.connector_id)
                }
              >
                {connector.display_name} ({connector.class || "unclassified"})
              </button>
            </li>
          ))}
        </ul>
      )}

      {selectedConnector ? (
        <section className="connector-details">
          <h2>{selectedConnector.display_name}</h2>
          <dl className="property-list">
            <dt>ID</dt>
            <dd>{selectedConnector.connector_id}</dd>
            <dt>Class</dt>
            <dd>{selectedConnector.class || "Unassigned"}</dd>
            <dt>Role</dt>
            <dd>{selectedConnector.role}</dd>
            <dt>Snap</dt>
            <dd>{selectedConnector.snap_tolerance}</dd>
          </dl>
        </section>
      ) : null}
    </aside>
  );
}
