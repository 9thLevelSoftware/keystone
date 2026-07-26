import "@testing-library/jest-dom/vitest";

import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";

import App from "./App";

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
