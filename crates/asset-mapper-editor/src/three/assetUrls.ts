import { convertFileSrc } from "@tauri-apps/api/core";

import type { EditorAssetStatus } from "../types";

export function canPreviewSourcePath(sourcePath: string): boolean {
  const extension = sourcePath.split(".").pop()?.toLowerCase();
  return extension === "glb" || extension === "gltf";
}

export function previewUrlForAsset(asset: EditorAssetStatus | null): string | null {
  if (!asset || !asset.exists || !asset.previewSupported) {
    return null;
  }

  return convertFileSrc(asset.absolutePath);
}
