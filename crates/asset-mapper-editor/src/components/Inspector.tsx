import {
  addConnector,
  axisOptions,
  clearAllReviewFlags,
  duplicateConnector,
  eulerDegToQuat,
  formatStringList,
  parseStringList,
  quatToEulerDeg,
  removeConnector,
  setReviewFlag,
  snapConnectorToBoundsFace,
  suggestClassFromName,
  updateAssetMetadata,
  updateConnector,
} from "../editorState";
import type {
  AssetRecord,
  Axis3,
  ConnectorRecord,
  ConnectorRole,
  EditorPackState,
  ReviewFlag,
  Vec3,
} from "../types";
import PackSettings from "./PackSettings";
import RulesEditor from "./RulesEditor";

interface InspectorProps {
  state: EditorPackState | null;
  selectedAsset: AssetRecord | null;
  selectedConnector: ConnectorRecord | null;
  onStateChange: (state: EditorPackState) => void;
  onSelectConnector: (assetId: string, connectorId: string) => void;
  onMeasureBounds: () => void;
  onAcceptDrift: () => void;
}

const REVIEW_FLAGS: { flag: ReviewFlag; label: string }[] = [
  { flag: "bounds_placeholder", label: "Bounds placeholder" },
  { flag: "orientation_placeholder", label: "Orientation placeholder" },
  { flag: "pivot_placeholder", label: "Pivot placeholder" },
  {
    flag: "auto_from_bounds_fallback",
    label: "Auto from AABB (mesh fallback)",
  },
];

const ROLES: ConnectorRole[] = ["symmetric", "plug", "receptacle"];

export default function Inspector({
  state,
  selectedAsset,
  selectedConnector,
  onStateChange,
  onSelectConnector,
  onMeasureBounds,
  onAcceptDrift,
}: InspectorProps) {
  if (!state) {
    return (
      <aside className="inspector-panel">
        <h1>Asset Mapper</h1>
        <p className="muted">Open a pack folder to inspect assets.</p>
      </aside>
    );
  }

  if (!selectedAsset) {
    return (
      <aside className="inspector-panel">
        <h1>{state.pack.display_name}</h1>
        <p className="muted">Select an asset to author connectors and tags.</p>
        <PackSettings state={state} onStateChange={onStateChange} />
        <RulesEditor state={state} onStateChange={onStateChange} />
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

  function patchAsset(
    patch: Parameters<typeof updateAssetMetadata>[2],
  ) {
    if (!state || !selectedAsset) {
      return;
    }
    onStateChange(updateAssetMetadata(state, selectedAsset.asset_id, patch));
  }

  const frame3d =
    selectedConnector?.frame.kind === "frame3d" ? selectedConnector.frame : null;
  const frame2d =
    selectedConnector?.frame.kind === "frame2d" ? selectedConnector.frame : null;
  const euler = frame3d
    ? quatToEulerDeg(frame3d.orientation_quat_xyzw)
    : ([0, 0, 0] as Vec3);

  const is2d =
    selectedAsset.asset_type === "sprite2d" ||
    selectedAsset.asset_type === "tile2d";

  return (
    <aside className="inspector-panel">
      <h1>{selectedAsset.display_name}</h1>
      <label>
        Display name
        <input
          value={selectedAsset.display_name}
          onChange={(event) =>
            patchAsset({ display_name: event.currentTarget.value })
          }
        />
      </label>
      <dl className="property-list">
        <dt>Source</dt>
        <dd>{selectedAsset.source_path}</dd>
        <dt>Type</dt>
        <dd>{selectedAsset.asset_type}</dd>
        <dt>Hash</dt>
        <dd>
          {sourceStatus?.hashMatches === false ? "Drifted" : "Current"}
          {sourceStatus?.hashMatches === false ? (
            <button type="button" className="inline-action" onClick={onAcceptDrift}>
              Accept drift
            </button>
          ) : null}
        </dd>
      </dl>

      <div className="section-heading">
        <h2>Bounds &amp; axes</h2>
        <button type="button" onClick={onMeasureBounds}>
          Measure from mesh
        </button>
      </div>
      <div className="numeric-grid">
        {(["X", "Y", "Z"] as const).map((label, index) => (
          <label key={`dim-${label}`}>
            Dim {label}
            <input
              type="number"
              step="0.01"
              value={selectedAsset.dimensions[index]}
              onChange={(event) => {
                const dimensions = [...selectedAsset.dimensions] as Vec3;
                dimensions[index] = Number(event.currentTarget.value);
                patchAsset({ dimensions });
              }}
            />
          </label>
        ))}
      </div>
      <div className="numeric-grid">
        {(["minX", "minY", "minZ"] as const).map((label, index) => (
          <label key={label}>
            {label}
            <input
              type="number"
              step="0.01"
              value={selectedAsset.bounds.min[index]}
              onChange={(event) => {
                const min = [...selectedAsset.bounds.min] as Vec3;
                min[index] = Number(event.currentTarget.value);
                patchAsset({ bounds: { ...selectedAsset.bounds, min } });
              }}
            />
          </label>
        ))}
        {(["maxX", "maxY", "maxZ"] as const).map((label, index) => (
          <label key={label}>
            {label}
            <input
              type="number"
              step="0.01"
              value={selectedAsset.bounds.max[index]}
              onChange={(event) => {
                const max = [...selectedAsset.bounds.max] as Vec3;
                max[index] = Number(event.currentTarget.value);
                patchAsset({ bounds: { ...selectedAsset.bounds, max } });
              }}
            />
          </label>
        ))}
      </div>
      <label>
        Pivot
        <select
          value={selectedAsset.pivot}
          onChange={(event) =>
            patchAsset({
              pivot: event.currentTarget.value as AssetRecord["pivot"],
            })
          }
        >
          <option value="origin">Origin</option>
          <option value="base_center">Base center</option>
          <option value="center">Center</option>
          <option value="custom">Custom</option>
        </select>
      </label>
      <label>
        Up axis
        <select
          value={selectedAsset.up_axis}
          onChange={(event) =>
            patchAsset({ up_axis: event.currentTarget.value as Axis3 })
          }
        >
          {axisOptions().map((axis) => (
            <option key={axis} value={axis}>
              {axis}
            </option>
          ))}
        </select>
      </label>
      <label>
        Forward axis
        <select
          value={selectedAsset.forward_axis}
          onChange={(event) =>
            patchAsset({ forward_axis: event.currentTarget.value as Axis3 })
          }
        >
          {axisOptions().map((axis) => (
            <option key={axis} value={axis}>
              {axis}
            </option>
          ))}
        </select>
      </label>

      <h2>Semantics</h2>
      <label>
        Tags
        <input
          value={formatStringList(selectedAsset.semantic_tags)}
          placeholder="wall, modular"
          onChange={(event) =>
            patchAsset({ semantic_tags: parseStringList(event.currentTarget.value) })
          }
        />
      </label>
      <label>
        Affordances
        <input
          value={formatStringList(selectedAsset.affordances)}
          placeholder="walkable, climbable"
          onChange={(event) =>
            patchAsset({ affordances: parseStringList(event.currentTarget.value) })
          }
        />
      </label>
      <label>
        Placement constraints
        <input
          value={formatStringList(selectedAsset.placement_constraints)}
          placeholder="upright_only"
          onChange={(event) =>
            patchAsset({
              placement_constraints: parseStringList(event.currentTarget.value),
            })
          }
        />
      </label>

      <div className="section-heading">
        <h2>Review flags</h2>
        <button
          type="button"
          onClick={() =>
            onStateChange(clearAllReviewFlags(state, selectedAsset.asset_id))
          }
        >
          Clear all
        </button>
      </div>
      {REVIEW_FLAGS.map(({ flag, label }) => (
        <label key={flag} className="checkbox-row">
          <input
            type="checkbox"
            checked={selectedAsset.review_flags.includes(flag)}
            onChange={(event) =>
              onStateChange(
                setReviewFlag(
                  state,
                  selectedAsset.asset_id,
                  flag,
                  event.currentTarget.checked,
                ),
              )
            }
          />
          {label}
        </label>
      ))}

      <PackSettings state={state} onStateChange={onStateChange} />
      <RulesEditor state={state} onStateChange={onStateChange} />

      <div className="section-heading">
        <h2>Connectors</h2>
        <div className="button-row">
          <button
            type="button"
            onClick={() =>
              onStateChange(
                addConnector(state, selectedAsset.asset_id, is2d ? "frame2d" : "frame3d"),
              )
            }
          >
            Add
          </button>
          {suggestClassFromName(selectedAsset.display_name) ? (
            <span className="muted small">
              Suggest class: {suggestClassFromName(selectedAsset.display_name)}
            </span>
          ) : null}
        </div>
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
                {connector.display_name} ({connector.class || "unclassified"}) [
                {connector.frame.kind === "frame2d" ? "2D" : "3D"}]
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
            Role
            <select
              value={selectedConnector.role}
              onChange={(event) =>
                patchConnector({ role: event.currentTarget.value as ConnectorRole })
              }
            >
              {ROLES.map((role) => (
                <option key={role} value={role}>
                  {role}
                </option>
              ))}
            </select>
          </label>
          <label>
            Mating axis
            <select
              value={selectedConnector.mating_axis}
              onChange={(event) =>
                patchConnector({ mating_axis: event.currentTarget.value as Axis3 })
              }
            >
              {axisOptions().map((axis) => (
                <option key={axis} value={axis}>
                  {axis}
                </option>
              ))}
            </select>
          </label>
          <label>
            Up reference
            <select
              value={selectedConnector.up_reference}
              onChange={(event) =>
                patchConnector({
                  up_reference: event.currentTarget.value as Axis3,
                })
              }
            >
              {axisOptions().map((axis) => (
                <option key={axis} value={axis}>
                  {axis}
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

          {frame3d ? (
            <>
              <h3>Position</h3>
              <div className="numeric-grid">
                {(["X", "Y", "Z"] as const).map((label, index) => (
                  <label key={label}>
                    {label}
                    <input
                      type="number"
                      step="0.01"
                      value={frame3d.position[index]}
                      onChange={(event) => {
                        const position = [...frame3d.position] as Vec3;
                        position[index] = Number(event.currentTarget.value);
                        patchConnector({
                          frame: { ...frame3d, position },
                        });
                      }}
                    />
                  </label>
                ))}
              </div>
              <h3>Orientation (Euler deg)</h3>
              <div className="numeric-grid">
                {(["X", "Y", "Z"] as const).map((label, index) => (
                  <label key={`euler-${label}`}>
                    {label}
                    <input
                      type="number"
                      step="1"
                      value={Number(euler[index].toFixed(3))}
                      onChange={(event) => {
                        const nextEuler = [...euler] as Vec3;
                        nextEuler[index] = Number(event.currentTarget.value);
                        patchConnector({
                          frame: {
                            ...frame3d,
                            orientation_quat_xyzw: eulerDegToQuat(nextEuler),
                          },
                        });
                      }}
                    />
                  </label>
                ))}
              </div>
              <h3>Orientation (Quat XYZW)</h3>
              <div className="numeric-grid">
                {(["X", "Y", "Z", "W"] as const).map((label, index) => (
                  <label key={`quat-${label}`}>
                    {label}
                    <input
                      type="number"
                      step="0.01"
                      value={frame3d.orientation_quat_xyzw[index]}
                      onChange={(event) => {
                        const orientation_quat_xyzw = [
                          ...frame3d.orientation_quat_xyzw,
                        ] as [number, number, number, number];
                        orientation_quat_xyzw[index] = Number(
                          event.currentTarget.value,
                        );
                        patchConnector({
                          frame: {
                            ...frame3d,
                            orientation_quat_xyzw,
                          },
                        });
                      }}
                    />
                  </label>
                ))}
              </div>
              <div className="button-row">
                <button
                  type="button"
                  onClick={() =>
                    onStateChange(
                      snapConnectorToBoundsFace(
                        state,
                        selectedAsset.asset_id,
                        selectedConnector.connector_id,
                      ),
                    )
                  }
                >
                  Snap to nearest face
                </button>
                <button
                  type="button"
                  onClick={() =>
                    onStateChange(
                      duplicateConnector(
                        state,
                        selectedAsset.asset_id,
                        selectedConnector.connector_id,
                      ),
                    )
                  }
                >
                  Duplicate
                </button>
              </div>
            </>
          ) : frame2d ? (
            <>
              <h3>2D position</h3>
              <div className="numeric-grid">
                {(["X", "Y"] as const).map((label, index) => (
                  <label key={`2d-pos-${label}`}>
                    {label}
                    <input
                      type="number"
                      step="1"
                      value={frame2d.position[index]}
                      onChange={(event) => {
                        const position = [...frame2d.position] as [
                          number,
                          number,
                        ];
                        position[index] = Number(event.currentTarget.value);
                        patchConnector({
                          frame: { ...frame2d, position },
                        });
                      }}
                    />
                  </label>
                ))}
              </div>
              <h3>2D normal</h3>
              <div className="numeric-grid">
                {(["X", "Y"] as const).map((label, index) => (
                  <label key={`2d-n-${label}`}>
                    {label}
                    <input
                      type="number"
                      step="0.1"
                      value={frame2d.normal[index]}
                      onChange={(event) => {
                        const normal = [...frame2d.normal] as [number, number];
                        normal[index] = Number(event.currentTarget.value);
                        patchConnector({
                          frame: { ...frame2d, normal },
                        });
                      }}
                    />
                  </label>
                ))}
              </div>
              <label>
                Grid cell X
                <input
                  type="number"
                  step="1"
                  value={frame2d.grid_cell?.[0] ?? ""}
                  onChange={(event) => {
                    const y = frame2d.grid_cell?.[1] ?? 0;
                    const x = event.currentTarget.value;
                    patchConnector({
                      frame: {
                        ...frame2d,
                        grid_cell: x === "" ? null : [Number(x), y],
                      },
                    });
                  }}
                />
              </label>
              <label>
                Grid cell Y
                <input
                  type="number"
                  step="1"
                  value={frame2d.grid_cell?.[1] ?? ""}
                  onChange={(event) => {
                    const x = frame2d.grid_cell?.[0] ?? 0;
                    const y = event.currentTarget.value;
                    patchConnector({
                      frame: {
                        ...frame2d,
                        grid_cell: y === "" ? null : [x, Number(y)],
                      },
                    });
                  }}
                />
              </label>
              <button
                type="button"
                onClick={() =>
                  onStateChange(
                    duplicateConnector(
                      state,
                      selectedAsset.asset_id,
                      selectedConnector.connector_id,
                    ),
                  )
                }
              >
                Duplicate
              </button>
            </>
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
