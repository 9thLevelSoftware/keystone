import { selectDiagnosticTarget } from "../editorState";
import type { Diagnostic, EditorPackState } from "../types";

interface DiagnosticsPanelProps {
  state: EditorPackState | null;
  status: string;
  busy: boolean;
  onValidate: () => void;
  onSave: () => void;
  onExport: () => void;
  onStateChange: (state: EditorPackState) => void;
}

export default function DiagnosticsPanel({
  state,
  status,
  busy,
  onValidate,
  onSave,
  onExport,
  onStateChange,
}: DiagnosticsPanelProps) {
  const diagnostics = state?.validation.diagnostics ?? [];

  function onDiagnosticClick(diagnostic: Diagnostic) {
    if (!state) {
      return;
    }
    onStateChange(selectDiagnosticTarget(state, diagnostic));
  }

  return (
    <section className="diagnostics-panel" aria-label="Validation diagnostics">
      <div className="diagnostics-toolbar">
        <p>{status}</p>
        <div>
          <button type="button" disabled={busy || !state} onClick={onValidate}>
            Validate
          </button>
          <button type="button" disabled={busy || !state || !state.dirty} onClick={onSave}>
            Save
          </button>
          <button type="button" disabled={busy || !state} onClick={onExport}>
            Export Bundle
          </button>
        </div>
      </div>
      {diagnostics.length === 0 ? (
        <p className="muted">No diagnostics.</p>
      ) : (
        <ul className="diagnostics-list">
          {diagnostics.map((diagnostic, index) => (
            <li key={`${diagnostic.code}-${index}`} className={diagnostic.severity}>
              <button
                type="button"
                className="diagnostic-item"
                onClick={() => onDiagnosticClick(diagnostic)}
              >
                <strong>{diagnostic.code}</strong>
                <span>{diagnostic.message}</span>
                {diagnostic.asset_id ? (
                  <span className="muted small">
                    {diagnostic.asset_id}
                    {diagnostic.connector_id
                      ? ` / ${diagnostic.connector_id}`
                      : ""}
                  </span>
                ) : null}
              </button>
            </li>
          ))}
        </ul>
      )}
    </section>
  );
}
