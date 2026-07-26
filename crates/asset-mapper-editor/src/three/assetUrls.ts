import type { EditorAssetStatus } from "../types";
import { readPackAssetBytes } from "../tauriApi";

export function canPreviewSourcePath(sourcePath: string): boolean {
  const extension = sourcePath.split(".").pop()?.toLowerCase();
  return extension === "glb" || extension === "gltf";
}

/**
 * Load a previewable asset via Rust (safe path check) into a blob URL.
 * Prefer this over convertFileSrc so previews work without broad asset-protocol scope.
 */
export async function previewBlobUrlForAsset(
  packRoot: string,
  asset: EditorAssetStatus | null,
): Promise<{ url: string; revoke: () => void } | null> {
  if (!asset || !asset.exists || !asset.previewSupported) {
    return null;
  }
  if (!canPreviewSourcePath(asset.sourcePath)) {
    return null;
  }

  const bytes = await readPackAssetBytes(packRoot, asset.sourcePath);
  const data = bytes instanceof Uint8Array ? bytes : new Uint8Array(bytes);
  // Copy into a plain ArrayBuffer-backed view for Blob compatibility.
  const copy = new Uint8Array(data.byteLength);
  copy.set(data);
  const blob = new Blob([copy.buffer], {
    type: asset.sourcePath.toLowerCase().endsWith(".gltf")
      ? "model/gltf+json"
      : "model/gltf-binary",
  });
  const url = URL.createObjectURL(blob);
  return {
    url,
    revoke: () => URL.revokeObjectURL(url),
  };
}

/** @deprecated Prefer previewBlobUrlForAsset — kept for unit tests of eligibility. */
export function previewUrlForAsset(asset: EditorAssetStatus | null): string | null {
  if (!asset || !asset.exists || !asset.previewSupported) {
    return null;
  }
  if (!canPreviewSourcePath(asset.sourcePath)) {
    return null;
  }
  return asset.absolutePath;
}
