// Result renderers — one view per structured result type. Results are an
// editorial document, not cards.
import React from "react";
import type {
  ExecOutput,
  SummaryStats,
  CorrResult,
  TTestResult,
  AnovaResult,
  EstimateResult,
  ComparisonResult,
  DiagnosticItem,
} from "../bridge";
import { formatNumber, formatP, significanceStars, NxSectionLabel } from "./Nx";

/* ---------- Summary ---------- */
export function SummaryView({ stats }: { stats: SummaryStats[] }) {
  return (
    <div className="nx-panel">
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
              <td>{formatNumber(s.mean)}</td>
              <td>{formatNumber(s.sd)}</td>
              <td>{formatNumber(s.min)}</td>
              <td>{formatNumber(s.q1)}</td>
              <td>{formatNumber(s.median)}</td>
              <td>{formatNumber(s.q3)}</td>
              <td>{formatNumber(s.max)}</td>
              <td>{formatNumber(s.skewness)}</td>
              <td>{formatNumber(s.kurtosis)}</td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}

/* ---------- Correlation matrix ---------- */
export function CorrView({ result }: { result: CorrResult }) {
  return (
    <div className="nx-panel">
      <div className="nx-panel-head">
        <span>
          {result.method} correlation — {result.n} observations
        </span>
      </div>
      <table className="nx-table">
        <thead>
          <tr>
            <th />
            {result.variables.map((v) => (
              <th key={v}>{v}</th>
            ))}
          </tr>
        </thead>
        <tbody>
          {result.variables.map((rv, i) => (
            <tr key={rv}>
              <td>{rv}</td>
              {result.variables.map((cv, j) => {
                const r = result.values[i]?.[j] ?? null;
                const p = result.p_values[i]?.[j] ?? null;
                const bg =
                  r === null
                    ? undefined
                    : `color-mix(in srgb, var(--color-accent) ${Math.min(Math.abs(r) * 0.35, 0.35)}%, transparent)`;
                return (
                  <td key={cv} style={{ background: bg }} title={p === null ? undefined : `p = ${formatP(p)}`}>
                    {formatNumber(r, 3)}
                    {significanceStars(p)}
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

/* ---------- t-test ---------- */
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
    <div className="nx-result-doc">
      <h3 className="nx-result-title">{kind}</h3>
      <div className="nx-result-sub">
        {r.variable}
        {r.by ? ` by ${r.by}` : r.n2 ? ` vs ${r.by ?? "second variable"}` : ""}
      </div>
      <div className="nx-fitstats">
        <Stat label="N" value={String(r.n1 + (r.n2 ?? 0))} />
        <Stat label="Mean" value={formatNumber(r.mean1)} />
        {r.mean2 !== null ? <Stat label="Mean (group 2)" value={formatNumber(r.mean2)} /> : null}
        <Stat label="Diff" value={formatNumber(r.diff)} />
        <Stat label="SE" value={formatNumber(r.se)} />
        <Stat label="t" value={formatNumber(r.t)} />
        <Stat label="df" value={formatNumber(r.df, 1)} />
        <Stat label="p (two-sided)" value={formatP(r.p_two_sided)} />
        <Stat label="95% CI" value={`${formatNumber(r.ci_lo)} to ${formatNumber(r.ci_hi)}`} />
      </div>
      <NxSectionLabel>Hypothesis</NxSectionLabel>
      <p style={{ fontSize: 13, color: "var(--color-text-secondary)", margin: 0 }}>
        H₀: mean difference = {formatNumber(r.mu0)}. The two-sided p-value is {formatP(r.p_two_sided)}.
      </p>
    </div>
  );
}

/* ---------- ANOVA ---------- */
export function AnovaView({ r }: { r: AnovaResult }) {
  return (
    <div className="nx-result-doc">
      <h3 className="nx-result-title">One-way ANOVA</h3>
      <div className="nx-result-sub">
        {r.variable} by {r.by}
      </div>
      <div className="nx-fitstats">
        <Stat label="N" value={String(r.n)} />
        <Stat label="Groups" value={String(r.groups.length)} />
        <Stat label="F" value={formatNumber(r.f)} />
        <Stat label="df" value={`${r.df_between}, ${r.df_within}`} />
        <Stat label="p" value={formatP(r.p)} />
        <Stat label="η²" value={formatNumber(r.eta_squared)} />
      </div>
      <NxSectionLabel>Group statistics</NxSectionLabel>
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
              <td>{formatNumber(g.mean)}</td>
              <td>{formatNumber(g.sd)}</td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}

/* ---------- Regression-family estimates ---------- */
export function EstimateView({ output }: { output: Extract<ExecOutput, { Estimate: unknown }> }) {
  const { id, label, result, diagnostics } = output.Estimate;
  const kind = Object.keys(result)[0];
  return (
    <EstimateDocument id={id} label={label} result={result as EstimateResult} diagnostics={diagnostics} kind={kind} />
  );
}

function Stat({ label, value }: { label: string; value: string }) {
  return (
    <div className="nx-fitstats-cell">
      <div className="nx-fitstats-label">{label}</div>
      <div className="nx-fitstats-value">{value}</div>
    </div>
  );
}

interface CoefRow {
  term: string;
  coef: number;
  se: number;
  stat: number;
  p: number;
  ci_lo: number;
  ci_hi: number;
}

export function EstimateDocument({
  id,
  label,
  result,
  diagnostics,
  kind,
}: {
  id: string;
  label: string;
  result: EstimateResult;
  diagnostics: DiagnosticItem[];
  kind: string;
}) {
  let rows: CoefRow[] = [];
  let fitStats: { label: string; value: string }[] = [];
  let subtitle = "";
  let modelTitle = label;

  if ("Ols" in result) {
    const r = result.Ols;
    subtitle = `Variance estimator: ${r.vcov_label}`;
    rows = r.terms.map((t, j) => ({
      term: t,
      coef: r.coef[j],
      se: r.se[j],
      stat: r.t[j],
      p: r.p[j],
      ci_lo: r.ci_lo[j],
      ci_hi: r.ci_hi[j],
    }));
    fitStats = [
      { label: "N", value: r.n.toLocaleString() },
      { label: "R²", value: formatNumber(r.r2, 4) },
      { label: "Adjusted R²", value: formatNumber(r.adj_r2, 4) },
      { label: "Root MSE", value: formatNumber(r.rmse) },
      { label: "F", value: r.f === null ? "—" : formatNumber(r.f) },
      { label: "AIC", value: formatNumber(r.aic, 1) },
      { label: "BIC", value: formatNumber(r.bic, 1) },
    ];
  } else if ("Glm" in result) {
    const r = result.Glm;
    subtitle = `${r.family} · ${r.vcov_label} · converged in ${r.iterations} iterations${r.converged ? "" : " (not converged)"}`;
    modelTitle = r.family === "Logit" ? "Logistic regression" : r.family === "Probit" ? "Probit regression" : "Poisson regression";
    rows = r.terms.map((t, j) => ({
      term: t,
      coef: r.coef[j],
      se: r.se[j],
      stat: r.z[j],
      p: r.p[j],
      ci_lo: 0,
      ci_hi: 0,
    }));
    fitStats = [
      { label: "N", value: r.n.toLocaleString() },
      { label: "Pseudo R²", value: formatNumber(r.pseudo_r2_mcfadden, 4) },
      { label: "Log-likelihood", value: formatNumber(r.loglik, 2) },
      { label: "AIC", value: formatNumber(r.aic, 1) },
      { label: "BIC", value: formatNumber(r.bic, 1) },
    ];
  } else if ("Panel" in result) {
    const r = result.Panel;
    modelTitle = r.estimator;
    subtitle = `${r.vcov_label} · entities: ${r.g}${r.dropped_singletons > 0 ? ` · ${r.dropped_singletons} singleton(s) dropped` : ""}`;
    rows = r.terms.map((t, j) => ({
      term: t,
      coef: r.coef[j],
      se: r.se[j],
      stat: r.t[j],
      p: r.p[j],
      ci_lo: 0,
      ci_hi: 0,
    }));
    fitStats = [
      { label: "N", value: r.n.toLocaleString() },
      { label: "Entities", value: String(r.g) },
      { label: "R² (within)", value: formatNumber(r.r2_within, 4) },
      { label: "sigma_u + sigma_e", value: formatNumber(r.sigma) },
    ];
  } else if ("Iv" in result) {
    const r = result.Iv;
    modelTitle = "2SLS instrumental variables";
    subtitle = r.vcov_label;
    rows = r.terms.map((t, j) => ({
      term: t,
      coef: r.coef[j],
      se: r.se[j],
      stat: r.z[j],
      p: r.p[j],
      ci_lo: 0,
      ci_hi: 0,
    }));
    fitStats = [
      { label: "N", value: r.n.toLocaleString() },
      ...r.first_stage_terms.map((t, j) => ({
        label: `First-stage F (${t})`,
        value: formatNumber(r.first_stage_f[j]),
      })),
      ...r.first_stage_terms.map((t, j) => ({
        label: `Partial R² (${t})`,
        value: formatNumber(r.first_stage_partial_r2[j]),
      })),
    ];
  } else if ("Did" in result) {
    const r = result.Did;
    modelTitle = "Difference-in-differences";
    subtitle = "Interaction term treat#post is the ATT";
    rows = r.terms.map((t, j) => ({
      term: t,
      coef: r.coef[j],
      se: r.se[j],
      stat: 0,
      p: r.p_values[j],
      ci_lo: 0,
      ci_hi: 0,
    }));
    fitStats = [
      { label: "N", value: r.n.toLocaleString() },
      { label: "ATT", value: formatNumber(r.att) },
      { label: "SE", value: formatNumber(r.att_se) },
      { label: "p", value: formatP(r.p) },
      { label: "95% CI", value: `${formatNumber(r.ci_lo)} to ${formatNumber(r.ci_hi)}` },
    ];
  }

  return (
    <div className="nx-result-doc">
      <h3 className="nx-result-title">{modelTitle}</h3>
      <div className="nx-result-sub">
        {id} · {subtitle || kind}
      </div>
      <div className="nx-fitstats">
        {fitStats.map((f) => (
          <Stat key={f.label} label={f.label} value={f.value} />
        ))}
      </div>
      <NxSectionLabel>Coefficients</NxSectionLabel>
      <table className="nx-table">
        <thead>
          <tr>
            <th>Term</th>
            <th>Coef.</th>
            <th>Std. err.</th>
            <th>t</th>
            <th>p&gt;|t|</th>
            <th>[95% conf.]</th>
          </tr>
        </thead>
        <tbody>
          {rows.map((r) => (
            <tr key={r.term}>
              <td>{r.term}</td>
              <td>
                {formatNumber(r.coef)}
                <span className="nx-signif">{significanceStars(r.p)}</span>
              </td>
              <td>{formatNumber(r.se)}</td>
              <td>{formatNumber(r.stat)}</td>
              <td>{formatP(r.p)}</td>
              <td>
                {r.ci_lo || r.ci_hi ? `${formatNumber(r.ci_lo)} … ${formatNumber(r.ci_hi)}` : "—"}
              </td>
            </tr>
          ))}
        </tbody>
      </table>
      {diagnostics.length > 0 ? (
        <>
          <NxSectionLabel>Diagnostics</NxSectionLabel>
          <DiagnosticsList diagnostics={diagnostics} />
        </>
      ) : null}
    </div>
  );
}

/* ---------- Diagnostics ---------- */
export function DiagnosticsList({ diagnostics }: { diagnostics: DiagnosticItem[] }) {
  return (
    <div>
      {diagnostics.map((d) => (
        <div key={d.name} className="nx-diag" data-status={d.status}>
          <span className="nx-diag-status">{d.status === "ok" ? "✓" : d.status === "warning" ? "!" : d.status === "attention" ? "✕" : "i"}</span>
          <span className="nx-diag-name">{d.name}</span>
          <span className="nx-diag-detail">{d.detail}</span>
        </div>
      ))}
    </div>
  );
}

/* ---------- Model comparison ---------- */
export function CompareView({ r }: { r: ComparisonResult }) {
  return (
    <div className="nx-result-doc" style={{ maxWidth: "100%" }}>
      <h3 className="nx-result-title">Model comparison</h3>
      <div className="nx-result-sub">{r.ids.join(" vs ")}</div>
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
                {r.ids.map((id, m) => {
                  const co = r.coef[m]?.[j] ?? null;
                  const p = r.p[m]?.[j] ?? null;
                  return (
                    <td key={id}>
                      {formatNumber(co)}
                      <span className="nx-signif">{significanceStars(p)}</span>
                    </td>
                  );
                })}
              </tr>
              <tr>
                <td />
                {r.ids.map((id, m) => (
                  <td key={id} style={{ color: "var(--color-text-tertiary)" }}>
                    ({formatNumber(r.se[m]?.[j] ?? null)})
                  </td>
                ))}
              </tr>
            </React.Fragment>
          ))}
          <tr>
            <td style={{ fontWeight: 600 }}>N</td>
            {r.n.map((n, m) => (
              <td key={m}>{n.toLocaleString()}</td>
            ))}
          </tr>
          {r.fit.map((f, m) => (
            <tr key={m}>
              <td />
              <td colSpan={r.ids.length} style={{ textAlign: "left", color: "var(--color-text-tertiary)" }}>
                {f}
              </td>
            </tr>
          ))}
        </tbody>
      </table>
      <p style={{ fontSize: 11, color: "var(--color-text-tertiary)" }}>
        *** p&lt;0.001 · ** p&lt;0.01 · * p&lt;0.05. Standard errors in parentheses.
      </p>
    </div>
  );
}

/* ---------- Generic output dispatcher ---------- */
export function OutputView({ output }: { output: ExecOutput }) {
  if ("Summary" in output) return <SummaryView stats={output.Summary} />;
  if ("Corr" in output) return <CorrView result={output.Corr} />;
  if ("TTest" in output) return <TTestView r={output.TTest} />;
  if ("Anova" in output) return <AnovaView r={output.Anova} />;
  if ("Estimate" in output) return <EstimateView output={output} />;
  if ("Compare" in output) return <CompareView r={output.Compare} />;
  if ("Message" in output) return <MessageView text={output.Message} />;
  if ("Help" in output) return <HelpView text={output.Help} />;
  if ("Notes" in output)
    return (
      <div className="nx-panel" style={{ padding: "var(--space-lg)" }}>
        {output.Notes.length === 0 ? (
          <p style={{ color: "var(--color-text-tertiary)" }}>No research notes yet.</p>
        ) : (
          output.Notes.map((n, i) => (
            <p key={i} style={{ fontSize: 13 }}>
              {n}
            </p>
          ))
        )}
      </div>
    );
  if ("EstimateList" in output)
    return (
      <div className="nx-panel">
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
            {output.EstimateList.map((e) => (
              <tr key={e.id}>
                <td>{e.id}</td>
                <td>{e.label}</td>
                <td>{e.n}</td>
                <td>{e.fit}</td>
                <td style={{ fontFamily: "var(--font-mono)", fontSize: 11 }}>{e.command}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    );
  if ("DatasetChanged" in output)
    return <MessageView text={output.DatasetChanged.message} />;
  if ("DataLoaded" in output) return <MessageView text={output.DataLoaded.message} />;
  if ("Frequency" in output)
    return (
      <div className="nx-panel">
        <div className="nx-panel-head">{output.Frequency.variable} — frequencies</div>
        <table className="nx-table">
          <thead>
            <tr>
              <th>Value</th>
              <th>Count</th>
              <th>Percent</th>
              <th>Cumulative</th>
            </tr>
          </thead>
          <tbody>
            {output.Frequency.rows.map((r) => (
              <tr key={r.value}>
                <td>{r.value}</td>
                <td>{r.count}</td>
                <td>{r.percent.toFixed(1)}</td>
                <td>{r.cumulative_percent.toFixed(1)}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    );
  if ("Describe" in output)
    return (
      <div className="nx-panel">
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
            {output.Describe.map((v) => (
              <tr key={v.name}>
                <td>{v.name}</td>
                <td>{v.storage}</td>
                <td>{v.semantic}</td>
                <td>{v.label || "—"}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    );
  return null;
}

export function MessageView({ text }: { text: string }) {
  return (
    <div style={{ padding: "var(--space-lg)", color: "var(--color-text-secondary)", fontSize: 13 }}>{text}</div>
  );
}

export function HelpView({ text }: { text: string }) {
  return (
    <div className="nx-panel" style={{ padding: "var(--space-lg)", whiteSpace: "pre-wrap", fontFamily: "var(--font-mono)", fontSize: 12 }}>
      {text}
    </div>
  );
}
