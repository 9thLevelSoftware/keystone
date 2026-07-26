import { useEffect, useMemo, useRef, useState } from "react";

import { updateConnectorFrame } from "../editorState";
import { previewBlobUrlForAsset } from "../three/assetUrls";
import { createAssetViewer, type AssetViewer } from "../three/createAssetViewer";
import type { AssetRecord, EditorPackState, ResolvedScene } from "../types";

interface ViewportProps {
  state: EditorPackState | null;
  selectedAsset: AssetRecord | null;
  onStateChange: (state: EditorPackState) => void;
  /** When set, show multi-asset assembly instead of single-asset authoring. */
  assemblyScene?: ResolvedScene | null;
  assemblyPackRoot?: string | null;
}

export default function Viewport({
  state,
  selectedAsset,
  onStateChange,
  assemblyScene = null,
  assemblyPackRoot = null,
}: ViewportProps) {
  const containerRef = useRef<HTMLDivElement | null>(null);
  const viewerRef = useRef<AssetViewer | null>(null);
  const stateRef = useRef<EditorPackState | null>(state);
  const [message, setMessage] = useState("Select an asset to preview.");

  const selectedStatus = useMemo(
    () =>
      state?.assets.find((asset) => asset.assetId === selectedAsset?.asset_id) ??
      null,
    [selectedAsset?.asset_id, state?.assets],
  );

  useEffect(() => {
    stateRef.current = state;
  }, [state]);

  useEffect(
    () => () => {
      viewerRef.current?.dispose();
      viewerRef.current = null;
    },
    [],
  );

  // Assembly multi-asset preview
  useEffect(() => {
    if (!assemblyScene || !state || !assemblyPackRoot) {
      return;
    }

    const viewer = ensureViewer();
    if (!viewer) {
      setMessage("Preview unavailable.");
      return;
    }

    let cancelled = false;
    let revokes: Array<() => void> = [];
    setMessage("Loading assembly preview...");

    (async () => {
      try {
        const loads: Array<{
          url: string;
          asset: AssetRecord;
          translation: [number, number, number];
          rotation: [number, number, number, number];
        }> = [];

        for (const placement of assemblyScene.placements) {
          const asset = state.pack.assets.find(
            (a) => a.asset_id === placement.asset_id,
          );
          const status = state.assets.find(
            (a) => a.assetId === placement.asset_id,
          );
          if (!asset || !status) {
            continue;
          }
          const blob = await previewBlobUrlForAsset(assemblyPackRoot, status);
          if (!blob) {
            continue;
          }
          revokes.push(blob.revoke);
          loads.push({
            url: blob.url,
            asset,
            translation: placement.transform.translation,
            rotation: placement.transform.rotation_quat_xyzw,
          });
        }

        if (cancelled) {
          return;
        }

        await viewer.loadAssembly(loads);
        setMessage(
          loads.length === 0
            ? "Assembly preview: no loadable assets."
            : `Assembly preview (${loads.length} piece(s)).`,
        );
      } catch (error: unknown) {
        if (!cancelled) {
          setMessage(error instanceof Error ? error.message : String(error));
        }
      }
    })();

    return () => {
      cancelled = true;
      for (const revoke of revokes) {
        revoke();
      }
    };
  }, [assemblyScene, assemblyPackRoot, state?.packRoot]);

  // Single-asset authoring preview
  useEffect(() => {
    if (assemblyScene) {
      return;
    }

    if (!selectedAsset || !state) {
      viewerRef.current?.clear();
      setMessage("Select an asset to preview.");
      return;
    }

    if (!selectedStatus?.previewSupported) {
      viewerRef.current?.clear();
      setMessage(
        selectedStatus?.exists === false
          ? "Source file missing — cannot preview."
          : "Preview only supports .glb / .gltf. Convert other formats or use measure bounds.",
      );
      return;
    }

    const viewer = ensureViewer();
    if (!viewer) {
      setMessage("Preview unavailable.");
      return;
    }

    let cancelled = false;
    let revoke: (() => void) | null = null;
    setMessage("Loading preview...");

    previewBlobUrlForAsset(state.packRoot, selectedStatus)
      .then(async (blob) => {
        if (cancelled) {
          return;
        }
        if (!blob) {
          setMessage("Could not load asset bytes for preview.");
          return;
        }
        revoke = blob.revoke;
        await viewer.loadAsset(blob.url, selectedAsset);
        if (cancelled) {
          return;
        }
        viewer.setConnectors(selectedAsset.connectors);
        viewer.selectConnector(state.selectedConnectorId ?? null);
        setMessage("");
      })
      .catch((error: unknown) => {
        if (!cancelled) {
          setMessage(
            error instanceof Error
              ? `Preview failed: ${error.message}`
              : `Preview failed: ${String(error)}`,
          );
        }
      });

    return () => {
      cancelled = true;
      revoke?.();
    };
  }, [
    assemblyScene,
    selectedAsset?.asset_id,
    selectedStatus?.absolutePath,
    selectedAsset?.connectors,
    state?.packRoot,
    state?.selectedConnectorId,
  ]);

  useEffect(() => {
    if (assemblyScene || !selectedAsset) {
      return;
    }
    viewerRef.current?.setConnectors(selectedAsset.connectors);
  }, [assemblyScene, selectedAsset?.connectors, selectedAsset]);

  useEffect(() => {
    if (assemblyScene) {
      return;
    }
    viewerRef.current?.selectConnector(state?.selectedConnectorId ?? null);
  }, [assemblyScene, state?.selectedConnectorId]);

  function ensureViewer(): AssetViewer | null {
    if (viewerRef.current) {
      return viewerRef.current;
    }

    const container = containerRef.current;
    if (!container) {
      return null;
    }

    viewerRef.current = createAssetViewer(
      container,
      (connectorId, position, orientation) => {
        const currentState = stateRef.current;
        if (!currentState?.selectedAssetId) {
          return;
        }

        onStateChange(
          updateConnectorFrame(
            currentState,
            currentState.selectedAssetId,
            connectorId,
            {
              position,
              orientation_quat_xyzw: orientation,
            },
          ),
        );
      },
    );
    return viewerRef.current;
  }

  return (
    <section className="viewport-panel">
      <div ref={containerRef} className="viewport-canvas" />
      {message ? <p className="viewport-message">{message}</p> : null}
    </section>
  );
}
