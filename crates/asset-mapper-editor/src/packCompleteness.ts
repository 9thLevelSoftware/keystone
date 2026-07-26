import type { PackProvenance } from "./types";

/** Mirrors Rust `license_summary_is_production_ready`. */
export function licenseSummaryIsProductionReady(summary: string): boolean {
  const trimmed = summary.trim();
  return (
    trimmed.length > 0 && !trimmed.toUpperCase().startsWith("UNSPECIFIED")
  );
}

/** Mirrors Rust `PackProvenance::meets_production_requirements`. */
export function provenanceMeetsProduction(provenance: PackProvenance): boolean {
  const source = provenance.source?.trim() ?? "";
  const author = provenance.author?.trim() ?? "";
  return source.length > 0 || author.length > 0;
}
