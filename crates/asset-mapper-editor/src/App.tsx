import { useMemo, useState } from "react";

import AssetList from "./components/AssetList";
import DiagnosticsPanel from "./components/DiagnosticsPanel";
import Inspector from "./components/Inspector";
import Viewport from "./components/Viewport";
import { selectAsset, selectConnector } from "./editorState";
import {
  acceptHashDrift,
  chooseBundleOutputPath,
  choosePackFolder,
  exportBundle,
  indexPackFolder,
  initPackFolder,
  measurePackBounds,
  openPackFolder,
  savePack,
  validatePack,
} from "./tauriApi";
import type { EditorCommandError, EditorPackState } from "./types";

function errorMessage(error: unknown): string {
  if (typeof error === "object" && error && "message" in error) {
    return String((error as EditorCommandError).message);
  }

  return String(error);
}

export default function App() {
  const [state, setState] = useState<EditorPackState | null>(null);
  const [status, setStatus] = useState("No pack open.");
  const [busy, setBusy] = useState(false);

  const selectedAsset = useMemo(
    () =>
      state?.pack.assets.find((asset) => asset.asset_id === state.selectedAssetId) ??
      null,
    [state],
  );
  const selectedConnector = useMemo(
    () =>
      selectedAsset?.connectors.find(
        (connector) => connector.connector_id === state?.selectedConnectorId,
      ) ?? null,
    [selectedAsset, state?.selectedConnectorId],
  );

  function confirmIfDirty(): boolean {
    if (!state?.dirty) {
      return true;
    }
    return window.confirm(
      "You have unsaved changes. Continue and discard local edits?",
    );
  }

  async function runAction(label: string, action: () => Promise<void>) {
    setBusy(true);
    setStatus(`${label}...`);

    try {
      await action();
    } catch (error) {
      setStatus(errorMessage(error));
    } finally {
      setBusy(false);
    }
  }

  return (
    <main className="editor-shell">
      <AssetList
        busy={busy}
        state={state}
        onOpen={() =>
          runAction("Opening pack", async () => {
            if (!confirmIfDirty()) {
              setStatus("Open cancelled.");
              return;
            }
            const folder = await choosePackFolder();
            if (!folder) {
              setStatus("Open cancelled.");
              return;
            }

            const opened = await openPackFolder(folder);
            setState(opened);
            setStatus(`Opened ${opened.pack.display_name}.`);
          })
        }
        onInit={() =>
          runAction("Initializing pack", async () => {
            if (!confirmIfDirty()) {
              setStatus("Init cancelled.");
              return;
            }
            const folder = await choosePackFolder();
            if (!folder) {
              setStatus("Init cancelled.");
              return;
            }

            const displayName = window.prompt("Pack name", "New Asset Pack");
            if (!displayName) {
              setStatus("Init cancelled.");
              return;
            }

            const initialized = await initPackFolder(folder, displayName);
            setState(initialized);
            setStatus(`Initialized ${initialized.pack.display_name}.`);
          })
        }
        onIndex={() =>
          runAction("Indexing pack", async () => {
            if (!state) {
              return;
            }
            if (!confirmIfDirty()) {
              setStatus("Index cancelled.");
              return;
            }

            const result = await indexPackFolder(state.packRoot);
            setState(result.state);
            setStatus(
              `Indexed pack: ${result.report.new_assets.length} new, ${result.report.drifted_assets.length} drifted.`,
            );
          })
        }
        onReload={() =>
          runAction("Reloading pack", async () => {
            if (!state) {
              return;
            }
            if (!confirmIfDirty()) {
              setStatus("Reload cancelled.");
              return;
            }
            const opened = await openPackFolder(state.packRoot);
            setState(opened);
            setStatus(`Reloaded ${opened.pack.display_name}.`);
          })
        }
        onDiscard={() =>
          runAction("Discarding changes", async () => {
            if (!state) {
              return;
            }
            if (!window.confirm("Discard all unsaved changes?")) {
              setStatus("Discard cancelled.");
              return;
            }
            const opened = await openPackFolder(state.packRoot);
            setState(opened);
            setStatus("Discarded local changes.");
          })
        }
        onSelectAsset={(assetId) => {
          if (state) {
            setState(selectAsset(state, assetId));
          }
        }}
      />
      <Viewport state={state} selectedAsset={selectedAsset} onStateChange={setState} />
      <Inspector
        state={state}
        selectedAsset={selectedAsset}
        selectedConnector={selectedConnector}
        onStateChange={setState}
        onSelectConnector={(assetId, connectorId) => {
          if (state) {
            setState(selectConnector(state, assetId, connectorId));
          }
        }}
        onMeasureBounds={() =>
          runAction("Measuring bounds", async () => {
            if (!state) {
              return;
            }
            // Disk-backed measure reloads from sidecar — refuse silent discard.
            if (!confirmIfDirty()) {
              setStatus("Measure cancelled (unsaved changes kept).");
              return;
            }
            const result = await measurePackBounds(state.packRoot);
            setState({
              ...result.state,
              selectedAssetId: state.selectedAssetId,
              selectedConnectorId: state.selectedConnectorId,
            });
            setStatus(
              `Measured ${result.report.measured.length} asset(s); ` +
                `${result.report.failed.length} failed; ` +
                `${result.report.missing.length} missing.`,
            );
          })
        }
        onAcceptDrift={() =>
          runAction("Accepting hash drift", async () => {
            if (!state || !selectedAsset) {
              return;
            }
            // Disk-backed accept-drift reloads from sidecar — refuse silent discard.
            if (!confirmIfDirty()) {
              setStatus("Accept drift cancelled (unsaved changes kept).");
              return;
            }
            const result = await acceptHashDrift(state.packRoot, [
              selectedAsset.asset_id,
            ]);
            setState({
              ...result.state,
              selectedAssetId: state.selectedAssetId,
              selectedConnectorId: state.selectedConnectorId,
            });
            setStatus(`Accepted hash drift for ${selectedAsset.asset_id}.`);
          })
        }
      />
      <DiagnosticsPanel
        state={state}
        status={status}
        busy={busy}
        onStateChange={setState}
        onValidate={() =>
          runAction("Validating pack", async () => {
            if (!state) {
              return;
            }

            const validation = await validatePack(state);
            setState({ ...state, validation });
            setStatus(`Validation returned ${validation.diagnostics.length} diagnostics.`);
          })
        }
        onSave={() =>
          runAction("Saving pack", async () => {
            if (!state) {
              return;
            }

            const result = await savePack(state);
            setState(result.state);
            setStatus("Saved sidecar.");
          })
        }
        onExport={() =>
          runAction("Exporting bundle", async () => {
            if (!state) {
              return;
            }

            const outputPath = await chooseBundleOutputPath();
            if (!outputPath) {
              setStatus("Export cancelled.");
              return;
            }

            const result = await exportBundle(state, outputPath);
            setStatus(`Exported ${result.outputPath}.`);
          })
        }
      />
    </main>
  );
}
