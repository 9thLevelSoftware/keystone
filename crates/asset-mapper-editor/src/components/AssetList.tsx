import type { EditorPackState } from "../types";

interface AssetListProps {
  busy: boolean;
  state: EditorPackState | null;
  onOpen: () => void;
  onInit: () => void;
  onIndex: () => void;
  onSelectAsset: (assetId: string) => void;
}

export default function AssetList({
  busy,
  state,
  onOpen,
  onInit,
  onIndex,
  onSelectAsset,
}: AssetListProps) {
  return (
    <aside className="asset-sidebar">
      <div className="toolbar">
        <button type="button" disabled={busy} onClick={onOpen}>
          Open
        </button>
        <button type="button" disabled={busy} onClick={onInit}>
          Init
        </button>
        <button type="button" disabled={busy || !state} onClick={onIndex}>
          Index
        </button>
      </div>
      {state ? (
        <>
          <h2>{state.pack.display_name}</h2>
          <p className="muted">{state.packRoot}</p>
          <ul className="asset-list">
            {state.pack.assets.map((asset) => {
              const status = state.assets.find(
                (candidate) => candidate.assetId === asset.asset_id,
              );
              return (
                <li key={asset.asset_id}>
                  <button
                    type="button"
                    className={
                      asset.asset_id === state.selectedAssetId
                        ? "asset-list-item selected"
                        : "asset-list-item"
                    }
                    onClick={() => onSelectAsset(asset.asset_id)}
                  >
                    <span>{asset.display_name}</span>
                    <span className="asset-meta">
                      {asset.connectors.length} connectors
                      {status?.hashMatches === false ? " / drifted" : ""}
                      {status?.exists === false ? " / missing" : ""}
                    </span>
                  </button>
                </li>
              );
            })}
          </ul>
        </>
      ) : (
        <p className="empty-state">No pack open.</p>
      )}
    </aside>
  );
}
