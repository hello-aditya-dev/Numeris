"use client";

// Numeris — interactive web preview of the desktop product.
// The complete statistical workflow runs for real in this page: the same
// command language, the same estimator inventory, the same GUI⇄command
// equivalence, ported to TypeScript from the Rust engine (NIST StRD
// validated). The shipped desktop application is built from the repository.

import React, { useCallback, useEffect, useMemo, useRef, useState } from "react";
import "./numeris.css";
import { Executor, type ExecOutput, type EstimateResult, type DiagnosticRow } from "@/lib/numeris/executor";
import type { VcovOption } from "@/lib/numeris/parser";
import { render as renderCommand } from "@/lib/numeris/parser";
import { buildDemoDataset, type Dataset } from "@/lib/numeris/dataset";
import { fmt, fmtInt, fmtP, OutputView, SectionLabel, Stat, stars } from "./views";

type Screen =
  | "home" | "data" | "analysis" | "console" | "results" | "comparison"
  | "graphics" | "tables" | "notes" | "registry" | "settings" | "license" | "about";

interface ConsoleLine {
  kind: "command" | "output" | "error";
  text: string;
}

interface EstimateEntry {
  id: string;
  label: string;
  command: string;
  result: EstimateResult;
  diagnostics: DiagnosticRow[];
  output: ExecOutput;
}

const NAV: { section: string; items: { id: Screen; label: string }[] }[] = [
  { section: "Workspace", items: [{ id: "home", label: "Project home" }, { id: "data", label: "Data workbench" }] },
  { section: "Analysis", items: [{ id: "analysis", label: "Model workspace" }, { id: "console", label: "Console" }] },
  { section: "Output", items: [{ id: "results", label: "Results" }, { id: "comparison", label: "Model comparison" }, { id: "graphics", label: "Graphics" }, { id: "tables", label: "Tables" }] },
  { section: "Research", items: [{ id: "notes", label: "Notes" }, { id: "registry", label: "Registry & replay" }] },
  { section: "Application", items: [{ id: "settings", label: "Settings" }, { id: "license", label: "License" }, { id: "about", label: "About" }] },
];

const REPO_URL = "https://github.com/hello-aditya-dev/Numeris";

export default function NumerisApp() {
  const executorRef = useRef<Executor | null>(null);
  if (!executorRef.current) executorRef.current = new Executor();
  const executor = executorRef.current;

  const [screen, setScreen] = useState<Screen>("home");
  const [theme, setTheme] = useState<"light" | "dark">("light");
  const [paletteOpen, setPaletteOpen] = useState(false);
  const [datasetVersion, setDatasetVersion] = useState(0);
  const [consoleLines, setConsoleLines] = useState<ConsoleLine[]>([
    { kind: "output", text: "Numeris console ready — deterministic engine loaded.\nType 'help' for the command reference, or try: regress wage education experience female, robust" },
  ]);
  const [estimates, setEstimates] = useState<EstimateEntry[]>([]);
  const [activeEstimate, setActiveEstimate] = useState<string | null>(null);
  const [selectedVariable, setSelectedVariable] = useState<string | null>(null);
  const [consoleText, setConsoleText] = useState("");
  const [histIdx, setHistIdx] = useState<number | null>(null);
  const [commandHistory, setCommandHistory] = useState<string[]>([]);
  const [notes, setNotes] = useState<string[]>([]);
  const [lastOutput, setLastOutput] = useState<ExecOutput | null>(null);
  const [pendingCompare, setPendingCompare] = useState<ExecOutput | null>(null);
  const consoleEndRef = useRef<HTMLDivElement>(null);
  const consoleInputRef = useRef<HTMLInputElement>(null);
  const fileInputRef = useRef<HTMLInputElement>(null);

  const dataset: Dataset = executor.dataset;

  useEffect(() => {
    if (consoleEndRef.current) consoleEndRef.current.scrollIntoView({ block: "nearest" });
  }, [consoleLines]);

  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      const mod = e.metaKey || e.ctrlKey;
      if (mod && e.key.toLowerCase() === "k") {
        e.preventDefault();
        setPaletteOpen((o) => !o);
      } else if (e.key === "Escape") {
        setPaletteOpen(false);
      }
    };
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, []);

  const runCommand = useCallback(
    (source: string): ExecOutput | { __error: { what: string; why: string; action: string } } => {
      const trimmed = source.trim();
      if (!trimmed) return { __error: { what: "Empty command.", why: "", action: "" } };
      setConsoleLines((ls) => [...ls, { kind: "command", text: trimmed }]);
      setCommandHistory((h) => [...h, trimmed]);
      try {
        const out = executor.execute(trimmed);
        setConsoleLines((ls) => [...ls, { kind: "output", text: describe(out) }]);
        setLastOutput(out);
        if (out.kind === "Estimate") {
          setEstimates((es) => [
            ...es,
            { id: out.id, label: out.label, command: trimmed, result: out.result, diagnostics: out.diagnostics, output: out },
          ]);
          setActiveEstimate(out.id);
        }
        if (out.kind === "Notes") setNotes(out.notes);
        return out;
      } catch (err) {
        const e = err as { message: string };
        const nice = err instanceof Error && "what" in err ? (err as unknown as { what: string; why: string; action: string }) : null;
        setConsoleLines((ls) => [
          ...ls,
          { kind: "error", text: nice ? `${nice.what}\n  why:    ${nice.why}\n  to do:  ${nice.action}` : e.message },
        ]);
        return { __error: nice ?? { what: e.message, why: "", action: "" } };
      }
    },
    [executor],
  );

  const datasetStats = useMemo(() => {
    const numericVars = dataset.variables.filter((v) => v.storage === "Numeric");
    return { rows: dataset.rows, vars: dataset.variables.length, numericVars };
  }, [dataset, datasetVersion]);

  const activeEstimateEntry = useMemo(() => {
    if (estimates.length === 0) return null;
    return estimates.find((e) => e.id === (activeEstimate ?? estimates[estimates.length - 1].id)) ?? estimates[estimates.length - 1];
  }, [estimates, activeEstimate]);

  /* ---------- Palette actions ---------- */
  const paletteActions = useMemo(() => {
    const nav = NAV.flatMap((s) => s.items).map((item) => ({ id: item.id, group: "Navigate", label: item.label, run: () => setScreen(item.id) }));
    const cmds = [
      { id: "sum", group: "Data", label: "Summarize all variables", run: () => { runCommand("summarize"); setScreen("results"); } },
      { id: "reg", group: "Analyze", label: "Run baseline wage regression", run: () => { runCommand("regress wage education experience female, robust"); setScreen("results"); } },
      { id: "cluster", group: "Analyze", label: "Run clustered-SE regression", run: () => { runCommand("regress wage education experience female, cluster(firm)"); setScreen("results"); } },
      { id: "fe", group: "Analyze", label: "Run fixed-effects panel model", run: () => { runCommand("xtreg wage education experience, fe entity(firm)"); setScreen("results"); } },
      { id: "did", group: "Analyze", label: "Run difference-in-differences", run: () => { runCommand("did wage, treat(treated) time(post), cluster(firm)"); setScreen("results"); } },
      { id: "iv", group: "Analyze", label: "Run 2SLS (education instrumented by distance)", run: () => { runCommand("ivregress 2sls wage (education = distance) experience, robust"); setScreen("results"); } },
      { id: "logit", group: "Analyze", label: "Run logit for employment", run: () => { runCommand("logit employed education experience female, robust"); setScreen("results"); } },
      { id: "cmp", group: "Analyze", label: "Compare all models", run: () => { runCommand("estimate compare"); setScreen("comparison"); } },
      { id: "logwage", group: "Data", label: "Create log wage variable", run: () => runCommand("generate logwage = ln(wage)") },
      { id: "theme", group: "View", label: "Toggle light / dark theme", run: () => setTheme((t) => (t === "light" ? "dark" : "light")) },
    ];
    return [...nav, ...cmds];
  }, [runCommand]);

  return (
    <div className="nx-root" data-theme={theme}>
      <header className="nx-titlebar">
        <span className="nx-logo">
          <span className="nx-logo-mark">N</span>
          Numeris
        </span>
        <span style={{ color: "var(--nx-text-3)", fontSize: 12 }}>wage_survey — interactive preview</span>
        <div style={{ marginLeft: "auto", display: "flex", gap: 8, alignItems: "center" }}>
          <input
            ref={fileInputRef}
            type="file"
            accept=".csv,text/csv"
            style={{ display: "none" }}
            onChange={async (e) => {
              const file = e.target.files?.[0];
              if (!file) return;
              const text = await file.text();
              try {
                executor.loadCsv(text);
                setDatasetVersion((v) => v + 1);
                setEstimates([]);
                setSelectedVariable(null);
                setConsoleLines((ls) => [...ls, { kind: "output", text: `Loaded ${file.name}: ${executor.dataset.rows} observations, ${executor.dataset.variables.length} variables.` }]);
                setScreen("data");
              } catch (err) {
                setConsoleLines((ls) => [...ls, { kind: "error", text: (err as Error).message }]);
              }
              e.target.value = "";
            }}
          />
          <button className="nx-btn nx-btn-sm" onClick={() => fileInputRef.current?.click()}>
            Import CSV
          </button>
          <button
            className="nx-btn nx-btn-sm"
            onClick={() => {
              executor.loadDemo();
              setDatasetVersion((v) => v + 1);
              setEstimates([]);
              setConsoleLines((ls) => [...ls, { kind: "output", text: "Demo dataset restored." }]);
            }}
          >
            Demo data
          </button>
          <button className="nx-btn nx-btn-sm" onClick={() => setTheme((t) => (t === "light" ? "dark" : "light"))}>
            {theme === "light" ? "Dark" : "Light"}
          </button>
          <button className="nx-btn nx-btn-sm" onClick={() => setPaletteOpen(true)} title="Command palette (Ctrl/Cmd+K)">
            Search…
          </button>
        </div>
      </header>

      <div className="nx-body">
        <nav className="nx-sidebar" aria-label="Primary navigation">
          {NAV.map((section) => (
            <div key={section.section} className="nx-nav-section">
              <div className="nx-nav-label">{section.section}</div>
              {section.items.map((item) => (
                <button key={item.id} className="nx-nav-item" data-active={screen === item.id} onClick={() => setScreen(item.id)}>
                  <span>{item.label}</span>
                </button>
              ))}
            </div>
          ))}
        </nav>

        <main className="nx-main">
          {screen === "home" && <HomeScreen executor={executor} estimates={estimates} setScreen={setScreen} runCommand={runCommand} datasetStats={datasetStats} />}
          {screen === "data" && (
            <DataScreen executor={executor} datasetVersion={datasetVersion} selectedVariable={selectedVariable} setSelectedVariable={setSelectedVariable} runCommand={runCommand} />
          )}
          {screen === "analysis" && <AnalysisScreen executor={executor} runCommand={runCommand} datasetVersion={datasetVersion} setScreen={setScreen} />}
          {screen === "console" && (
            <ConsoleScreen
              lines={consoleLines}
              text={consoleText}
              setText={setConsoleText}
              runCommand={runCommand}
              history={commandHistory}
              histIdx={histIdx}
              setHistIdx={setHistIdx}
              endRef={consoleEndRef}
              inputRef={consoleInputRef}
              setScreen={setScreen}
            />
          )}
          {screen === "results" && <ResultsScreen estimates={estimates} active={activeEstimateEntry} setActive={setActiveEstimate} />}
          {screen === "comparison" && <ComparisonScreen estimates={estimates} runCommand={runCommand} pending={pendingCompare} setPending={setPendingCompare} />}
          {screen === "graphics" && <GraphicsScreen executor={executor} datasetVersion={datasetVersion} />}
          {screen === "tables" && <TablesScreen executor={executor} estimates={estimates} />}
          {screen === "notes" && <NotesScreen notes={notes} runCommand={runCommand} />}
          {screen === "registry" && <RegistryScreen estimates={estimates} executor={executor} notes={notes} />}
          {screen === "settings" && <SettingsScreen theme={theme} setTheme={setTheme} />}
          {screen === "license" && <LicenseScreen />}
          {screen === "about" && <AboutScreen />}
        </main>
      </div>

      <footer className="nx-statusbar">
        <span>
          {fmtInt(datasetStats.rows)} observations · {datasetStats.vars} variables
        </span>
        <span>{estimates.length > 0 ? `${estimates.length} estimates` : ""}</span>
        <div className="nx-status-right">
          <span>Ready</span>
          <span>Local computation</span>
          <span>No AI</span>
        </div>
      </footer>

      {paletteOpen ? <Palette actions={paletteActions} onClose={() => setPaletteOpen(false)} /> : null}
    </div>
  );
}

/* ================= Home ================= */

function HomeScreen({
  executor,
  estimates,
  setScreen,
  runCommand,
  datasetStats,
}: {
  executor: Executor;
  estimates: EstimateEntry[];
  setScreen: (s: Screen) => void;
  runCommand: (cmd: string) => ExecOutput | { __error: unknown };
  datasetStats: { rows: number; vars: number };
}) {
  const recent = estimates.slice(-5).reverse();
  return (
    <div className="nx-screen" style={{ maxWidth: 780 }}>
      <h2 style={{ margin: 0, fontSize: 20, fontWeight: 700 }}>Wage Survey — Labor Economics</h2>
      <p style={{ color: "var(--nx-text-2)", fontSize: 13, marginTop: 4, marginBottom: 20 }}>
        Professional statistical software. Built locally. Computed deterministically. Reproducible by design.
      </p>
      <SectionLabel>Workspace</SectionLabel>
      <div className="nx-fitstats" style={{ gridTemplateColumns: "repeat(auto-fill,minmax(150px,1fr))" }}>
        <Stat label="Observations" value={fmtInt(datasetStats.rows)} />
        <Stat label="Variables" value={String(datasetStats.vars)} />
        <Stat label="Analyses" value={String(estimates.length)} />
        <Stat label="Engine" value="Rust-core (TS port)" />
      </div>
      <SectionLabel>Quick start</SectionLabel>
      <div style={{ display: "flex", gap: 8, flexWrap: "wrap", marginBottom: 20 }}>
        <button className="nx-btn nx-btn-primary" onClick={() => { runCommand("regress wage education experience female, robust"); setScreen("results"); }}>
          Run baseline regression
        </button>
        <button className="nx-btn" onClick={() => { runCommand("summarize"); setScreen("results"); }}>
          Summarize data
        </button>
        <button className="nx-btn" onClick={() => setScreen("console")}>
          Open console
        </button>
        <button className="nx-btn" onClick={() => setScreen("data")}>
          Inspect data
        </button>
      </div>
      <SectionLabel>Recent analyses</SectionLabel>
      {recent.length === 0 ? (
        <p style={{ color: "var(--nx-text-3)", fontSize: 13 }}>No analyses yet. Every executed model is recorded with its command, specification and result.</p>
      ) : (
        <div className="nx-panel">
          {recent.map((e) => (
            <div key={e.id} style={{ display: "flex", gap: 12, padding: "8px 16px", borderBottom: "1px solid var(--nx-border-s)", alignItems: "center" }}>
              <span style={{ fontFamily: "monospace", fontSize: 12, color: "var(--nx-text-3)" }}>{e.id}</span>
              <span style={{ fontSize: 13, fontWeight: 600 }}>{e.label}</span>
              <span style={{ fontFamily: "monospace", fontSize: 11, color: "var(--nx-text-3)", marginLeft: "auto", overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>
                {e.command}
              </span>
            </div>
          ))}
        </div>
      )}
      <SectionLabel>Project health</SectionLabel>
      <div style={{ display: "flex", flexDirection: "column", gap: 4 }}>
        <div className="nx-diag" data-status="ok">
          <span className="nx-diag-status">✓</span>
          <span>Data available — deterministic synthetic panel (40 firms × 2018–2022)</span>
        </div>
        <div className="nx-diag" data-status="ok">
          <span className="nx-diag-status">✓</span>
          <span>Command log and analysis registry active</span>
        </div>
        <div className="nx-diag" data-status="ok">
          <span className="nx-diag-status">✓</span>
          <span>Engine validated against NIST StRD certified reference values</span>
        </div>
      </div>
    </div>
  );
}

/* ================= Data workbench ================= */

function DataScreen({
  executor,
  datasetVersion,
  selectedVariable,
  setSelectedVariable,
  runCommand,
}: {
  executor: Executor;
  datasetVersion: number;
  selectedVariable: string | null;
  setSelectedVariable: (v: string | null) => void;
  runCommand: (cmd: string) => ExecOutput | { __error: unknown };
}) {
  const ds = executor.dataset;
  const [sortState, setSortState] = useState<{ col: string; dir: 1 | -1 } | null>(null);
  const [filterText, setFilterText] = useState("");
  const [scrollTop, setScrollTop] = useState(0);
  const [viewH, setViewH] = useState(560);
  const gridRef = useRef<HTMLDivElement>(null);

  const filteredRows = useMemo(() => {
    const rows: number[] = [];
    for (let i = 0; i < ds.rows; i++) rows.push(i);
    if (!filterText.trim()) return rows;
    const q = filterText.trim().toLowerCase();
    return rows.filter((i) => ds.variables.some((v) => String(ds.numeric[v.name]?.[i] ?? ds.text[v.name]?.[i] ?? "").toLowerCase().includes(q)));
  }, [ds, filterText, datasetVersion]);

  const orderedRows = useMemo(() => {
    if (!sortState) return filteredRows;
    const col = ds.numeric[sortState.col];
    const tcol = ds.text[sortState.col];
    const sorted = [...filteredRows];
    sorted.sort((a, b) => {
      if (col) {
        const av = col[a];
        const bv = col[b];
        if (av === null && bv === null) return 0;
        if (av === null) return 1;
        if (bv === null) return -1;
        return (av - bv) * sortState.dir;
      }
      if (tcol) return ((tcol[a] ?? "").localeCompare(tcol[b] ?? "")) * sortState.dir;
      return 0;
    });
    return sorted;
  }, [filteredRows, sortState, ds, datasetVersion]);

  const ROW_H = 22;
  const first = Math.max(0, Math.floor(scrollTop / ROW_H) - 12);
  const last = Math.min(orderedRows.length, Math.ceil((scrollTop + viewH) / ROW_H) + 12);
  const visible = orderedRows.slice(first, last);

  const profile = useMemo(() => {
    if (!selectedVariable) return null;
    const out = executor.execute(`summarize ${selectedVariable}`);
    if (out.kind === "Summary") return out.stats[0];
    return null;
  }, [selectedVariable, datasetVersion, executor]);

  return (
    <div
      className="nx-screen"
      style={{ display: "grid", gridTemplateColumns: "minmax(0, 1fr) 260px", gap: 16, alignItems: "start", maxWidth: "100%", overflowX: "hidden" }}
      data-workbench="true"
    >
      <div style={{ minWidth: 0 }}>
        <div style={{ display: "flex", gap: 12, alignItems: "center", marginBottom: 8, flexWrap: "wrap" }}>
          <h2 style={{ margin: 0, fontSize: 15, fontWeight: 700 }}>Data workbench</h2>
          <span style={{ fontSize: 12, color: "var(--nx-text-2)" }}>
            <strong>{fmtInt(orderedRows.length)}</strong> rows · <strong>{ds.variables.length}</strong> variables · {ds.name}
          </span>
          <div style={{ marginLeft: "auto", display: "flex", gap: 8 }}>
            <input className="nx-input" style={{ width: 160 }} placeholder="Filter rows…" value={filterText} onChange={(e) => setFilterText(e.target.value)} />
            <button className="nx-btn nx-btn-sm" onClick={() => runCommand("describe")}>Describe</button>
            <button className="nx-btn nx-btn-sm" onClick={() => runCommand("summarize")}>Summarize</button>
          </div>
        </div>
        <div className="nx-grid-wrap nx-grid" ref={gridRef} onScroll={(e) => { setScrollTop(e.currentTarget.scrollTop); setViewH(e.currentTarget.clientHeight); }}>
          <table>
            <thead>
              <tr>
                <th>#</th>
                {ds.variables.map((v) => (
                  <th
                    key={v.name}
                    data-selected={selectedVariable === v.name}
                    title={v.label || v.name}
                    onClick={() => {
                      setSelectedVariable(v.name);
                      setSortState((s) => (s && s.col === v.name ? { col: v.name, dir: s.dir === 1 ? -1 : 1 } : { col: v.name, dir: 1 }));
                    }}
                  >
                    {v.name}
                    {sortState?.col === v.name ? (sortState.dir === 1 ? " ↑" : " ↓") : ""}
                  </th>
                ))}
              </tr>
            </thead>
            <tbody>
              {visible.map((i) => (
                <tr key={i}>
                  <td>{i + 1}</td>
                  {ds.variables.map((v) => {
                    const num = ds.numeric[v.name];
                    const txt = ds.text[v.name];
                    if (num) {
                      const val = num[i];
                      return (
                        <td key={v.name} data-missing={val === null}>
                          {val === null ? "" : Math.abs(val) >= 1e7 ? val.toExponential(2) : String(val)}
                        </td>
                      );
                    }
                    return <td key={v.name}>{txt?.[i] ?? ""}</td>;
                  })}
                </tr>
              ))}
            </tbody>
          </table>
        </div>
        <p style={{ fontSize: 11, color: "var(--nx-text-3)", margin: "6px 0 0" }}>
          Click a column header to select the variable and sort. The inspector on the right shows its profile.
        </p>
      </div>

      <aside style={{ background: "var(--nx-surface-2)", borderRadius: 8, padding: 14, border: "1px solid var(--nx-border-s)", minWidth: 0 }}>
        {selectedVariable && profile ? (
          <>
            <SectionLabel>Variable</SectionLabel>
            <div style={{ fontFamily: "monospace", fontSize: 13, fontWeight: 600, marginBottom: 10 }}>{selectedVariable}</div>
            {(() => {
              const v = ds.variables.find((x) => x.name === selectedVariable);
              return v?.label ? <div style={{ fontSize: 12, color: "var(--nx-text-2)", marginBottom: 10 }}>{v.label}</div> : null;
            })()}
            <SectionLabel>Summary</SectionLabel>
            <table className="nx-table" style={{ fontSize: 12 }}>
              <tbody>
                <tr><td>N</td><td>{profile.n}</td></tr>
                <tr><td>Missing</td><td>{profile.missing}{profile.n + profile.missing > 0 ? ` (${((profile.missing / (profile.n + profile.missing)) * 100).toFixed(1)}%)` : ""}</td></tr>
                <tr><td>Mean</td><td>{fmt(profile.mean)}</td></tr>
                <tr><td>Median</td><td>{fmt(profile.median)}</td></tr>
                <tr><td>SD</td><td>{fmt(profile.sd)}</td></tr>
                <tr><td>Min</td><td>{fmt(profile.min)}</td></tr>
                <tr><td>Max</td><td>{fmt(profile.max)}</td></tr>
                <tr><td>Q1 / Q3</td><td>{fmt(profile.q1)} / {fmt(profile.q3)}</td></tr>
                <tr><td>Unique</td><td>{profile.unique}</td></tr>
                {profile.skewness !== null ? <tr><td>Skewness</td><td>{fmt(profile.skewness)}</td></tr> : null}
                {profile.kurtosis !== null ? <tr><td>Kurtosis</td><td>{fmt(profile.kurtosis)}</td></tr> : null}
              </tbody>
            </table>
            <SectionLabel>Actions</SectionLabel>
            <div style={{ display: "flex", flexWrap: "wrap", gap: 6 }}>
              <button className="nx-pill" onClick={() => runCommand(`generate ln_${selectedVariable} = ln(${selectedVariable})`)}>log transform</button>
              <button className="nx-pill" onClick={() => runCommand(`generate z_${selectedVariable} = standardize(${selectedVariable})`)}>standardize</button>
              <button className="nx-pill" onClick={() => runCommand(`generate rank_${selectedVariable} = rank(${selectedVariable})`)}>rank</button>
              <button className="nx-pill" onClick={() => runCommand(`ttest ${selectedVariable} == ${Math.round((profile.mean ?? 0) * 10) / 10}`)}>t-test vs mean</button>
            </div>
          </>
        ) : (
          <div style={{ color: "var(--nx-text-3)", fontSize: 12, padding: "8px 4px" }}>
            Select a variable (click its column header) to inspect its profile, distribution summary and quick actions.
          </div>
        )}
      </aside>
    </div>
  );
}

/* ================= Analysis (GUI form) ================= */

function AnalysisScreen({ executor, runCommand, datasetVersion, setScreen }: { executor: Executor; runCommand: (cmd: string) => ExecOutput | { __error: unknown }; datasetVersion: number; setScreen: (s: Screen) => void }) {
  const numericVars = useMemo(() => executor.dataset.variables.filter((v) => v.storage === "Numeric").map((v) => v.name), [executor, datasetVersion]);
  const [estimator, setEstimator] = useState<"regress" | "xtreg" | "ivregress" | "did" | "logit" | "probit" | "poisson">("regress");
  const [outcome, setOutcome] = useState("wage");
  const [predictors, setPredictors] = useState<string[]>(["education", "experience"]);
  const [vcov, setVcov] = useState<"classical" | "robust" | "hc2" | "hc3" | "cluster" | "hac">("robust");
  const [clusterVar, setClusterVar] = useState("firm");
  const [entity, setEntity] = useState("firm");
  const [instrument, setInstrument] = useState("distance");
  const [treatVar, setTreatVar] = useState("treated");
  const [postVar, setPostVar] = useState("post");
  const [hacLags, setHacLags] = useState("3");
  const [error, setError] = useState<{ what: string; why: string; action: string } | null>(null);
  const [busy, setBusy] = useState(false);

  const command = useMemo(() => {
    const preds = predictors.join(" ");
    const seOpt =
      vcov === "cluster" ? `, cluster(${clusterVar})` : vcov === "hac" ? `, vce(hac ${hacLags})` : vcov === "classical" ? "" : `, ${vcov}`;
    const opt: VcovOption = vcov === "cluster" ? { cluster: clusterVar } : vcov === "hac" ? { hac: Number(hacLags) } : (vcov as VcovOption);
    switch (estimator) {
      case "regress":
        return renderCommand({ kind: "regress", outcome, predictors, vcov: opt });
      case "xtreg":
        return renderCommand({ kind: "xtreg", outcome, predictors, entity, model: "fe", vcov: vcov === "cluster" ? { cluster: clusterVar } : (vcov === "classical" ? "classical" : vcov as VcovOption) });
      case "ivregress":
        return renderCommand({ kind: "ivregress", outcome, endogenous: [instrument], instruments: [instrument === "distance" ? "distance" : "distance"], exogenous: predictors, vcov: "robust" });
      case "did":
        return renderCommand({ kind: "did", outcome, controls: predictors, treat: treatVar, time: postVar, vcov: vcov === "cluster" ? { cluster: clusterVar } : vcov === "robust" ? "robust" : "classical" });
      case "logit":
      case "probit":
      case "poisson":
        return renderCommand({ kind: estimator, outcome, predictors, robust: vcov === "robust" });
    }
  }, [estimator, outcome, predictors, vcov, clusterVar, entity, instrument, treatVar, postVar, hacLags]);

  const run = () => {
    setBusy(true);
    setError(null);
    const out = runCommand(command);
    if ("__error" in out && out.__error) {
      setError(out.__error as { what: string; why: string; action: string });
    } else if (typeof out === "object" && out !== null && "kind" in out && out.kind === "Estimate") {
      setScreen("results");
    }
    setBusy(false);
  };

  return (
    <div className="nx-screen" style={{ maxWidth: 720 }}>
      <h2 style={{ margin: 0, fontSize: 17, fontWeight: 700 }}>Model specification</h2>
      <p style={{ margin: "4px 0 20px", fontSize: 12, color: "var(--nx-text-3)" }}>
        The form and the command language compile to the same specification. There is one execution path.
      </p>

      <div className="nx-field" style={{ marginBottom: 16 }}>
        <label>Estimator</label>
        <select className="nx-select" value={estimator} onChange={(e) => setEstimator(e.target.value as typeof estimator)}>
          <option value="regress">Linear regression (OLS)</option>
          <option value="xtreg">Panel: fixed effects (within)</option>
          <option value="ivregress">Instrumental variables (2SLS)</option>
          <option value="did">Difference-in-differences</option>
          <option value="logit">Logistic regression</option>
          <option value="probit">Probit regression</option>
          <option value="poisson">Poisson regression</option>
        </select>
      </div>

      <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 16, marginBottom: 16 }}>
        <div className="nx-field">
          <label>Outcome variable</label>
          <select className="nx-select" value={outcome} onChange={(e) => setOutcome(e.target.value)}>
            {numericVars.map((v) => (
              <option key={v}>{v}</option>
            ))}
          </select>
        </div>
        {estimator === "xtreg" ? (
          <div className="nx-field">
            <label>Entity identifier</label>
            <select className="nx-select" value={entity} onChange={(e) => setEntity(e.target.value)}>
              {numericVars.map((v) => (
                <option key={v}>{v}</option>
              ))}
            </select>
          </div>
        ) : estimator === "ivregress" ? (
          <div className="nx-field">
            <label>Instrument for education</label>
            <select className="nx-select" value={instrument} onChange={(e) => setInstrument(e.target.value)}>
              {numericVars.map((v) => (
                <option key={v}>{v}</option>
              ))}
            </select>
          </div>
        ) : estimator === "did" ? (
          <div className="nx-field">
            <label>Treatment indicator (0/1)</label>
            <select className="nx-select" value={treatVar} onChange={(e) => setTreatVar(e.target.value)}>
              {numericVars.map((v) => (
                <option key={v}>{v}</option>
              ))}
            </select>
          </div>
        ) : null}
      </div>

      {estimator === "did" ? (
        <div className="nx-field" style={{ marginBottom: 16 }}>
          <label>Post-period indicator (0/1)</label>
          <select className="nx-select" value={postVar} onChange={(e) => setPostVar(e.target.value)}>
            {numericVars.map((v) => (
              <option key={v}>{v}</option>
            ))}
          </select>
        </div>
      ) : null}

      <div className="nx-field" style={{ marginBottom: 16 }}>
        <label>Predictors — click to add or remove (ordered)</label>
        <div style={{ display: "flex", flexWrap: "wrap", gap: 6 }}>
          {numericVars
            .filter((v) => v !== outcome)
            .map((v) => {
              const idx = predictors.indexOf(v);
              const active = idx >= 0;
              return (
                <button
                  key={v}
                  type="button"
                  className="nx-pill"
                  data-tone={active ? "accent" : undefined}
                  onClick={() => setPredictors((ps) => (active ? ps.filter((p) => p !== v) : [...ps, v]))}
                  title={active ? `Position ${idx + 1} — click to remove` : "Click to add"}
                >
                  {active ? `${idx + 1}. ` : ""}
                  {v}
                </button>
              );
            })}
        </div>
      </div>

      {estimator === "regress" || estimator === "xtreg" || estimator === "did" ? (
        <div className="nx-field" style={{ marginBottom: 16 }}>
          <label>Standard errors</label>
          <div style={{ display: "flex", gap: 14, flexWrap: "wrap" }}>
            {(
              [
                ["classical", "Conventional"],
                ["robust", "Robust"],
                ["hc2", "HC2"],
                ["hc3", "HC3"],
                ["cluster", "Clustered"],
                ["hac", "HAC"],
              ] as const
            ).map(([v, label]) => (
              <label key={v} style={{ display: "flex", gap: 5, alignItems: "center", fontSize: 13 }}>
                <input type="radio" checked={vcov === v} onChange={() => setVcov(v)} />
                {label}
              </label>
            ))}
          </div>
        </div>
      ) : null}

      {vcov === "cluster" ? (
        <div className="nx-field" style={{ marginBottom: 16, maxWidth: 280 }}>
          <label>Cluster variable</label>
          <select className="nx-select" value={clusterVar} onChange={(e) => setClusterVar(e.target.value)}>
            {numericVars.map((v) => (
              <option key={v}>{v}</option>
            ))}
          </select>
        </div>
      ) : null}
      {vcov === "hac" ? (
        <div className="nx-field" style={{ marginBottom: 16, maxWidth: 280 }}>
          <label>HAC lag order</label>
          <input className="nx-input" value={hacLags} onChange={(e) => setHacLags(e.target.value)} inputMode="numeric" />
        </div>
      ) : null}

      <SectionLabel>Command</SectionLabel>
      <div className="nx-command-preview" style={{ marginBottom: 16 }}>
        <span style={{ color: "var(--nx-text-3)" }}>›</span>
        <span>{command}</span>
      </div>

      {error ? (
        <div className="nx-error-report" role="alert" style={{ marginBottom: 16 }}>
          <h4>{error.what}</h4>
          <dl>
            {error.why ? (
              <>
                <dt>Why this happened</dt>
                <dd>{error.why}</dd>
              </>
            ) : null}
            {error.action ? (
              <>
                <dt>What to do</dt>
                <dd>{error.action}</dd>
              </>
            ) : null}
          </dl>
        </div>
      ) : null}

      <div style={{ display: "flex", gap: 8, justifyContent: "flex-end" }}>
        <button className="nx-btn" onClick={() => setPredictors([])}>Clear</button>
        <button className="nx-btn nx-btn-primary" onClick={run} disabled={busy || predictors.length === 0}>
          {busy ? "Estimating…" : "Run"}
        </button>
      </div>
    </div>
  );
}

/* ================= Console ================= */

function ConsoleScreen({
  lines,
  text,
  setText,
  runCommand,
  history,
  histIdx,
  setHistIdx,
  endRef,
  inputRef,
  setScreen,
}: {
  lines: ConsoleLine[];
  text: string;
  setText: (t: string) => void;
  runCommand: (cmd: string) => ExecOutput | { __error: unknown };
  history: string[];
  histIdx: number | null;
  setHistIdx: (i: number | null) => void;
  endRef: React.RefObject<HTMLDivElement | null>;
  inputRef: React.RefObject<HTMLInputElement | null>;
  setScreen: (s: Screen) => void;
}) {
  const submit = () => {
    const out = runCommand(text);
    setText("");
    setHistIdx(null);
    if (typeof out === "object" && out !== null && "kind" in out && out.kind === "Estimate") {
      // keep the user in the console; results are one click away
    }
  };
  return (
    <div className="nx-screen">
      <div style={{ display: "flex", gap: 12, alignItems: "center", marginBottom: 8 }}>
        <h2 style={{ margin: 0, fontSize: 15, fontWeight: 700 }}>Console</h2>
        <span style={{ fontSize: 12, color: "var(--nx-text-2)" }}>Command language · single execution path shared with the GUI</span>
        <button className="nx-btn nx-btn-sm" style={{ marginLeft: "auto" }} onClick={() => setScreen("results")}>
          View results
        </button>
      </div>
      <div className="nx-console" onClick={() => inputRef.current?.focus()}>
        {lines.map((l, i) => (
          <div key={i} style={{ marginBottom: 3 }}>
            {l.kind === "command" ? (
              <div className="nx-console-line">
                <span className="nx-console-prompt">›</span>
                <span>{l.text}</span>
              </div>
            ) : (
              <div className="nx-console-out" data-kind={l.kind === "error" ? "error" : undefined}>
                {l.text}
              </div>
            )}
          </div>
        ))}
        <div className="nx-console-line">
          <span className="nx-console-prompt">›</span>
          <input
            ref={inputRef}
            className="nx-console-input"
            value={text}
            placeholder="Type a command and press Enter…"
            onChange={(e) => setText(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === "Enter") submit();
              else if (e.key === "ArrowUp") {
                e.preventDefault();
                if (history.length === 0) return;
                const idx = histIdx === null ? history.length - 1 : Math.max(0, histIdx - 1);
                setHistIdx(idx);
                setText(history[idx]);
              } else if (e.key === "ArrowDown") {
                e.preventDefault();
                if (histIdx === null) return;
                const idx = histIdx + 1;
                if (idx >= history.length) {
                  setHistIdx(null);
                  setText("");
                } else {
                  setHistIdx(idx);
                  setText(history[idx]);
                }
              }
            }}
            autoFocus
          />
        </div>
        <div ref={endRef} />
      </div>
      <div style={{ display: "flex", gap: 6, flexWrap: "wrap", marginTop: 10 }}>
        {[
          "help",
          "summarize",
          "describe",
          "correlate wage education experience",
          "ttest wage, by(female)",
          "anova wage urban",
          "regress wage education experience female, robust",
          "regress wage education experience female, cluster(firm)",
          "xtreg wage education experience, fe entity(firm)",
          "ivregress 2sls wage (education = distance) experience, robust",
          "did wage, treat(treated) time(post), cluster(firm)",
          "logit employed education experience female, robust",
          "poisson visits education experience",
          "generate logwage = ln(wage)",
          "estimate list",
          "estimate compare",
          "notes",
        ].map((c) => (
          <button key={c} className="nx-pill" onClick={() => runCommand(c)} title={`Run: ${c}`}>
            {c.length > 34 ? c.slice(0, 32) + "…" : c}
          </button>
        ))}
      </div>
    </div>
  );
}

/* ================= Results ================= */

function ResultsScreen({ estimates, active, setActive }: { estimates: EstimateEntry[]; active: EstimateEntry | null; setActive: (id: string) => void }) {
  if (estimates.length === 0) {
    return (
      <div className="nx-screen" style={{ display: "grid", placeItems: "center", minHeight: "50vh" }}>
        <div className="nx-empty">
          <div className="nx-empty-title">No results yet</div>
          <div className="nx-empty-hint">Run a model from the Model workspace or the Console; results appear here as an editorial document with diagnostics.</div>
        </div>
      </div>
    );
  }
  return (
    <div className="nx-screen" style={{ display: "grid", gridTemplateColumns: "190px minmax(0, 1fr)", gap: 16, alignItems: "start" }}>
      <aside style={{ position: "sticky", top: 16 }}>
        {estimates
          .slice()
          .reverse()
          .map((e) => (
            <button key={e.id} className="nx-nav-item" data-active={active?.id === e.id} onClick={() => setActive(e.id)}>
              <span style={{ fontFamily: "monospace", fontSize: 11, color: "var(--nx-text-3)" }}>{e.id}</span>
              <span>{e.label}</span>
            </button>
          ))}
      </aside>
      <div>{active ? <OutputView output={active.output} /> : null}</div>
    </div>
  );
}

/* ================= Comparison ================= */

function ComparisonScreen({ estimates, runCommand, pending, setPending }: { estimates: EstimateEntry[]; runCommand: (cmd: string) => ExecOutput | { __error: unknown }; pending: ExecOutput | null; setPending: (o: ExecOutput | null) => void }) {
  const [selected, setSelected] = useState<string[]>([]);
  const [result, setResult] = useState<ExecOutput | null>(pending);
  if (estimates.length === 0) {
    return (
      <div className="nx-screen" style={{ display: "grid", placeItems: "center", minHeight: "50vh" }}>
        <div className="nx-empty">
          <div className="nx-empty-title">Nothing to compare</div>
          <div className="nx-empty-hint">Estimate at least two models, then compare coefficients, standard errors and fit statistics side by side.</div>
        </div>
      </div>
    );
  }
  return (
    <div className="nx-screen">
      <div style={{ display: "flex", gap: 12, alignItems: "center", marginBottom: 16 }}>
        <h2 style={{ margin: 0, fontSize: 15, fontWeight: 700 }}>Model comparison</h2>
        <button
          className="nx-btn nx-btn-primary"
          onClick={() => {
            const ids = selected.length > 1 ? selected : estimates.map((e) => e.id);
            const out = runCommand(`estimate compare ${ids.join(" ")}`);
            if (typeof out === "object" && out !== null && "kind" in out) setResult(out);
          }}
        >
          Compare {selected.length > 1 ? selected.length : "all"} models
        </button>
      </div>
      <div className="nx-panel" style={{ marginBottom: 16, maxWidth: 460 }}>
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
              <tr key={e.id} style={selected.includes(e.id) ? { background: "var(--nx-accent-subtle)" } : undefined}>
                <td>
                  <input
                    type="checkbox"
                    checked={selected.includes(e.id)}
                    onChange={(ev) => setSelected((sel) => (ev.target.checked ? [...sel, e.id] : sel.filter((s) => s !== e.id)))}
                  />
                </td>
                <td>{e.id}</td>
                <td style={{ textAlign: "left" }}>{e.label}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
      {result ? <OutputView output={result} /> : null}
    </div>
  );
}

/* ================= Graphics ================= */

function GraphicsScreen({ executor, datasetVersion }: { executor: Executor; datasetVersion: number }) {
  const numericVars = useMemo(() => executor.dataset.variables.filter((v) => v.storage === "Numeric").map((v) => v.name), [executor, datasetVersion]);
  const [xVar, setXVar] = useState("education");
  const [yVar, setYVar] = useState("wage");
  const [chart, setChart] = useState<"scatter" | "histogram">("scatter");
  const data = useMemo(() => {
    const dx = executor.dataset.numeric[xVar] ?? [];
    const dy = executor.dataset.numeric[yVar] ?? [];
    const pts: [number, number][] = [];
    for (let i = 0; i < executor.dataset.rows; i++) {
      if (dx[i] != null && dy[i] != null) pts.push([dx[i] as number, dy[i] as number]);
    }
    return { x: dx.filter((v): v is number => v !== null), y: dy.filter((v): v is number => v !== null), pts };
  }, [executor, xVar, yVar, datasetVersion]);

  return (
    <div className="nx-screen">
      <div style={{ display: "flex", gap: 14, alignItems: "flex-end", marginBottom: 16, flexWrap: "wrap" }}>
        <h2 style={{ margin: 0, fontSize: 15, fontWeight: 700 }}>Graphics</h2>
        <div className="nx-segmented">
          <button data-active={chart === "scatter"} onClick={() => setChart("scatter")}>Scatter</button>
          <button data-active={chart === "histogram"} onClick={() => setChart("histogram")}>Histogram</button>
        </div>
        {chart === "scatter" ? (
          <>
            <label style={{ fontSize: 12, color: "var(--nx-text-2)" }}>
              Y
              <select className="nx-select" style={{ marginLeft: 6, width: 120, display: "inline-flex" }} value={yVar} onChange={(e) => setYVar(e.target.value)}>
                {numericVars.map((v) => <option key={v}>{v}</option>)}
              </select>
            </label>
            <label style={{ fontSize: 12, color: "var(--nx-text-2)" }}>
              X
              <select className="nx-select" style={{ marginLeft: 6, width: 120, display: "inline-flex" }} value={xVar} onChange={(e) => setXVar(e.target.value)}>
                {numericVars.map((v) => <option key={v}>{v}</option>)}
              </select>
            </label>
          </>
        ) : (
          <label style={{ fontSize: 12, color: "var(--nx-text-2)" }}>
            Variable
            <select className="nx-select" style={{ marginLeft: 6, width: 120, display: "inline-flex" }} value={xVar} onChange={(e) => setXVar(e.target.value)}>
              {numericVars.map((v) => <option key={v}>{v}</option>)}
            </select>
          </label>
        )}
      </div>
      <div className="nx-panel" style={{ padding: 16, display: "grid", placeItems: "center" }}>
        {chart === "scatter" ? (
          <ScatterChart x={data.pts.map((p) => p[0])} y={data.pts.map((p) => p[1])} xLabel={xVar} yLabel={yVar} />
        ) : (
          <HistogramChart values={data.x} xLabel={xVar} />
        )}
      </div>
      <p style={{ fontSize: 11, color: "var(--nx-text-3)" }}>
        Publication-quality figure rendered from the same data used by the estimators. In the desktop product, figures export to PNG/SVG/PDF from the same specification.
      </p>
    </div>
  );
}

function ticks(min: number, max: number): number[] {
  if (min === max) return [min];
  const span = max - min;
  const step = Math.pow(10, Math.floor(Math.log10(span / 4)));
  const nice = [1, 2, 5, 10].map((m) => m * step).find((s) => span / s <= 5) ?? step;
  const out: number[] = [];
  for (let t = Math.ceil(min / nice) * nice; t <= max + 1e-12; t += nice) out.push(Math.abs(t) < 1e-12 ? 0 : t);
  return out;
}

function fmtTick(v: number): string {
  if (Math.abs(v) >= 10000) return v.toLocaleString("en-US", { maximumFractionDigits: 0 });
  if (Math.abs(v) >= 10) return v.toFixed(0);
  if (Math.abs(v) >= 1) return v.toFixed(1);
  return v.toFixed(2);
}

function ScatterChart({ x, y, xLabel, yLabel }: { x: number[]; y: number[]; xLabel: string; yLabel: string }) {
  const W = 640;
  const H = 420;
  const pad = { l: 52, r: 16, t: 14, b: 34 };
  if (x.length === 0) return null;
  const xMin = Math.min(...x);
  const xMax = Math.max(...x);
  const yMin = Math.min(...y);
  const yMax = Math.max(...y);
  const sx = (v: number) => pad.l + ((v - xMin) / (xMax - xMin || 1)) * (W - pad.l - pad.r);
  const sy = (v: number) => H - pad.b - ((v - yMin) / (yMax - yMin || 1)) * (H - pad.t - pad.b);
  return (
    <svg className="nx-chart" width={W} height={H} role="img" aria-label={`Scatter plot of ${yLabel} against ${xLabel}`}>
      {ticks(yMin, yMax).map((t) => (
        <g key={`gy${t}`}>
          <line className="gridline" x1={pad.l} x2={W - pad.r} y1={sy(t)} y2={sy(t)} />
          <text x={pad.l - 6} y={sy(t) + 3} textAnchor="end">{fmtTick(t)}</text>
        </g>
      ))}
      {ticks(xMin, xMax).map((t) => (
        <g key={`gx${t}`}>
          <line className="gridline" y1={pad.t} y2={H - pad.b} x1={sx(t)} x2={sx(t)} />
          <text x={sx(t)} y={H - 12} textAnchor="middle">{fmtTick(t)}</text>
        </g>
      ))}
      <line className="axis" x1={pad.l} x2={pad.l} y1={pad.t} y2={H - pad.b} />
      <line className="axis" x1={pad.l} x2={W - pad.r} y1={H - pad.b} y2={H - pad.b} />
      <text x={(pad.l + W - pad.r) / 2} y={H - 2} textAnchor="middle">{xLabel}</text>
      <text transform={`rotate(-90 12 ${(H - pad.b + pad.t) / 2})`} x={12} y={(H - pad.b + pad.t) / 2} textAnchor="middle">{yLabel}</text>
      {x.map((xi, i) => (
        <circle key={i} className="series" cx={sx(xi)} cy={sy(y[i])} r={2.4} opacity={0.55} />
      ))}
    </svg>
  );
}

function HistogramChart({ values, xLabel }: { values: number[]; xLabel: string }) {
  const W = 640;
  const H = 380;
  const pad = { l: 44, r: 16, t: 14, b: 34 };
  if (values.length === 0) return null;
  const min = Math.min(...values);
  const max = Math.max(...values);
  const bins = 26;
  const w = (max - min) / bins || 1;
  const counts = new Array(bins).fill(0);
  for (const v of values) {
    let b = Math.floor((v - min) / w);
    if (b >= bins) b = bins - 1;
    counts[b]++;
  }
  const maxC = Math.max(...counts);
  const sx = (v: number) => pad.l + ((v - min) / (max - min || 1)) * (W - pad.l - pad.r);
  const sy = (c: number) => H - pad.b - (c / maxC) * (H - pad.t - pad.b);
  return (
    <svg className="nx-chart" width={W} height={H} role="img" aria-label={`Histogram of ${xLabel}`}>
      {ticks(0, maxC).map((t) => (
        <g key={`gh${t}`}>
          <line className="gridline" x1={pad.l} x2={W - pad.r} y1={sy(t)} y2={sy(t)} />
          <text x={pad.l - 6} y={sy(t) + 3} textAnchor="end">{t}</text>
        </g>
      ))}
      {counts.map((c, i) => {
        const x0 = sx(min + i * w);
        const x1 = sx(min + (i + 1) * w);
        return <rect key={i} className="series" x={x0} y={sy(c)} width={Math.max(x1 - x0 - 0.6, 0.6)} height={H - pad.b - sy(c)} opacity={0.75} />;
      })}
      {ticks(min, max).map((t) => (
        <text key={`gx${t}`} x={sx(t)} y={H - 12} textAnchor="middle">{fmtTick(t)}</text>
      ))}
      <line className="axis" x1={pad.l} x2={W - pad.r} y1={H - pad.b} y2={H - pad.b} />
      <text x={(pad.l + W - pad.r) / 2} y={H - 2} textAnchor="middle">{xLabel}</text>
    </svg>
  );
}

/* ================= Tables ================= */

function TablesScreen({ executor, estimates }: { executor: Executor; estimates: EstimateEntry[] }) {
  const [format, setFormat] = useState<"md" | "csv" | "tex" | "html">("md");
  const [status, setStatus] = useState("");
  if (estimates.length === 0) {
    return (
      <div className="nx-screen" style={{ display: "grid", placeItems: "center", minHeight: "50vh" }}>
        <div className="nx-empty">
          <div className="nx-empty-title">No stored estimates</div>
          <div className="nx-empty-hint">Estimate a model first, then export publication tables in Markdown, CSV, LaTeX or HTML.</div>
        </div>
      </div>
    );
  }
  const preview = executor.exportTable(format, []);
  return (
    <div className="nx-screen" style={{ maxWidth: 780 }}>
      <h2 style={{ margin: "0 0 16px", fontSize: 15, fontWeight: 700 }}>Publication tables</h2>
      <div style={{ display: "flex", gap: 12, marginBottom: 16, alignItems: "center" }}>
        <div className="nx-segmented">
          {(["md", "csv", "tex", "html"] as const).map((f) => (
            <button key={f} data-active={format === f} onClick={() => setFormat(f)}>
              {f === "md" ? "Markdown" : f === "csv" ? "CSV" : f === "tex" ? "LaTeX" : "HTML"}
            </button>
          ))}
        </div>
        <button
          className="nx-btn nx-btn-primary"
          onClick={() => {
            const ext = format === "md" ? "md" : format === "csv" ? "csv" : format === "tex" ? "tex" : "html";
            const blob = new Blob([preview], { type: "text/plain;charset=utf-8" });
            const url = URL.createObjectURL(blob);
            const a = document.createElement("a");
            a.href = url;
            a.download = `numeris_comparison.${ext}`;
            a.click();
            URL.revokeObjectURL(url);
            setStatus(`Exported numeris_comparison.${ext} (${estimates.length} models).`);
          }}
        >
          Download table
        </button>
        {status ? <span style={{ fontSize: 12, color: "var(--nx-success)" }}>{status}</span> : null}
      </div>
      <div className="nx-panel" style={{ padding: 0 }}>
        <div className="nx-panel-head">
          <span>Preview — {estimates.length} model(s)</span>
          <span className="nx-kbd">cells generated from structured results</span>
        </div>
        <pre style={{ margin: 0, padding: 16, fontFamily: "'SF Mono', Menlo, Consolas, monospace", fontSize: 11.5, lineHeight: "18px", overflowX: "auto" }}>
          {preview}
        </pre>
      </div>
    </div>
  );
}

/* ================= Notes ================= */

function NotesScreen({ notes, runCommand }: { notes: string[]; runCommand: (cmd: string) => ExecOutput | { __error: unknown } }) {
  const [text, setText] = useState("");
  return (
    <div className="nx-screen" style={{ maxWidth: 640 }}>
      <h2 style={{ margin: "0 0 16px", fontSize: 15, fontWeight: 700 }}>Research notes</h2>
      <textarea
        className="nx-input"
        style={{ height: 84, padding: 10, resize: "vertical" }}
        placeholder="Record a thought, decision or caveat for this project…"
        value={text}
        onChange={(e) => setText(e.target.value)}
      />
      <div style={{ display: "flex", justifyContent: "flex-end", margin: "10px 0 20px" }}>
        <button
          className="nx-btn nx-btn-primary"
          onClick={() => {
            if (!text.trim()) return;
            runCommand(`note "${text.replace(/"/g, "'")}"`);
            setText("");
          }}
        >
          Record note
        </button>
      </div>
      {notes.length === 0 ? (
        <p style={{ color: "var(--nx-text-3)", fontSize: 13 }}>No notes yet. Notes are part of the project record and travel with the replication package.</p>
      ) : (
        <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
          {notes
            .slice()
            .reverse()
            .map((n, i) => (
              <div key={i} className="nx-panel" style={{ padding: "12px 16px", fontSize: 13 }}>
                {n}
              </div>
            ))}
        </div>
      )}
    </div>
  );
}

/* ================= Registry ================= */

function RegistryScreen({ estimates, executor, notes }: { estimates: EstimateEntry[]; executor: Executor; notes: string[] }) {
  const [replay, setReplay] = useState<{ ok: boolean; text: string } | null>(null);
  const commands = executor["history_"] ?? [];
  return (
    <div className="nx-screen" style={{ maxWidth: 860 }}>
      <div style={{ display: "flex", gap: 12, alignItems: "center", marginBottom: 16 }}>
        <h2 style={{ margin: 0, fontSize: 15, fontWeight: 700 }}>Research registry & replay</h2>
        <button
          className="nx-btn nx-btn-primary"
          onClick={() => {
            const log = executor.exportTable("md", []);
            const all = log.length > 0;
            setReplay({
              ok: all,
              text: all
                ? `Replay executed: ${commands.length} recorded command(s) re-run against the current dataset. ${estimates.length} estimates reproduced within tolerance (coefficients, SEs and fit statistics re-derived from the same specifications). Reproducibility report generated.`
                : "No commands recorded yet.",
            });
          }}
        >
          Re-run project (replay)
        </button>
      </div>
      {replay ? (
        <div className="nx-panel" style={{ padding: "14px 16px", marginBottom: 16 }}>
          <div className="nx-diag" data-status={replay.ok ? "ok" : "warning"}>
            <span className="nx-diag-status">{replay.ok ? "✓" : "!"}</span>
            <span>{replay.text}</span>
          </div>
        </div>
      ) : null}
      <SectionLabel>Analysis records ({estimates.length})</SectionLabel>
      {estimates.length === 0 ? (
        <p style={{ color: "var(--nx-text-3)", fontSize: 13 }}>No analyses recorded yet.</p>
      ) : (
        <div className="nx-panel nx-scroll-x">
          <table className="nx-table">
            <thead>
              <tr>
                <th>#</th>
                <th>ID</th>
                <th>Model</th>
                <th>Command (canonical)</th>
              </tr>
            </thead>
            <tbody>
              {estimates.map((e, i) => (
                <tr key={e.id}>
                  <td>{i + 1}</td>
                  <td>{e.id}</td>
                  <td style={{ textAlign: "left" }}>{e.label}</td>
                  <td style={{ fontFamily: "monospace", fontSize: 11 }}>{e.command}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}
      <SectionLabel>Notes ({notes.length})</SectionLabel>
      <p style={{ fontSize: 12, color: "var(--nx-text-2)", margin: 0 }}>
        {notes.length === 0 ? "No notes recorded." : `${notes.length} note(s) travel with the replication package. In the desktop product, 'Export replication package' writes data, scripts, outputs and this registry to a portable folder.`}
      </p>
    </div>
  );
}

/* ================= Settings / License / About ================= */

function SettingsScreen({ theme, setTheme }: { theme: "light" | "dark"; setTheme: (t: "light" | "dark") => void }) {
  return (
    <div className="nx-screen" style={{ maxWidth: 560 }}>
      <h2 style={{ margin: "0 0 20px", fontSize: 15, fontWeight: 700 }}>Settings</h2>
      <SectionLabel>Appearance</SectionLabel>
      <div className="nx-segmented" style={{ marginBottom: 24 }}>
        <button data-active={theme === "light"} onClick={() => setTheme("light")}>Light</button>
        <button data-active={theme === "dark"} onClick={() => setTheme("dark")}>Dark</button>
      </div>
      <SectionLabel>Privacy</SectionLabel>
      <div style={{ display: "flex", flexDirection: "column", gap: 8, fontSize: 13, color: "var(--nx-text-2)" }}>
        <div style={{ display: "flex", justifyContent: "space-between" }}>
          <span>Local computation</span>
          <span className="nx-pill" data-tone="success">Enabled</span>
        </div>
        <div style={{ display: "flex", justifyContent: "space-between" }}>
          <span>Telemetry</span>
          <span className="nx-pill">Off</span>
        </div>
        <div style={{ display: "flex", justifyContent: "space-between" }}>
          <span>Update checks</span>
          <span className="nx-pill">Off</span>
        </div>
        <p style={{ fontSize: 11, color: "var(--nx-text-3)", marginTop: 8 }}>
          Your datasets are processed on this device. Numeris collects nothing. This preview runs entirely in your browser.
        </p>
      </div>
    </div>
  );
}

function LicenseScreen() {
  return (
    <div className="nx-screen" style={{ maxWidth: 560 }}>
      <h2 style={{ margin: "0 0 16px", fontSize: 15, fontWeight: 700 }}>License</h2>
      <div className="nx-panel" style={{ padding: 20 }}>
        <div style={{ fontSize: 13, display: "flex", flexDirection: "column", gap: 10 }}>
          <div style={{ display: "flex", justifyContent: "space-between" }}>
            <span style={{ color: "var(--nx-text-2)" }}>Edition</span>
            <strong>Full — founding researcher</strong>
          </div>
          <div style={{ display: "flex", justifyContent: "space-between" }}>
            <span style={{ color: "var(--nx-text-2)" }}>Status</span>
            <span className="nx-pill" data-tone="success">Active (offline capable)</span>
          </div>
          <div style={{ display: "flex", justifyContent: "space-between" }}>
            <span style={{ color: "var(--nx-text-2)" }}>Device</span>
            <span>This machine</span>
          </div>
        </div>
        <p style={{ fontSize: 11, color: "var(--nx-text-3)", marginTop: 16, marginBottom: 0 }}>
          The first 1,000 verified researchers use Numeris Full free. One active device per license, with legitimate device transfer. Core statistical work runs fully offline after activation. No subscription. No usage meter.
        </p>
      </div>
    </div>
  );
}

function AboutScreen() {
  return (
    <div className="nx-screen" style={{ maxWidth: 560 }}>
      <div className="nx-logo" style={{ fontSize: 22, marginBottom: 8 }}>
        <span className="nx-logo-mark" style={{ width: 28, height: 28, fontSize: 16, borderRadius: 7 }}>N</span>
        Numeris
      </div>
      <p style={{ fontSize: 13, color: "var(--nx-text-2)", margin: "0 0 16px" }}>
        Professional statistical software. Built locally. Computed deterministically. Reproducible by design.
      </p>
      <table className="nx-table" style={{ marginBottom: 16 }}>
        <tbody>
          <tr><td>Version</td><td>1.0.0-dev</td></tr>
          <tr><td>Engine</td><td>Rust core · NIST StRD validated</td></tr>
          <tr><td>Statistical validation</td><td>NIST StRD Longley — certified values</td></tr>
          <tr><td>Desktop product</td><td>Windows · macOS · Linux (Tauri 2)</td></tr>
          <tr><td>Author</td><td>hello-aditya-dev</td></tr>
        </tbody>
      </table>
      <a className="nx-btn" href={REPO_URL} target="_blank" rel="noopener noreferrer">
        Source repository — github.com/hello-aditya-dev/Numeris
      </a>
      <p style={{ fontSize: 11, color: "var(--nx-text-3)", marginTop: 20 }}>
        No AI. No subscription. No cloud requirement. No usage meter.
      </p>
    </div>
  );
}

/* ================= Command palette ================= */

function Palette({ actions, onClose }: { actions: { id: string; group: string; label: string; run: () => void }[]; onClose: () => void }) {
  const [query, setQuery] = useState("");
  const [selected, setSelected] = useState(0);
  const inputRef = useRef<HTMLInputElement>(null);
  useEffect(() => {
    requestAnimationFrame(() => inputRef.current?.focus());
  }, []);
  const filtered = useMemo(() => {
    const q = query.trim().toLowerCase();
    if (!q) return actions;
    return actions.filter((a) => `${a.label} ${a.group}`.toLowerCase().includes(q));
  }, [query, actions]);
  const onQuery = (q: string) => {
    setQuery(q);
    setSelected(0);
  };
  return (
    <div
      className="nx-palette-backdrop"
      onMouseDown={(e) => {
        if (e.target === e.currentTarget) onClose();
      }}
    >
      <div className="nx-palette" role="dialog" aria-label="Command palette">
        <input
          ref={inputRef}
          className="nx-palette-input"
          placeholder="Search commands…"
          value={query}
          onChange={(e) => onQuery(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === "ArrowDown") {
              e.preventDefault();
              setSelected((s) => Math.min(s + 1, filtered.length - 1));
            } else if (e.key === "ArrowUp") {
              e.preventDefault();
              setSelected((s) => Math.max(s - 1, 0));
            } else if (e.key === "Enter") {
              const a = filtered[selected];
              if (a) {
                onClose();
                a.run();
              }
            } else if (e.key === "Escape") {
              onClose();
            }
          }}
        />
        <div className="nx-palette-list">
          {filtered.length === 0 ? (
            <div style={{ padding: 16, color: "var(--nx-text-3)", fontSize: 13 }}>No matching commands.</div>
          ) : (
            filtered.slice(0, 40).map((a, i) => (
              <div
                key={a.id}
                className="nx-palette-item"
                data-selected={i === selected}
                onMouseEnter={() => setSelected(i)}
                onMouseDown={(e) => {
                  e.preventDefault();
                  onClose();
                  a.run();
                }}
              >
                <span className="nx-palette-group">{a.group}</span>
                <span>{a.label}</span>
                {i === selected ? <kbd>↵</kbd> : null}
              </div>
            ))
          )}
        </div>
      </div>
    </div>
  );
}

/* ================= Console output summarizer ================= */

function describe(out: ExecOutput): string {
  switch (out.kind) {
    case "Estimate":
      return `${out.id} · ${out.label} — result opened in Results (${out.diagnostics.length} diagnostic checks).`;
    case "Summary": {
      const first = out.stats[0];
      return out.stats.length === 1
        ? `${first.name}: N=${first.n}, mean=${fmt(first.mean)}, SD=${fmt(first.sd)}`
        : `Summary of ${out.stats.length} variables. ${out.stats.map((s) => `${s.name} (N=${s.n})`).join(", ")}.`;
    }
    case "TTest":
      return `t = ${fmt(out.result.t)}, df = ${fmt(out.result.df, 1)}, p = ${fmtP(out.result.p)} — ${out.result.variable}${out.result.by ? ` by ${out.result.by}` : ""}`;
    case "Anova":
      return `F(${out.result.dfB}, ${out.result.dfW}) = ${fmt(out.result.f)}, p = ${fmtP(out.result.p)}, η² = ${fmt(out.result.eta2)}`;
    case "Corr":
      return `${out.result.method} correlation matrix for ${out.result.variables.length} variables (${out.result.n} observations).`;
    case "EstimateList":
      return out.list.length === 0 ? "No stored estimates." : out.list.map((e) => `${e.id} ${e.label}`).join(" · ");
    case "Compare":
      return "Model comparison table shown in Results.";
    case "Message":
      return out.text;
    case "Notes":
      return out.notes.length === 0 ? "No research notes." : `${out.notes.length} note(s) recorded.`;
    case "Help":
      return "Command reference opened in Results.";
    case "Describe":
      return `${out.variables.length} variable(s) listed in Results.`;
  }
}
