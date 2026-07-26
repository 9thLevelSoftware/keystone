import {
  licenseSummaryIsProductionReady,
  provenanceMeetsProduction,
} from "../packCompleteness";
import type { EditorPackState } from "../types";

interface Props {
  state: EditorPackState;
}

/** Shown when pack metadata would fail production validation gates. */
export default function PackCompletenessBanner({ state }: Props) {
  const issues: string[] = [];
  if (!licenseSummaryIsProductionReady(state.pack.license_summary)) {
    issues.push("Set a real license summary (not empty or UNSPECIFIED).");
  }
  if (!provenanceMeetsProduction(state.pack.provenance)) {
    issues.push("Set provenance source and/or author.");
  }

  if (issues.length === 0) {
    return null;
  }

  return (
    <div className="pack-completeness-banner" role="alert">
      <strong>Pack incomplete for production</strong>
      <p>
        Fill the Pack settings panel before export. Save is allowed with
        warnings; export is blocked while validation reports errors.
      </p>
      <ul>
        {issues.map((issue) => (
          <li key={issue}>{issue}</li>
        ))}
      </ul>
    </div>
  );
}
