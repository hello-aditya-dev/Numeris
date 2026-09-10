"use client";

// Formatting helpers + shared view primitives for the Numeris preview.

import React from "react";
import type { EstimateResult, CompareResult, DiagnosticRow, SummaryRow, ExecOutput } from "@/lib/numeris/executor";
import type { CorrResult, TTestResult, AnovaResult, Regression, GlmResult, PanelResult } from "@/lib/numeris/stats";

export function fmt(v: number | null | undefined, digits = 3): string {
  if (v === null || v === undefined || Number.isNaN(v)) return "—";
  if (!Number.isFinite(v)) return v > 0 ? "∞" : "−∞";
  if (v === 0) return "0";
  const abs = Math.abs(v);
  if (abs >= 1e7 || abs < 1e-4) return v.toExponential(2);
  return v.toFixed(digits).replace(/\.?0+$/, "");
}

export function fmtP(p: number | null | undefined): string {
  if (p === null || p === undefined || Number.isNaN(p)) return "—";
  if (p < 0.001) return "<0.001";
  return p.toFixed(3);
}

export function stars(p: number | null | undefined): string {
  if (p === null || p === undefined || Number.isNaN(p)) return "";
  if (p < 0.001) return "***";
  if (p < 0.01) return "**";
  if (p < 0.05) return "*";
  return "";
}

export function fmtInt(v: number): string {
  return v.toLocaleString("en-US");
}

/* ---------- Shared render pieces ---------- */

export function Stat({ label, value }: { label: string; value: string }) {
  return (
    <div className="nx-fitstats-cell">
      <div className="nx-fitstats-label">{label}</div>
      <div className="nx-fitstats-value">{value}</div>
    </div>
  );
}

export function SectionLabel({ children }: { children: React.ReactNode }) {
  return <div className="nx-section-label">{children}</div>;
}

export interface CoefRow {
  term: string;
  coef: number;
  se: number;
  stat: number;
  p: number;
  ciLo?: number;
  ciHi?: number;
}

export function CoefTable({ rows, statLabel = "t" }: { rows: CoefRow[]; statLabel?: string }) {
  return (
    <table className="nx-table">
      <thead>
        <tr>
          <th>Term</th>
          <th>Coef.</th>
          <th>Std. err.</th>
          <th>{statLabel}</th>
          <th>p&gt;|{statLabel}|</th>
          <th>[95% conf.]</th>
        </tr>
      </thead>
      <tbody>
        {rows.map((r) => (
          <tr key={r.term}>
            <td>{r.term}</td>
            <td>
              {fmt(r.coef)}
              <span className="nx-signif">{stars(r.p)}</span>
            </td>
            <td>{fmt(r.se)}</td>
            <td>{fmt(r.stat)}</td>
            <td>{fmtP(r.p)}</td>
            <td>{r.ciLo !== undefined ? `${fmt(r.ciLo)} … ${fmt(r.ciHi)}` : "—"}</td>
          </tr>
        ))}
      </tbody>
    </table>
  );
}

export function DiagnosticsList({ diagnostics }: { diagnostics: DiagnosticRow[] }) {
  return (
    <div>
      {diagnostics.map((d) => (
        <div key={d.name} className="nx-diag" data-status={d.status}>
          <span className="nx-diag-status">
            {d.status === "ok" ? "✓" : d.status === "warning" ? "!" : d.status === "attention" ? "✕" : "i"}
          </span>
          <span className="nx-diag-name">{d.name}</span>
          <span style={{ color: "var(--nx-text-2)" }}>{d.detail}</span>
        </div>
      ))}
    </div>
  );
}

/* ---------- Estimate document (regression-family) ---------- */

export function EstimateDocument({ id, label, result, diagnostics }: { id: string; label: string; result: EstimateResult; diagnostics: DiagnosticRow[] }) {
  if ("Ols" in result) return <OlsDoc id={id} r={result.Ols} diagnostics={diagnostics} />;
  if ("Glm" in result) return <GlmDoc id={id} r={result.Glm} diagnostics={diagnostics} />;
  if ("Panel" in result) return <PanelDoc id={id} r={result.Panel} diagnostics={diagnostics} />;
  if ("Iv" in result) return <IvDoc id={id} r={result.Iv} diagnostics={diagnostics} />;
  return <DidDoc id={id} r={result.Did} diagnostics={diagnostics} />;
}

function OlsDoc({ id, r, diagnostics }: { id: string; r: Regression; diagnostics: DiagnosticRow[] }) {
  return (
    <div className="nx-doc">
      <h3 className="nx-doc-title">{r.estimatorLabel} regression</h3>
      <div className="nx-doc-sub">
        {id} · Variance estimator: {r.vcovLabel} · QR decomposition
      </div>
      <div className="nx-fitstats">
        <Stat label="N" value={fmtInt(r.n)} />
        <Stat label="R²" value={fmt(r.r2, 4)} />
        <Stat label="Adjusted R²" value={fmt(r.adjR2, 4)} />
        <Stat label="Root MSE" value={fmt(r.rmse)} />
        <Stat label="F" value={r.f === null ? "—" : fmt(r.f)} />
        {r.fP !== null && r.fP !== undefined ? <Stat label="p (F)" value={fmtP(r.fP)} /> : null}
        <Stat label="AIC" value={fmt(r.aic, 1)} />
        <Stat label="BIC" value={fmt(r.bic, 1)} />
      </div>
      <SectionLabel>Coefficients</SectionLabel>
      <CoefTable
        rows={r.terms.map((t, j) => ({
          term: t,
          coef: r.coef[j],
          se: r.se[j],
          stat: r.t[j],
          p: r.p[j],
          ciLo: r.ciLo[j],
          ciHi: r.ciHi[j],
        }))}
      />
      {diagnostics.length > 0 ? (
        <>
          <SectionLabel>Diagnostics</SectionLabel>
          <DiagnosticsList diagnostics={diagnostics} />
        </>
      ) : null}
    </div>
  );
}

function GlmDoc({ id, r, diagnostics }: { id: string; r: GlmResult; diagnostics: DiagnosticRow[] }) {
  const title = r.family === "Logit" ? "Logistic regression" : r.family === "Probit" ? "Probit regression" : "Poisson regression";
  return (
    <div className="nx-doc">
      <h3 className="nx-doc-title">{title}</h3>
      <div className="nx-doc-sub">
        {id} · {r.vcovLabel} · {r.iterations} iterations{r.converged ? "" : " · NOT CONVERGED"}
      </div>
      <div className="nx-fitstats">
        <Stat label="N" value={fmtInt(r.n)} />
        <Stat label="Pseudo R²" value={fmt(r.pseudoR2, 4)} />
        <Stat label="Log-likelihood" value={fmt(r.loglik, 2)} />
        <Stat label="AIC" value={fmt(r.aic, 1)} />
        <Stat label="BIC" value={fmt(r.bic, 1)} />
      </div>
      <SectionLabel>Coefficients</SectionLabel>
      <CoefTable
        statLabel="z"
        rows={r.terms.map((t, j) => ({ term: t, coef: r.coef[j], se: r.se[j], stat: r.z[j], p: r.p[j], ciLo: r.ciLo[j], ciHi: r.ciHi[j] }))}
      />
      {r.warnings.length > 0 ? (
        <>
          <SectionLabel>Estimation warnings</SectionLabel>
          {r.warnings.map((w, i) => (
            <div key={i} className="nx-diag" data-status="warning">
              <span className="nx-diag-status">!</span>
              <span style={{ color: "var(--nx-text-2)" }}>{w}</span>
            </div>
          ))}
        </>
      ) : null}
      {diagnostics.length > 0 ? (
        <>
          <SectionLabel>Diagnostics</SectionLabel>
          <DiagnosticsList diagnostics={diagnostics} />
        </>
      ) : null}
    </div>
  );
}

function PanelDoc({ id, r, diagnostics }: { id: string; r: PanelResult; diagnostics: DiagnosticRow[] }) {
  return (
    <div className="nx-doc">
      <h3 className="nx-doc-title">{r.estimator}</h3>
      <div className="nx-doc-sub">
        {id} · {r.vcovLabel} · entities: {r.g}
        {r.droppedSingletons > 0 ? ` · ${r.droppedSingletons} singleton(s) dropped` : ""} · df = {r.dfR}
      </div>
      <div className="nx-fitstats">
        <Stat label="N" value={fmtInt(r.n)} />
        <Stat label="Entities" value={String(r.g)} />
        <Stat label="R² (within)" value={fmt(r.r2Within, 4)} />
        <Stat label="sigma" value={fmt(r.sigma)} />
      </div>
      <SectionLabel>Coefficients</SectionLabel>
      <CoefTable
        rows={r.terms.map((t, j) => ({ term: t, coef: r.coef[j], se: r.se[j], stat: r.t[j], p: r.p[j], ciLo: r.ciLo[j], ciHi: r.ciHi[j] }))}
      />
      {diagnostics.length > 0 ? (
        <>
          <SectionLabel>Diagnostics</SectionLabel>
          <DiagnosticsList diagnostics={diagnostics} />
        </>
      ) : null}
    </div>
  );
}

function IvDoc({ id, r, diagnostics }: { id: string; r: import("@/lib/numeris/executor").IvResult; diagnostics: DiagnosticRow[] }) {
  return (
    <div className="nx-doc">
      <h3 className="nx-doc-title">2SLS instrumental variables</h3>
      <div className="nx-doc-sub">{id} · {r.vcovLabel}</div>
      <div className="nx-fitstats">
        <Stat label="N" value={fmtInt(r.n)} />
        {r.firstStageTerms.map((t, j) => (
          <Stat key={`f${j}`} label={`First-stage F (${t})`} value={fmt(r.firstStageF[j])} />
        ))}
        {r.firstStageTerms.map((t, j) => (
          <Stat key={`p${j}`} label={`Partial R² (${t})`} value={fmt(r.firstStagePartialR2[j])} />
        ))}
      </div>
      <SectionLabel>Coefficients</SectionLabel>
      <CoefTable
        statLabel="t"
        rows={r.terms.map((t, j) => ({ term: t, coef: r.coef[j], se: r.se[j], stat: r.z[j], p: r.p[j], ciLo: r.ciLo[j], ciHi: r.ciHi[j] }))}
      />
      <p style={{ fontSize: 11, color: "var(--nx-text-3)", margin: "8px 0 0" }}>
        First-stage F above 10 is the conventional rule of thumb for instrument strength; weak instruments bias 2SLS toward OLS.
      </p>
      {diagnostics.length > 0 ? (
        <>
          <SectionLabel>Diagnostics</SectionLabel>
          <DiagnosticsList diagnostics={diagnostics} />
        </>
      ) : null}
    </div>
  );
}

function DidDoc({ id, r, diagnostics }: { id: string; r: import("@/lib/numeris/executor").DidResult; diagnostics: DiagnosticRow[] }) {
  const cells = [
    ["Control, pre", 0],
    ["Control, post", 1],
    ["Treated, pre", 2],
    ["Treated, post", 3],
  ] as const;
  return (
    <div className="nx-doc">
      <h3 className="nx-doc-title">Difference-in-differences</h3>
      <div className="nx-doc-sub">
        {id} · {r.vcovLabel} · interaction term treat#post is the ATT
      </div>
      <div className="nx-fitstats">
        <Stat label="N" value={fmtInt(r.n)} />
        <Stat label="ATT" value={fmt(r.att)} />
        <Stat label="SE" value={fmt(r.attSe)} />
        <Stat label="p" value={fmtP(r.p)} />
        <Stat label="95% CI" value={`${fmt(r.ciLo)} … ${fmt(r.ciHi)}`} />
      </div>
      <SectionLabel>Cell means</SectionLabel>
      <table className="nx-table">
        <thead>
          <tr>
            <th>Cell</th>
            <th>N</th>
            <th>Mean</th>
          </tr>
        </thead>
        <tbody>
          {cells.map(([name, idx]) => (
            <tr key={name}>
              <td>{name}</td>
              <td>{r.means[idx][1]}</td>
              <td>{fmt(r.means[idx][0])}</td>
            </tr>
          ))}
        </tbody>
      </table>
      <SectionLabel>Underlying regression</SectionLabel>
      <CoefTable
        rows={r.terms.map((t, j) => ({ term: t, coef: r.coef[j], se: r.se[j], stat: 0, p: r.pValues[j] }))}
      />
      <p style={{ fontSize: 11, color: "var(--nx-text-3)", margin: "8px 0 0" }}>
        The DID design assumes parallel trends between treated and control groups in the absence of treatment.
      </p>
      {diagnostics.length > 0 ? (
        <>
          <SectionLabel>Diagnostics</SectionLabel>
          <DiagnosticsList diagnostics={diagnostics} />
        </>
      ) : null}
    </div>
  );
}

/* ---------- Other output views ---------- */

export function SummaryView({ stats }: { stats: SummaryRow[] }) {
  return (
    <div className="nx-panel nx-scroll-x">
      <table className="nx-table">
        <thead>
          <tr>
            <th>Variable</th>
            <th>N</th>
            <th>Missing</th>
            <th>Mean</th>
            <th>SD</th>
            <th>Min</th>
            <th>p25</th>
            <th>Median</th>
            <th>p75</th>
            <th>Max</th>
            <th>Skew.</th>
            <th>Kurt.</th>
          </tr>
        </thead>
        <tbody>
          {stats.map((s) => (
            <tr key={s.name}>
              <td>{s.name}</td>
              <td>{s.n}</td>
              <td>{s.missing}</td>
              <td>{fmt(s.mean)}</td>
              <td>{fmt(s.sd)}</td>
              <td>{fmt(s.min)}</td>
              <td>{fmt(s.q1)}</td>
              <td>{fmt(s.median)}</td>
              <td>{fmt(s.q3)}</td>
              <td>{fmt(s.max)}</td>
              <td>{fmt(s.skewness)}</td>
              <td>{fmt(s.kurtosis)}</td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}

export function CorrView({ r }: { r: CorrResult }) {
  return (
    <div className="nx-panel nx-scroll-x">
      <div className="nx-panel-head">
        <span>
          {r.method} correlation · {r.n} observations
        </span>
        <span className="nx-kbd">cell tooltips show p-values</span>
      </div>
      <table className="nx-table">
        <thead>
          <tr>
            <th />
            {r.variables.map((v) => (
              <th key={v}>{v}</th>
            ))}
          </tr>
        </thead>
        <tbody>
          {r.variables.map((rv, i) => (
            <tr key={rv}>
              <td>{rv}</td>
              {r.variables.map((cv, j) => {
                const v = r.values[i]?.[j] ?? null;
                const p = r.pValues[i]?.[j] ?? null;
                const bg =
                  v === null
                    ? undefined
                    : `color-mix(in srgb, var(--nx-accent) ${Math.min(Math.abs(v) * 0.32, 0.32)}%, transparent)`;
                return (
                  <td key={cv} style={{ background: bg }} title={p === null ? undefined : `p = ${fmtP(p)}`}>
                    {fmt(v, 3)}
                    <span className="nx-signif">{stars(p)}</span>
                  </td>
                );
              })}
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}

export function TTestView({ r }: { r: TTestResult }) {
  const kind =
    r.kind === "OneSample"
      ? "One-sample t-test"
      : r.kind === "Paired"
        ? "Paired t-test"
        : r.kind === "Welch"
          ? "Two-sample t-test (Welch)"
          : "Two-sample t-test";
  return (
    <div className="nx-doc">
      <h3 className="nx-doc-title">{kind}</h3>
      <div className="nx-doc-sub">{r.variable}{r.by ? ` by ${r.by}` : ""}</div>
      <div className="nx-fitstats">
        <Stat label="N" value={String(r.n1 + (r.n2 ?? 0))} />
        <Stat label="Mean" value={fmt(r.mean1)} />
        {r.mean2 !== null ? <Stat label="Mean (group 2)" value={fmt(r.mean2)} /> : null}
        <Stat label="Difference" value={fmt(r.diff)} />
        <Stat label="SE" value={fmt(r.se)} />
        <Stat label="t" value={fmt(r.t)} />
        <Stat label="df" value={fmt(r.df, 1)} />
        <Stat label="p (two-sided)" value={fmtP(r.p)} />
        <Stat label="95% CI" value={`${fmt(r.ciLo)} … ${fmt(r.ciHi)}`} />
      </div>
      <p style={{ fontSize: 13, color: "var(--nx-text-2)", margin: 0 }}>
        H₀: mean difference = {fmt(r.mu0)}. Two-sided p = {fmtP(r.p)}.
      </p>
    </div>
  );
}

export function AnovaView({ r }: { r: AnovaResult }) {
  return (
    <div className="nx-doc">
      <h3 className="nx-doc-title">One-way ANOVA</h3>
      <div className="nx-doc-sub">
        {r.variable} by {r.by}
      </div>
      <div className="nx-fitstats">
        <Stat label="N" value={String(r.n)} />
        <Stat label="Groups" value={String(r.groups.length)} />
        <Stat label="F" value={fmt(r.f)} />
        <Stat label="df" value={`${r.dfB}, ${r.dfW}`} />
        <Stat label="p" value={fmtP(r.p)} />
        <Stat label="η²" value={fmt(r.eta2)} />
      </div>
      <SectionLabel>Group statistics</SectionLabel>
      <table className="nx-table">
        <thead>
          <tr>
            <th>Group</th>
            <th>N</th>
            <th>Mean</th>
            <th>SD</th>
          </tr>
        </thead>
        <tbody>
          {r.groups.map((g) => (
            <tr key={g.label}>
              <td>{g.label}</td>
              <td>{g.n}</td>
              <td>{fmt(g.mean)}</td>
              <td>{fmt(g.sd)}</td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}

export function CompareView({ r }: { r: CompareResult }) {
  return (
    <div className="nx-doc" style={{ maxWidth: "100%" }}>
      <h3 className="nx-doc-title">Model comparison</h3>
      <div className="nx-doc-sub">{r.ids.join(" vs ")}</div>
      <table className="nx-table">
        <thead>
          <tr>
            <th>Term</th>
            {r.ids.map((id, i) => (
              <th key={id}>
                ({id}) {r.labels[i]}
              </th>
            ))}
          </tr>
        </thead>
        <tbody>
          {r.terms.map((t, j) => (
            <React.Fragment key={t}>
              <tr>
                <td>{t}</td>
                {r.ids.map((id, m) => (
                  <td key={id}>
                    {fmt(r.coef[m]?.[j] ?? null)}
                    <span className="nx-signif">{stars(r.p[m]?.[j] ?? null)}</span>
                  </td>
                ))}
              </tr>
              <tr>
                <td />
                {r.ids.map((id, m) => (
                  <td key={id} style={{ color: "var(--nx-text-3)" }}>
                    ({fmt(r.se[m]?.[j] ?? null)})
                  </td>
                ))}
              </tr>
            </React.Fragment>
          ))}
          <tr>
            <td style={{ fontWeight: 600 }}>N</td>
            {r.n.map((n, m) => (
              <td key={m}>{fmtInt(n)}</td>
            ))}
          </tr>
        </tbody>
      </table>
      <p className="nx-table-note">*** p&lt;0.001 · ** p&lt;0.01 · * p&lt;0.05. Standard errors in parentheses.</p>
    </div>
  );
}

export function HelpView({ text }: { text: string }) {
  return (
    <div className="nx-panel" style={{ padding: "16px", whiteSpace: "pre-wrap", fontFamily: "'SF Mono', Menlo, Consolas, monospace", fontSize: 12 }}>
      {text}
    </div>
  );
}

export function OutputView({ output }: { output: ExecOutput }) {
  switch (output.kind) {
    case "Summary":
      return <SummaryView stats={output.stats} />;
    case "Corr":
      return <CorrView r={output.result} />;
    case "TTest":
      return <TTestView r={output.result} />;
    case "Anova":
      return <AnovaView r={output.result} />;
    case "Estimate":
      return <EstimateDocument id={output.id} label={output.label} result={output.result} diagnostics={output.diagnostics} />;
    case "Compare":
      return <CompareView r={output.result} />;
    case "Message":
      return <p style={{ color: "var(--nx-text-2)", fontSize: 13 }}>{output.text}</p>;
    case "Notes":
      return output.notes.length === 0 ? (
        <p style={{ color: "var(--nx-text-3)" }}>No research notes yet.</p>
      ) : (
        <div style={{ display: "flex", flexDirection: "column", gap: 8, maxWidth: 640 }}>
          {output.notes.map((n, i) => (
            <div key={i} className="nx-panel" style={{ padding: "12px 16px", fontSize: 13 }}>
              {n}
            </div>
          ))}
        </div>
      );
    case "EstimateList":
      return (
        <div className="nx-panel nx-scroll-x">
          <table className="nx-table">
            <thead>
              <tr>
                <th>ID</th>
                <th>Model</th>
                <th>N</th>
                <th>Fit</th>
                <th>Command</th>
              </tr>
            </thead>
            <tbody>
              {output.list.map((e) => (
                <tr key={e.id}>
                  <td>{e.id}</td>
                  <td>{e.label}</td>
                  <td>{e.n}</td>
                  <td>{e.fit}</td>
                  <td style={{ fontFamily: "inherit", fontSize: 11 }}>{e.command}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      );
    case "Help":
      return <HelpView text={output.text} />;
    case "Describe":
      return (
        <div className="nx-panel nx-scroll-x">
          <table className="nx-table">
            <thead>
              <tr>
                <th>Name</th>
                <th>Storage</th>
                <th>Semantic</th>
                <th>Label</th>
              </tr>
            </thead>
            <tbody>
              {output.variables.map((v) => (
                <tr key={v.name}>
                  <td>{v.name}</td>
                  <td>{v.storage}</td>
                  <td>{v.semantic}</td>
                  <td style={{ textAlign: "left" }}>{v.label || "—"}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      );
    default:
      return null;
  }
}
