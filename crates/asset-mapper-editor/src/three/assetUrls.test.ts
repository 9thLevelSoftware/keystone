import { describe, expect, it } from "vitest";

import { canPreviewSourcePath, previewUrlForAsset } from "./assetUrls";

describe("asset preview URLs", () => {
  it("allows glb and gltf previews only", () => {
    expect(canPreviewSourcePath("wall.glb")).toBe(true);
    expect(canPreviewSourcePath("wall.gltf")).toBe(true);
    expect(canPreviewSourcePath("wall.GLB")).toBe(true);
    expect(canPreviewSourcePath("wall.obj")).toBe(false);
    expect(canPreviewSourcePath("sprite.png")).toBe(false);
  });

  it("returns null when the asset is missing or unsupported", () => {
    expect(
      previewUrlForAsset({
        assetId: "wall",
        sourcePath: "wall.obj",
        absolutePath: "C:/pack/wall.obj",
        exists: true,
        contentHash: "sha256:abc",
        hashMatches: true,
        previewSupported: false,
      }),
    ).toBeNull();

    expect(
      previewUrlForAsset({
        assetId: "wall",
        sourcePath: "wall.glb",
        absolutePath: "C:/pack/wall.glb",
        exists: false,
        contentHash: null,
        hashMatches: null,
        previewSupported: true,
      }),
    ).toBeNull();
  });
});
