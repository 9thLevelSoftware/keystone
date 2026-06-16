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
    expect(screen.getByText("No pack open.")).toBeInTheDocument();
    expect(screen.getByRole("region", { name: "Asset preview" })).toHaveTextContent(
      "Select an asset to preview.",
    );
    expect(
      screen.getByRole("region", { name: "Validation diagnostics" }),
    ).toHaveTextContent("No diagnostics.");
  });
});
