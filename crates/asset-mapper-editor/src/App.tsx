export default function App() {
  return (
    <main className="editor-shell">
      <aside className="asset-sidebar">
        <div className="toolbar">
          <button type="button">Open</button>
          <button type="button">Init</button>
          <button type="button">Index</button>
        </div>
        <p className="empty-state">No pack open.</p>
      </aside>
      <section className="viewport-panel" aria-label="Asset preview">
        <div className="viewport-placeholder">Select an asset to preview.</div>
      </section>
      <aside className="inspector-panel">
        <h1>Asset Mapper</h1>
        <p>Open a pack folder to start authoring connectors.</p>
      </aside>
      <section className="diagnostics-panel" aria-label="Validation diagnostics">
        <p>No diagnostics.</p>
      </section>
    </main>
  );
}
