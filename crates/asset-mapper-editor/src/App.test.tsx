import "@testing-library/jest-dom/vitest";

import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";

import App, { packReadinessKey } from "./App";
import type { PackRecord } from "./types";

describe("App", () => {
  it("renders the initial editor scaffold", () => {
    render(<App />);

    expect(
      screen.getByRole("heading", { name: "Asset Mapper" }),
    ).toBeInTheDocument();
    expect(
      screen.getByText(/No pack open\. Open or Init a pack, then Analyze/i),
    ).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Open" })).toBeEnabled();
    expect(screen.getByRole("button", { name: "Init" })).toBeEnabled();
    expect(screen.getByRole("button", { name: "Index" })).toBeDisabled();
    expect(screen.getByRole("button", { name: "Analyze" })).toBeDisabled();
    expect(screen.getByRole("button", { name: "Reload" })).toBeDisabled();
    expect(screen.getByRole("button", { name: "Discard" })).toBeDisabled();
    expect(screen.getByRole("button", { name: "Validate" })).toBeDisabled();
    expect(screen.getByRole("button", { name: "Save" })).toBeDisabled();
    expect(screen.getByRole("button", { name: "Export Bundle" })).toBeDisabled();
    expect(screen.getByText("Select an asset to preview.")).toBeInTheDocument();
    expect(
      screen.getByRole("region", { name: "Validation diagnostics" }),
    ).toHaveTextContent("No diagnostics.");
  });
});

describe("packReadinessKey", () => {
  const basePack = {
    assets: [
      {
        asset_id: "a",
        connectors: [{ connector_id: "c1", class: "wall_edge" }],
      },
    ],
    compatibility_rules: [{ a_class: "wall_edge", b_class: "wall_edge" }],
    connector_classes: [{ class: "wall_edge" }],
  } as unknown as PackRecord;

  it("changes when connectors change", () => {
    const before = packReadinessKey(basePack);
    const after = packReadinessKey({
      ...basePack,
      assets: [
        {
          ...basePack.assets[0],
          connectors: [],
        },
      ],
    });
    expect(before).not.toEqual(after);
  });

  it("is stable for selection-irrelevant clones", () => {
    const a = packReadinessKey(basePack);
    const b = packReadinessKey({ ...basePack });
    expect(a).toEqual(b);
  });
});
