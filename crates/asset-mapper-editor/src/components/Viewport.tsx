import { useEffect, useMemo, useRef, useState } from "react";

import { updateConnectorFrame } from "../editorState";
import { previewUrlForAsset } from "../three/assetUrls";
import { createAssetViewer, type AssetViewer } from "../three/createAssetViewer";
import type { AssetRecord, EditorPackState } from "../types";

interface ViewportProps {
  state: EditorPackState | null;
  selectedAsset: AssetRecord | null;
  onStateChange: (state: EditorPackState) => void;
}

export default function Viewport({
  state,
  selectedAsset,
  onStateChange,
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
  const previewUrl = previewUrlForAsset(selectedStatus);

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

  useEffect(() => {
    if (!selectedAsset) {
      viewerRef.current?.clear();
      setMessage("Select an asset to preview.");
      return;
    }

    if (!previewUrl) {
      viewerRef.current?.clear();
      setMessage("Preview unavailable for this asset.");
      return;
    }

    const viewer = ensureViewer();
    if (!viewer) {
      setMessage("Preview unavailable.");
      return;
    }

    let cancelled = false;
    setMessage("Loading preview...");
    viewer
      .loadAsset(previewUrl, selectedAsset)
      .then(() => {
        if (cancelled) {
          return;
        }

        viewer.setConnectors(selectedAsset.connectors);
        viewer.selectConnector(state?.selectedConnectorId ?? null);
        setMessage("");
      })
      .catch((error: unknown) => {
        if (cancelled) {
          return;
        }

        setMessage(error instanceof Error ? error.message : String(error));
      });

    return () => {
      cancelled = true;
    };
  }, [previewUrl, selectedAsset?.asset_id]);

  useEffect(() => {
    if (selectedAsset) {
      viewerRef.current?.setConnectors(selectedAsset.connectors);
    }
  }, [selectedAsset?.connectors, selectedAsset]);

  useEffect(() => {
    viewerRef.current?.selectConnector(state?.selectedConnectorId ?? null);
  }, [state?.selectedConnectorId]);

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
          updateConnectorFrame(currentState, currentState.selectedAssetId, connectorId, {
            position,
            orientation_quat_xyzw: orientation,
          }),
        );
      },
    );

    return viewerRef.current;
  }

  return (
    <section className="viewport-panel" aria-label="Asset preview">
      <div ref={containerRef} className="viewport-canvas" />
      {message ? <div className="viewport-placeholder">{message}</div> : null}
    </section>
  );
}
