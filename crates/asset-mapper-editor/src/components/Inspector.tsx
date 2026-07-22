import {
  addConnector,
  removeConnector,
  updateConnector,
} from "../editorState";
import type { AssetRecord, ConnectorRecord, EditorPackState } from "../types";
import RulesEditor from "./RulesEditor";

interface InspectorProps {
  state: EditorPackState | null;
  selectedAsset: AssetRecord | null;
  selectedConnector: ConnectorRecord | null;
  onStateChange: (state: EditorPackState) => void;
  onSelectConnector: (assetId: string, connectorId: string) => void;
}

export default function Inspector({
  state,
  selectedAsset,
  selectedConnector,
  onStateChange,
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

  function patchConnector(patch: Partial<ConnectorRecord>) {
    if (!state || !selectedAsset || !selectedConnector) {
      return;
    }

    onStateChange(
      updateConnector(
        state,
        selectedAsset.asset_id,
        selectedConnector.connector_id,
        patch,
      ),
    );
  }

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

      <RulesEditor state={state} onStateChange={onStateChange} />

      <div className="section-heading">
        <h2>Connectors</h2>
        <button
          type="button"
          onClick={() => onStateChange(addConnector(state, selectedAsset.asset_id))}
        >
          Add
        </button>
      </div>

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
          <label>
            ID
            <input
              value={selectedConnector.connector_id}
              onChange={(event) =>
                patchConnector({ connector_id: event.currentTarget.value })
              }
            />
          </label>
          <label>
            Name
            <input
              value={selectedConnector.display_name}
              onChange={(event) =>
                patchConnector({ display_name: event.currentTarget.value })
              }
            />
          </label>
          <label>
            Class
            <select
              value={selectedConnector.class}
              onChange={(event) =>
                patchConnector({ class: event.currentTarget.value })
              }
            >
              <option value="">Unassigned</option>
              {state.pack.connector_classes.map((connectorClass, index) => (
                <option
                  key={`${connectorClass.class}-${index}`}
                  value={connectorClass.class}
                >
                  {connectorClass.display_name}
                </option>
              ))}
            </select>
          </label>
          <label>
            Snap tolerance
            <input
              type="number"
              step="0.01"
              value={selectedConnector.snap_tolerance}
              onChange={(event) =>
                patchConnector({ snap_tolerance: Number(event.currentTarget.value) })
              }
            />
          </label>
          {selectedConnector.frame.kind === "frame3d" ? (
            <div className="numeric-grid">
              {(["X", "Y", "Z"] as const).map((label, index) => (
                <label key={label}>
                  {label}
                  <input
                    type="number"
                    step="0.01"
                    value={selectedConnector.frame.position[index]}
                    onChange={(event) => {
                      if (selectedConnector.frame.kind !== "frame3d") {
                        return;
                      }

                      const position = [...selectedConnector.frame.position] as [
                        number,
                        number,
                        number,
                      ];
                      position[index] = Number(event.currentTarget.value);
                      patchConnector({
                        frame: { ...selectedConnector.frame, position },
                      });
                    }}
                  />
                </label>
              ))}
            </div>
          ) : null}
          <button
            type="button"
            onClick={() =>
              onStateChange(
                removeConnector(
                  state,
                  selectedAsset.asset_id,
                  selectedConnector.connector_id,
                ),
              )
            }
          >
            Delete Connector
          </button>
        </section>
      ) : null}
    </aside>
  );
}
