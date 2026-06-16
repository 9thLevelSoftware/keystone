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
    expect(screen.getAllByText("No pack open.")).toHaveLength(2);
    expect(screen.getByRole("button", { name: "Open" })).toBeEnabled();
    expect(screen.getByRole("button", { name: "Init" })).toBeEnabled();
    expect(screen.getByRole("button", { name: "Index" })).toBeDisabled();
    expect(screen.getByRole("button", { name: "Validate" })).toBeDisabled();
    expect(screen.getByRole("button", { name: "Save" })).toBeDisabled();
    expect(screen.getByRole("button", { name: "Export Bundle" })).toBeDisabled();
    expect(screen.getByRole("region", { name: "Asset preview" })).toHaveTextContent(
      "Select an asset to preview.",
    );
    expect(
      screen.getByRole("region", { name: "Validation diagnostics" }),
    ).toHaveTextContent("No diagnostics.");
  });
});
