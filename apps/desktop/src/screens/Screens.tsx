// Remaining screens: Home, Results, Comparison, Graphics, Tables, Notes,
// Registry, Settings, License, About. All compose the same primitives.
import { useEffect, useMemo, useState } from "react";
import { replayProject, runCommand, type ExecOutput, type ReproducibilityReport } from "../bridge";
import { EstimateDocument, CompareView } from "../components/ResultViews";
import { NxButton, NxEmpty, NxPill, NxSectionLabel, NxSegmented, formatInt } from "../components/Nx";
import { HistogramChart, ScatterChart } from "../components/Charts";
import { useApp } from "../state/store";

/* ---------------- Home / Project ---------------- */
export function HomeScreen() {
  const projectTitle = useApp((s) => s.projectTitle);
  const estimates = useApp((s) => s.estimates);
  const dataLoaded = useApp((s) => s.dataLoaded);
  const nRows = useApp((s) => s.nRows);
  const variables = useApp((s) => s.variables);
  const setScreen = useApp((s) => s.setScreen);
  const recent = estimates.slice(-5).reverse();

  return (
    <div style={{ padding: "var(--space-3xl)", maxWidth: 760 }}>
      <h2 style={{ margin: 0, fontSize: 20, fontWeight: 700 }}>{projectTitle ?? "Untitled workspace"}</h2>
      <p style={{ color: "var(--color-text-secondary)", fontSize: 13, marginTop: 4 }}>
        Professional statistical software. Built locally. Computed deterministically.
      </p>

      <NxSectionLabel>Workspace</NxSectionLabel>
      <div className="nx-fitstats" style={{ gridTemplateColumns: "repeat(auto-fill,minmax(160px,1fr))" }}>
        <Cell label="Datasets" value={dataLoaded ? "1" : "0"} />
        <Cell label="Observations" value={dataLoaded ? formatInt(nRows) : "0"} />
        <Cell label="Variables" value={dataLoaded ? String(variables.length) : "0"} />
        <Cell label="Analyses" value={String(estimates.length)} />
      </div>

      <NxSectionLabel>Recent analyses</NxSectionLabel>
      {recent.length === 0 ? (
        <p style={{ color: "var(--color-text-tertiary)", fontSize: 13 }}>
          No analyses yet. Open the Analysis workspace or the Console to estimate your first model.
        </p>
      ) : (
        <div className="nx-panel">
          {recent.map((e) => (
            <div key={e.id} style={{ display: "flex", gap: "var(--space-md)", padding: "var(--space-sm) var(--space-md)", borderBottom: "var(--border-subtle)", alignItems: "center" }}>
              <span style={{ fontFamily: "var(--font-mono)", fontSize: 12, color: "var(--color-text-tertiary)" }}>{e.id}</span>
              <span style={{ fontSize: 13, fontWeight: 600 }}>{e.label}</span>
              <NxPill tone="accent">{e.diagnostics.length} diagnostics</NxPill>
            </div>
          ))}
        </div>
      )}

      <NxSectionLabel>Quick start</NxSectionLabel>
      <div style={{ display: "flex", gap: "var(--space-sm)", flexWrap: "wrap" }}>
        <NxButton onClick={() => setScreen("data")}>Open Data</NxButton>
        <NxButton onClick={() => setScreen("analysis")}>New Analysis</NxButton>
        <NxButton onClick={() => setScreen("console")}>Command Console</NxButton>
        <NxButton onClick={() => setScreen("registry")}>Research Registry</NxButton>
      </div>
    </div>
  );
}

function Cell({ label, value }: { label: string; value: string }) {
  return (
    <div className="nx-fitstats-cell">
      <div className="nx-fitstats-label">{label}</div>
      <div className="nx-fitstats-value">{value}</div>
    </div>
  );
}

/* ---------------- Results ---------------- */
export function ResultsScreen() {
  const estimates = useApp((s) => s.estimates);
  const [active, setActive] = useState<string | null>(null);
  const current = useMemo(() => {
    if (estimates.length === 0) return null;
    return estimates.find((e) => e.id === (active ?? estimates[estimates.length - 1].id)) ?? estimates[estimates.length - 1];
  }, [estimates, active]);

  if (estimates.length === 0) {
    return (
      <div style={{ height: "100%", display: "grid", placeItems: "center" }}>
        <NxEmpty title="No results yet" hint="Run a model from the Analysis workspace or the Console; results appear here." />
      </div>
    );
  }

  return (
    <div style={{ display: "grid", gridTemplateColumns: "200px 1fr", height: "100%", overflow: "hidden" }}>
      <aside style={{ borderRight: "var(--border-subtle)", background: "var(--color-surface-secondary)", overflowY: "auto", padding: "var(--space-sm)" }}>
        {estimates
          .slice()
          .reverse()
          .map((e) => (
            <button
              key={e.id}
              className="nx-nav-item"
              data-active={(current?.id ?? "") === e.id}
              onClick={() => setActive(e.id)}
            >
              <span style={{ fontFamily: "var(--font-mono)", fontSize: 11, color: "var(--color-text-tertiary)" }}>{e.id}</span>
              {e.label}
            </button>
          ))}
      </aside>
      <div style={{ overflowY: "auto", padding: "var(--space-2xl)", display: "flex", justifyContent: "center", alignItems: "flex-start" }}>
        {current && "Estimate" in current.result ? (
          <EstimateDocument
            id={current.result.Estimate.id}
            label={current.result.Estimate.label}
            result={current.result.Estimate.result}
            diagnostics={current.diagnostics}
            kind={Object.keys(current.result.Estimate.result)[0]}
          />
        ) : null}
      </div>
    </div>
  );
}

/* ---------------- Model comparison ---------------- */
export function ComparisonScreen() {
  const estimates = useApp((s) => s.estimates);
  const [selected, setSelected] = useState<string[]>([]);
  const [compare, setCompare] = useState<ExecOutput | null>(null);

  if (estimates.length === 0) {
    return (
      <div style={{ height: "100%", display: "grid", placeItems: "center" }}>
        <NxEmpty title="Nothing to compare" hint="Estimate at least two models, then compare coefficients, standard errors and fit statistics side by side." />
      </div>
    );
  }

  return (
    <div style={{ padding: "var(--space-2xl)", maxWidth: 1000, overflowY: "auto", height: "100%" }}>
      <div style={{ display: "flex", gap: "var(--space-lg)", alignItems: "center", marginBottom: "var(--space-xl)" }}>
        <h2 style={{ margin: 0, fontSize: 17, fontWeight: 700 }}>Model comparison</h2>
        <NxButton
          variant="primary"
          onClick={async () => {
            const ids = selected.length > 1 ? selected : estimates.map((e) => e.id);
            const out = await runCommand(`estimate compare ${ids.join(" ")}`);
            setCompare(out);
          }}
        >
          Compare {selected.length > 1 ? selected.length : "all"}
        </NxButton>
      </div>
      <div className="nx-panel" style={{ marginBottom: "var(--space-xl)", maxWidth: 480 }}>
        <table className="nx-table">
          <thead>
            <tr>
              <th>Select</th>
              <th>ID</th>
              <th>Model</th>
            </tr>
          </thead>
          <tbody>
            {estimates.map((e) => (
              <tr key={e.id} data-selected={selected.includes(e.id)}>
                <td>
                  <input
                    type="checkbox"
                    checked={selected.includes(e.id)}
                    onChange={(ev) =>
                      setSelected((sel) => (ev.target.checked ? [...sel, e.id] : sel.filter((s) => s !== e.id)))
                    }
                  />
                </td>
                <td>{e.id}</td>
                <td>{e.label}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
      {compare && "Compare" in compare ? <CompareView r={compare.Compare} /> : null}
    </div>
  );
}

/* ---------------- Graphics ---------------- */
export function GraphicsScreen() {
  const variables = useApp((s) => s.variables);
  const [xVar, setXVar] = useState("");
  const [yVar, setYVar] = useState("");
  const [chart, setChart] = useState<"scatter" | "histogram">("scatter");
  const dataLoaded = useApp((s) => s.dataLoaded);
  const [columns, setColumns] = useState<{ Numeric: (number | null)[] }[]>([]);

  useEffect(() => {
    if (!dataLoaded) return;
    datasetColumns().then(setColumns);
  }, [dataLoaded, variables]);

  if (!dataLoaded) {
    return (
      <div style={{ height: "100%", display: "grid", placeItems: "center" }}>
        <NxEmpty title="No dataset" hint="Import data to create figures." />
      </div>
    );
  }

  const numeric = variables.filter((v) => v.storage === "Numeric").map((v) => v.name);
  const xi = numeric.indexOf(xVar);
  const yi = numeric.indexOf(yVar);
  const xCol = columns[xi]?.Numeric ?? [];
  const yCol = columns[yi]?.Numeric ?? [];

  return (
    <div style={{ padding: "var(--space-2xl)", overflowY: "auto", height: "100%" }}>
      <h2 style={{ margin: "0 0 var(--space-lg)", fontSize: 17, fontWeight: 700 }}>Graphics</h2>
      <div style={{ display: "flex", gap: "var(--space-lg)", marginBottom: "var(--space-lg)", flexWrap: "wrap", alignItems: "flex-end" }}>
        <NxSegmented
          value={chart}
          onChange={(v) => setChart(v as "scatter" | "histogram")}
          options={[
            { value: "scatter", label: "Scatter" },
            { value: "histogram", label: "Histogram" },
          ]}
        />
        {chart === "scatter" ? (
          <>
            <label style={{ fontSize: 12, color: "var(--color-text-secondary)" }}>
              Y
              <select className="nx-select" value={yVar} onChange={(e) => setYVar(e.target.value)} style={{ marginLeft: 6 }}>
                <option value="">Select…</option>
                {numeric.map((v) => <option key={v}>{v}</option>)}
              </select>
            </label>
            <label style={{ fontSize: 12, color: "var(--color-text-secondary)" }}>
              X
              <select className="nx-select" value={xVar} onChange={(e) => setXVar(e.target.value)} style={{ marginLeft: 6 }}>
                <option value="">Select…</option>
                {numeric.map((v) => <option key={v}>{v}</option>)}
              </select>
            </label>
          </>
        ) : (
          <label style={{ fontSize: 12, color: "var(--color-text-secondary)" }}>
            Variable
            <select className="nx-select" value={xVar} onChange={(e) => setXVar(e.target.value)} style={{ marginLeft: 6 }}>
              <option value="">Select…</option>
              {numeric.map((v) => <option key={v}>{v}</option>)}
            </select>
          </label>
        )}
      </div>
      <div className="nx-panel" style={{ padding: "var(--space-lg)", display: "grid", placeItems: "center" }}>
        {chart === "scatter" && xVar && yVar ? (
          <ScatterChart x={xCol.filter((v) => v !== null)} y={yCol.filter((v) => v !== null)} xLabel={xVar} yLabel={yVar} width={640} height={420} />
        ) : chart === "histogram" && xVar ? (
          <HistogramChart values={xCol.filter((v) => v !== null)} xLabel={xVar} width={640} height={380} />
        ) : (
          <NxEmpty title="Choose variables" hint="Select a variable pair to draw a publication-quality figure." />
        )}
      </div>
    </div>
  );
}

async function datasetColumns(): Promise<{ Numeric: (number | null)[] }[]> {
  const { datasetPreview } = await import("../bridge");
  const p = await datasetPreview(0, 5000);
  return (p.rows[0] ?? []).map((c) => ("Numeric" in c ? { Numeric: c.Numeric } : { Numeric: [] }));
}

/* ---------------- Tables (export) ---------------- */
export function TablesScreen() {
  const estimates = useApp((s) => s.estimates);
  const [format, setFormat] = useState("md");
  const [path, setPath] = useState("comparison.md");
  const [status, setStatus] = useState("");
  if (estimates.length === 0) {
    return (
      <div style={{ height: "100%", display: "grid", placeItems: "center" }}>
        <NxEmpty title="No stored estimates" hint="Estimate a model first, then export publication tables." />
      </div>
    );
  }
  return (
    <div style={{ padding: "var(--space-2xl)", maxWidth: 560 }}>
      <h2 style={{ margin: "0 0 var(--space-lg)", fontSize: 17, fontWeight: 700 }}>Publication tables</h2>
      <div style={{ display: "flex", flexDirection: "column", gap: "var(--space-lg)" }}>
        <NxSegmented
          value={format}
          onChange={setFormat}
          options={[
            { value: "md", label: "Markdown" },
            { value: "csv", label: "CSV" },
            { value: "tex", label: "LaTeX" },
            { value: "html", label: "HTML" },
          ]}
        />
        <label className="nx-field">
          File name
          <input className="nx-input" value={path} onChange={(e) => setPath(e.target.value)} />
        </label>
        <NxButton
          variant="primary"
          onClick={async () => {
            const out = await runCommand(`export table "${path}"`);
            setStatus("Message" in out ? out.Message : "Exported.");
          }}
        >
          Export {estimates.length} model{estimates.length > 1 ? "s" : ""}
        </NxButton>
        {status ? <p style={{ color: "var(--color-text-secondary)", fontSize: 13 }}>{status}</p> : null}
      </div>
    </div>
  );
}

/* ---------------- Notes ---------------- */
export function NotesScreen() {
  const notes = useApp((s) => s.notes);
  const [text, setText] = useState("");
  return (
    <div style={{ padding: "var(--space-2xl)", maxWidth: 640, height: "100%", overflowY: "auto" }}>
      <h2 style={{ margin: "0 0 var(--space-lg)", fontSize: 17, fontWeight: 700 }}>Research notes</h2>
      <div style={{ display: "flex", flexDirection: "column", gap: "var(--space-md)" }}>
        <textarea
          className="nx-textarea"
          rows={4}
          placeholder="Record a thought, decision or caveat for this project…"
          value={text}
          onChange={(e) => setText(e.target.value)}
        />
        <NxButton
          variant="primary"
          style={{ alignSelf: "flex-start" }}
          onClick={async () => {
            if (!text.trim()) return;
            await runCommand(`note "${text.replace(/"/g, "'")}"`);
            setText("");
          }}
        >
          Record note
        </NxButton>
        {notes.length === 0 ? (
          <p style={{ color: "var(--color-text-tertiary)", fontSize: 13 }}>No notes yet.</p>
        ) : (
          notes
            .slice()
            .reverse()
            .map((n, i) => (
              <div key={i} className="nx-panel" style={{ padding: "var(--space-md) var(--space-lg)", fontSize: 13 }}>
                {n}
              </div>
            ))
        )}
      </div>
    </div>
  );
}

/* ---------------- Registry / Reproduction ---------------- */
export function RegistryScreen() {
  const estimates = useApp((s) => s.estimates);
  const [report, setReport] = useState<ReproducibilityReport | null>(null);
  const [busy, setBusy] = useState(false);
  return (
    <div style={{ padding: "var(--space-2xl)", maxWidth: 860, height: "100%", overflowY: "auto" }}>
      <div style={{ display: "flex", alignItems: "center", gap: "var(--space-lg)", marginBottom: "var(--space-lg)" }}>
        <h2 style={{ margin: 0, fontSize: 17, fontWeight: 700 }}>Research registry</h2>
        <NxButton
          variant="primary"
          disabled={busy}
          onClick={async () => {
            setBusy(true);
            try {
              setReport(await replayProject());
            } catch (e) {
              setReport(null);
            } finally {
              setBusy(false);
            }
          }}
        >
          {busy ? "Replaying…" : "Re-run project (replay)"}
        </NxButton>
      </div>
      {report ? (
        <div className="nx-panel" style={{ padding: "var(--space-lg)", marginBottom: "var(--space-lg)" }}>
          <div style={{ display: "flex", gap: "var(--space-lg)", alignItems: "center" }}>
            <NxPill tone={report.all_reproduced ? "success" : "warning"}>
              {report.all_reproduced ? "All results reproduced" : `${report.failed} difference(s)`}
            </NxPill>
            <span style={{ fontSize: 12, color: "var(--color-text-secondary)" }}>
              {report.commands_replayed} commands replayed · Numeris {report.software_version}
            </span>
          </div>
          {report.steps.map((s) => (
            <div key={s.seq} className="nx-diag" data-status={s.status === "reproduced" ? "ok" : s.status === "error" ? "attention" : "info"}>
              <span className="nx-diag-status">{s.status === "reproduced" ? "✓" : s.status === "error" ? "✕" : "i"}</span>
              <span className="nx-diag-name" style={{ fontFamily: "var(--font-mono)", fontSize: 12 }}>{s.command}</span>
              <span className="nx-diag-detail">{s.detail}</span>
            </div>
          ))}
        </div>
      ) : null}
      {estimates.length === 0 ? (
        <NxEmpty title="No analyses recorded" hint="Every executed model is recorded here with its command, specification, software version and result." />
      ) : (
        <div className="nx-panel">
          <table className="nx-table">
            <thead>
              <tr>
                <th>#</th>
                <th>ID</th>
                <th>Model</th>
              </tr>
            </thead>
            <tbody>
              {estimates.map((e, i) => (
                <tr key={e.id}>
                  <td>{i + 1}</td>
                  <td>{e.id}</td>
                  <td>{e.label}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}
    </div>
  );
}

/* ---------------- Settings ---------------- */
export function SettingsScreen() {
  const theme = useApp((s) => s.theme);
  const toggleTheme = useApp((s) => s.toggleTheme);
  return (
    <div style={{ padding: "var(--space-2xl)", maxWidth: 560 }}>
      <h2 style={{ margin: "0 0 var(--space-xl)", fontSize: 17, fontWeight: 700 }}>Settings</h2>
      <NxSectionLabel>Appearance</NxSectionLabel>
      <NxSegmented
        value={theme}
        onChange={() => toggleTheme()}
        options={[
          { value: "light", label: "Light" },
          { value: "dark", label: "Dark" },
        ]}
      />
      <NxSectionLabel>Privacy</NxSectionLabel>
      <div style={{ display: "flex", flexDirection: "column", gap: "var(--space-sm)", fontSize: 13, color: "var(--color-text-secondary)" }}>
        <div style={{ display: "flex", justifyContent: "space-between" }}>
          <span>Local computation</span>
          <NxPill tone="success">Enabled</NxPill>
        </div>
        <div style={{ display: "flex", justifyContent: "space-between" }}>
          <span>Telemetry</span>
          <NxPill>Off</NxPill>
        </div>
        <div style={{ display: "flex", justifyContent: "space-between" }}>
          <span>Update checks</span>
          <NxPill>Off</NxPill>
        </div>
        <p className="nx-help" style={{ marginTop: "var(--space-sm)" }}>
          Your datasets are processed on this device. Numeris collects nothing.
        </p>
      </div>
    </div>
  );
}

/* ---------------- License ---------------- */
export function LicenseScreen() {
  return (
    <div style={{ padding: "var(--space-2xl)", maxWidth: 560 }}>
      <h2 style={{ margin: "0 0 var(--space-lg)", fontSize: 17, fontWeight: 700 }}>License</h2>
      <div className="nx-panel" style={{ padding: "var(--space-lg)" }}>
        <div style={{ fontSize: 13, display: "flex", flexDirection: "column", gap: "var(--space-sm)" }}>
          <div style={{ display: "flex", justifyContent: "space-between" }}>
            <span style={{ color: "var(--color-text-secondary)" }}>Edition</span>
            <strong>Full — founding researcher</strong>
          </div>
          <div style={{ display: "flex", justifyContent: "space-between" }}>
            <span style={{ color: "var(--color-text-secondary)" }}>Status</span>
            <NxPill tone="success">Active (offline capable)</NxPill>
          </div>
          <div style={{ display: "flex", justifyContent: "space-between" }}>
            <span style={{ color: "var(--color-text-secondary)" }}>Device</span>
            <span>This machine</span>
          </div>
        </div>
        <p className="nx-help" style={{ marginTop: "var(--space-lg)" }}>
          The first 1,000 verified researchers use Numeris Full free. One active device per license, with legitimate
          device transfer. Core statistical work runs fully offline.
        </p>
        <NxButton style={{ marginTop: "var(--space-md)" }}>Deactivate this device…</NxButton>
      </div>
    </div>
  );
}

/* ---------------- About ---------------- */
export function AboutScreen() {
  return (
    <div style={{ padding: "var(--space-3xl)", maxWidth: 520 }}>
      <div className="nx-logo" style={{ fontSize: 20, marginBottom: "var(--space-sm)" }}>
        <span className="nx-logo-mark" style={{ width: 26, height: 26, fontSize: 15 }}>N</span>
        Numeris
      </div>
      <p style={{ fontSize: 13, color: "var(--color-text-secondary)" }}>
        Professional statistical software. Built locally. Computed deterministically. Reproducible by design.
      </p>
      <table className="nx-table" style={{ marginTop: "var(--space-lg)" }}>
        <tbody>
          <tr><td>Version</td><td>1.0.0-dev</td></tr>
          <tr><td>Engine</td><td>numeris-rust-engine</td></tr>
          <tr><td>Statistical validation</td><td>NIST StRD + golden datasets</td></tr>
          <tr><td>Acknowledgement</td><td>NIST StRD reference data (public domain)</td></tr>
          <tr><td>License</td><td>Numeris Software License (source available)</td></tr>
          <tr><td>Author</td><td>hello-aditya-dev</td></tr>
        </tbody>
      </table>
      <p style={{ fontSize: 11, color: "var(--color-text-tertiary)", marginTop: "var(--space-lg)" }}>
        No AI. No subscription. No cloud requirement. No usage meter.
      </p>
    </div>
  );
}
