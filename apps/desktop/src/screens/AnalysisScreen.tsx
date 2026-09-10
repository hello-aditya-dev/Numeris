// Analysis workspace: GUI form that compiles to the same AST as the command
// language. The canonical command is always shown (and copyable).
import { useMemo, useState } from "react";
import { NxButton, NxField, NxInput, NxSectionLabel, NxSelect } from "../components/Nx";
import { runCommand } from "../bridge";
import { useApp } from "../state/store";
import type { ApiError } from "../bridge";

type Vcov = "classical" | "robust" | "hc2" | "hc3" | "cluster" | "hac";
type Estimator = "regress" | "xtreg" | "ivregress" | "did" | "logit" | "probit" | "poisson";

export function AnalysisScreen() {
  const variables = useApp((s) => s.variables);
  const dataLoaded = useApp((s) => s.dataLoaded);
  const pushConsole = useApp((s) => s.pushConsole);
  const setScreen = useApp((s) => s.setScreen);
  const handleOutput = useApp((s) => s.handleOutput);

  const numericVars = variables.filter((v) => v.storage === "Numeric").map((v) => v.name);
  const textVars = variables.filter((v) => v.storage === "Text").map((v) => v.name);

  const [estimator, setEstimator] = useState<Estimator>("regress");
  const [outcome, setOutcome] = useState("");
  const [predictors, setPredictors] = useState<string[]>([]);
  const [vcov, setVcov] = useState<Vcov>("classical");
  const [clusterVar, setClusterVar] = useState("");
  const [hacLags, setHacLags] = useState("3");
  const [entity, setEntity] = useState("");
  const [endogenous, setEndogenous] = useState("");
  const [instrument, setInstrument] = useState("");
  const [treatVar, setTreatVar] = useState("");
  const [postVar, setPostVar] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<ApiError | null>(null);

  const command = useMemo(() => {
    const preds = predictors.join(" ");
    const seOpt =
      vcov === "cluster"
        ? `, cluster(${clusterVar || "cluster?"})`
        : vcov === "hac"
          ? `, vce(hac ${hacLags})`
          : vcov === "classical"
            ? ""
            : `, ${vcov}`;
    switch (estimator) {
      case "regress":
        return `regress ${outcome} ${preds}${seOpt}`;
      case "xtreg":
        return `xtreg ${outcome} ${preds}, fe, entity(${entity || "entity?"})${vcov !== "classical" && vcov !== "cluster" ? `, ${vcov}` : vcov === "cluster" ? `, cluster(${clusterVar || "cluster?"})` : ""}`;
      case "ivregress":
        return `ivregress 2sls ${outcome} (${endogenous} = ${instrument}) ${preds}, robust`;
      case "did":
        return `did ${outcome} ${preds}, treat(${treatVar || "treat?"}), time(${postVar || "post?"})${vcov === "cluster" ? `, cluster(${clusterVar || "cluster?"})` : vcov === "robust" ? ", robust" : ""}`;
      case "logit":
      case "probit":
      case "poisson":
        return `${estimator} ${outcome} ${preds}${vcov === "robust" ? ", robust" : ""}`;
    }
  }, [estimator, outcome, predictors, vcov, clusterVar, hacLags, entity, endogenous, instrument, treatVar, postVar]);

  const run = async () => {
    setBusy(true);
    setError(null);
    try {
      pushConsole({ kind: "command", text: command });
      const out = await runCommand(command);
      pushConsole({ kind: "output", text: summarize(out) });
      handleOutput(out);
      if ("Estimate" in out) setScreen("results");
    } catch (e) {
      const err = e as ApiError;
      setError(err);
      pushConsole({ kind: "error", text: err.what });
    } finally {
      setBusy(false);
    }
  };

  if (!dataLoaded) {
    return (
      <div className="nx-empty">
        <div className="nx-empty-title">No dataset is loaded</div>
        <div className="nx-empty-hint">Import a CSV file to configure analyses.</div>
        <NxButton variant="primary" onClick={() => setScreen("data")}>
          Open Data Workbench
        </NxButton>
      </div>
    );
  }

  return (
    <div style={{ maxWidth: 720, padding: "var(--space-2xl)", display: "flex", flexDirection: "column", gap: "var(--space-xl)", overflowY: "auto" }}>
      <div>
        <h2 style={{ margin: 0, fontSize: 17, fontWeight: 700 }}>Model specification</h2>
        <p style={{ margin: "4px 0 0", fontSize: 12, color: "var(--color-text-tertiary)" }}>
          The form and the command language compile to the same specification. There is one execution path.
        </p>
      </div>

      <NxField label="Estimator">
        <NxSelect
          value={estimator}
          onChange={(v) => setEstimator(v as Estimator)}
          options={[
            { value: "regress", label: "Linear regression (OLS)" },
            { value: "xtreg", label: "Panel: fixed effects" },
            { value: "ivregress", label: "Instrumental variables (2SLS)" },
            { value: "did", label: "Difference-in-differences" },
            { value: "logit", label: "Logistic regression" },
            { value: "probit", label: "Probit regression" },
            { value: "poisson", label: "Poisson regression" },
          ]}
        />
      </NxField>

      <NxField label="Outcome variable">
        <NxSelect
          value={outcome}
          onChange={setOutcome}
          options={[{ value: "", label: "Select outcome…" }, ...numericVars.map((v) => ({ value: v, label: v }))]}
        />
      </NxField>

      <NxField label="Predictors" help="Select variables in order; click again to remove.">
        <div style={{ display: "flex", flexWrap: "wrap", gap: 6 }}>
          {numericVars
            .filter((v) => v !== outcome)
            .map((v) => {
              const active = predictors.includes(v);
              const idx = predictors.indexOf(v);
              return (
                <button
                  key={v}
                  type="button"
                  className="nx-pill"
                  data-tone={active ? "accent" : undefined}
                  style={{ cursor: "pointer", height: 22 }}
                  onClick={() =>
                    setPredictors((ps) => (ps.includes(v) ? ps.filter((p) => p !== v) : [...ps, v]))
                  }
                  title={active ? `Position ${idx + 1} — click to remove` : "Click to add"}
                >
                  {active ? `${idx + 1}. ` : ""}
                  {v}
                </button>
              );
            })}
        </div>
      </NxField>

      {estimator === "xtreg" ? (
        <NxField label="Entity identifier">
          <NxSelect
            value={entity}
            onChange={setEntity}
            options={[
              { value: "", label: "Select entity variable…" },
              ...[...numericVars, ...textVars].map((v) => ({ value: v, label: v })),
            ]}
          />
        </NxField>
      ) : null}

      {estimator === "ivregress" ? (
        <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: "var(--space-lg)" }}>
          <NxField label="Endogenous variable">
            <NxSelect
              value={endogenous}
              onChange={setEndogenous}
              options={[{ value: "", label: "Select…" }, ...numericVars.map((v) => ({ value: v, label: v }))]}
            />
          </NxField>
          <NxField label="Instrument">
            <NxSelect
              value={instrument}
              onChange={setInstrument}
              options={[{ value: "", label: "Select…" }, ...numericVars.map((v) => ({ value: v, label: v }))]}
            />
          </NxField>
        </div>
      ) : null}

      {estimator === "did" ? (
        <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: "var(--space-lg)" }}>
          <NxField label="Treatment indicator (0/1)">
            <NxSelect
              value={treatVar}
              onChange={setTreatVar}
              options={[{ value: "", label: "Select…" }, ...numericVars.map((v) => ({ value: v, label: v }))]}
            />
          </NxField>
          <NxField label="Post-period indicator (0/1)">
            <NxSelect
              value={postVar}
              onChange={setPostVar}
              options={[{ value: "", label: "Select…" }, ...numericVars.map((v) => ({ value: v, label: v }))]}
            />
          </NxField>
        </div>
      ) : null}

      {estimator === "regress" || estimator === "xtreg" || estimator === "did" ? (
        <NxField label="Standard errors">
          <div style={{ display: "flex", gap: "var(--space-lg)", flexWrap: "wrap" }}>
            {(
              [
                ["classical", "Conventional"],
                ["robust", "Robust"],
                ["hc2", "HC2"],
                ["hc3", "HC3"],
                ["cluster", "Clustered"],
                ["hac", "HAC"],
              ] as [Vcov, string][]
            ).map(([v, label]) => (
              <label key={v} style={{ display: "flex", gap: 5, alignItems: "center", fontSize: 13 }}>
                <input type="radio" checked={vcov === v} onChange={() => setVcov(v)} />
                {label}
              </label>
            ))}
          </div>
        </NxField>
      ) : null}

      {vcov === "cluster" ? (
        <NxField label="Cluster variable" help="Every estimation row must have a cluster identifier.">
          <NxSelect
            value={clusterVar}
            onChange={setClusterVar}
            options={[
              { value: "", label: "Select cluster variable…" },
              ...[...numericVars, ...textVars].map((v) => ({ value: v, label: v })),
            ]}
          />
        </NxField>
      ) : null}

      {vcov === "hac" ? <NxField label="HAC lag order"><NxInput value={hacLags} onChange={(e) => setHacLags(e.target.value)} inputMode="numeric" /></NxField> : null}

      <NxSectionLabel>Command</NxSectionLabel>
      <div className="nx-command-preview">
        <span style={{ color: "var(--color-text-tertiary)" }}>›</span>
        <span>{command}</span>
      </div>

      {error ? (
        <div className="nx-error-report" role="alert">
          <h4>{error.what}</h4>
          <dl>
            <dt>Why this happened</dt>
            <dd>{error.why}</dd>
            <dt>What to do</dt>
            <dd>{error.action}</dd>
          </dl>
        </div>
      ) : null}

      <div style={{ display: "flex", gap: "var(--space-sm)", justifyContent: "flex-end" }}>
        <NxButton onClick={() => { setPredictors([]); setOutcome(""); setError(null); }}>Clear</NxButton>
        <NxButton
          variant="primary"
          disabled={busy || !outcome || predictors.length === 0}
          onClick={run}
          title="Ctrl/Cmd + Enter"
        >
          {busy ? "Estimating…" : "Run"}
        </NxButton>
      </div>
    </div>
  );
}

function summarize(out: Parameters<typeof JSON.stringify>[0]): string {
  const o = out as Record<string, unknown>;
  if ("Estimate" in o) {
    const est = o.Estimate as { id: string; label: string };
    return `${est.id} · ${est.label} — see Results.`;
  }
  if ("Message" in o) return o.Message as string;
  if ("DataLoaded" in o) return (o.DataLoaded as { message: string }).message;
  if ("DatasetChanged" in o) return (o.DatasetChanged as { message: string }).message;
  return "Completed.";
}
