import type { VibeReadinessReport } from "../tauriApi";

interface Props {
  report: VibeReadinessReport | null;
  onRefresh?: () => void;
  busy?: boolean;
}

/** Summary banner for vibe-builder readiness after open/analyze. */
export default function VibeReadinessBanner({ report, onRefresh, busy }: Props) {
  if (!report) {
    return (
      <div className="vibe-readiness-banner vibe-unknown">
        <strong>Vibe readiness</strong>
        <p className="muted">Not computed yet.</p>
        {onRefresh ? (
          <button type="button" disabled={busy} onClick={onRefresh}>
            Check readiness
          </button>
        ) : null}
      </div>
    );
  }

  const cls = report.ready ? "vibe-ready" : "vibe-not-ready";
  return (
    <div className={`vibe-readiness-banner ${cls}`} role="status">
      <div className="vibe-readiness-header">
        <strong>
          Vibe readiness: {report.score}/100 {report.ready ? "— ready" : "— not ready"}
        </strong>
        {onRefresh ? (
          <button type="button" disabled={busy} onClick={onRefresh}>
            Refresh
          </button>
        ) : null}
      </div>
      <p>
        Coverage {(report.coverage * 100).toFixed(0)}% · orphans{" "}
        {report.orphan_classes.length} · gaps {report.connectivity_gaps.length}
      </p>
      <ul>
        {report.checklist.map((item) => (
          <li key={item.id} className={item.ok ? "ok" : "fail"}>
            {item.ok ? "✓" : "✗"} {item.detail}
          </li>
        ))}
      </ul>
      {report.notes[0] ? <p className="muted">{report.notes[0]}</p> : null}
    </div>
  );
}
