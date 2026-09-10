// Data Workbench: grid + variable inspector (Data Quality Lab summary).
import { useEffect, useState } from "react";
import { datasetPreview, runCommand, variableProfile } from "../bridge";
import type { ColumnData, SummaryStats, FrequencyRow } from "../bridge";
import { NxDataGrid } from "../components/DataGrid";
import { NxButton, NxEmpty, formatInt, formatNumber } from "../components/Nx";
import { useApp } from "../state/store";

export function DataWorkbenchScreen() {
  const dataLoaded = useApp((s) => s.dataLoaded);
  const selectedVariable = useApp((s) => s.selectedVariable);
  const selectVariable = useApp((s) => s.selectVariable);
  const nRows = useApp((s) => s.nRows);
  const variables = useApp((s) => s.variables);
  const pushConsole = useApp((s) => s.pushConsole);
  const handleOutput = useApp((s) => s.handleOutput);
  const [columns, setColumns] = useState<ColumnData[]>([]);
  const [profile, setProfile] = useState<{ stats: SummaryStats; frequencies: FrequencyRow[] } | null>(null);

  useEffect(() => {
    if (!dataLoaded) return;
    let alive = true;
    datasetPreview(0, 2000).then((p) => {
      if (!alive) return;
      setColumns(p.rows[0] ?? []);
    });
    return () => {
      alive = false;
    };
  }, [dataLoaded, variables]);

  useEffect(() => {
    if (!selectedVariable) {
      setProfile(null);
      return;
    }
    let alive = true;
    variableProfile(selectedVariable).then((p) => {
      if (alive) setProfile(p);
    });
    return () => {
      alive = false;
    };
  }, [selectedVariable, columns]);

  if (!dataLoaded) {
    return (
      <div style={{ height: "100%", display: "grid", placeItems: "center" }}>
        <NxEmpty
          title="No datasets yet"
          hint="Import a CSV, JSON or Parquet dataset to begin. CSV is fully supported in this version."
          action={
            <NxButton
              variant="primary"
              onClick={async () => {
                // The desktop file dialog is opened by the engine layer;
                // the command path keeps a single execution entry point.
                const path = await import("@tauri-apps/plugin-dialog").then((m) => m.open({ filters: [{ name: "CSV", extensions: ["csv"] }] }));
                if (typeof path === "string") {
                  const cmd = `use "${path}"`;
                  pushConsole({ kind: "command", text: cmd });
                  const out = await runCommand(cmd);
                  pushConsole({ kind: "output", text: "DataLoaded" in out ? out.DataLoaded.message : "Imported." });
                  handleOutput(out);
                }
              }}
            >
              Import Data
            </NxButton>
          }
        />
      </div>
    );
  }

  return (
    <div style={{ display: "grid", gridTemplateColumns: "1fr 264px", height: "100%", overflow: "hidden" }}>
      <div style={{ overflow: "hidden", display: "flex", flexDirection: "column", borderRight: "var(--border-subtle)" }}>
        <div
          style={{
            display: "flex",
            alignItems: "center",
            gap: "var(--space-lg)",
            padding: "6px var(--space-md)",
            borderBottom: "var(--border-subtle)",
            background: "var(--color-surface-primary)",
            fontSize: 12,
            color: "var(--color-text-secondary)",
          }}
        >
          <strong style={{ fontSize: 12 }}>{formatInt(nRows)}</strong> observations
          <span style={{ color: "var(--color-border-strong)" }}>|</span>
          <strong style={{ fontSize: 12 }}>{variables.length}</strong> variables
          <span style={{ marginLeft: "auto", display: "flex", gap: "var(--space-sm)" }}>
            <NxButton size="sm" onClick={() => runCommand("describe").then(handleOutput)}>
              Describe
            </NxButton>
            <NxButton size="sm" onClick={() => runCommand("summarize").then(handleOutput)}>
              Summarize
            </NxButton>
          </span>
        </div>
        <div style={{ flex: 1, overflow: "hidden", background: "var(--color-surface-primary)" }}>
          <NxDataGrid
            variables={variables}
            columns={columns}
            nRows={nRows}
            selectedVariable={selectedVariable}
            onSelectVariable={selectVariable}
          />
        </div>
      </div>

      {/* Variable inspector */}
      <aside style={{ overflowY: "auto", background: "var(--color-surface-secondary)", padding: "var(--space-md)" }}>
        {selectedVariable && profile ? (
          <VariableInspector name={selectedVariable} stats={profile.stats} frequencies={profile.frequencies} />
        ) : (
          <div style={{ color: "var(--color-text-tertiary)", fontSize: 12, padding: "var(--space-lg) var(--space-sm)" }}>
            Select a variable (click its column header) to inspect it.
          </div>
        )}
      </aside>
    </div>
  );
}

function VariableInspector({
  name,
  stats,
  frequencies,
}: {
  name: string;
  stats: SummaryStats;
  frequencies: FrequencyRow[];
}) {
  return (
    <div style={{ display: "flex", flexDirection: "column", gap: "var(--space-md)" }}>
      <div>
        <div className="nx-section-label">Variable</div>
        <div style={{ fontFamily: "var(--font-mono)", fontSize: 13, fontWeight: 600 }}>{name}</div>
      </div>
      <div>
        <div className="nx-section-label">Summary</div>
        <table className="nx-table" style={{ fontSize: 12 }}>
          <tbody>
            <tr><td>N</td><td>{stats.n}</td></tr>
            <tr><td>Missing</td><td>{stats.missing} ({stats.n + stats.missing > 0 ? ((stats.missing / (stats.n + stats.missing)) * 100).toFixed(1) : "0"}%)</td></tr>
            <tr><td>Mean</td><td>{formatNumber(stats.mean)}</td></tr>
            <tr><td>Median</td><td>{formatNumber(stats.median)}</td></tr>
            <tr><td>SD</td><td>{formatNumber(stats.sd)}</td></tr>
            <tr><td>Min</td><td>{formatNumber(stats.min)}</td></tr>
            <tr><td>Max</td><td>{formatNumber(stats.max)}</td></tr>
            <tr><td>Q1 / Q3</td><td>{formatNumber(stats.q1)} / {formatNumber(stats.q3)}</td></tr>
            <tr><td>Unique</td><td>{stats.unique}</td></tr>
            {stats.skewness !== null ? <tr><td>Skewness</td><td>{formatNumber(stats.skewness)}</td></tr> : null}
            {stats.kurtosis !== null ? <tr><td>Kurtosis</td><td>{formatNumber(stats.kurtosis)}</td></tr> : null}
          </tbody>
        </table>
      </div>
      {frequencies.length > 0 ? (
        <div>
          <div className="nx-section-label">Frequencies</div>
          <table className="nx-table" style={{ fontSize: 12 }}>
            <tbody>
              {frequencies.slice(0, 12).map((f) => (
                <tr key={f.value}>
                  <td>{f.value}</td>
                  <td>{f.count}</td>
                  <td>{f.percent.toFixed(1)}%</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      ) : null}
    </div>
  );
}
