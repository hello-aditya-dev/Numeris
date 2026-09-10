// Numeris web-preview engine — executor.
// Mirrors crates/numeris-command/src/exec.rs over the TS engine.

import {
  AnovaResult, CorrResult, EngineError, Family, GlmResult, PanelResult, Regression,
  TTestResult, corrMatrix, fixedEffects, fitGlm, fitOls, mean, oneWayAnova,
  quantileSorted, rankAverage, tTestOneSample, tTestPaired, tTestTwoSample,
  variance, vif, type VcovSpec,
} from "./stats";
import { parse, render, type Command, type Expr } from "./parser";
import { type Dataset, buildDemoDataset, parseCsv } from "./dataset";
import { qrDecompose, qrSolve, xtxInv } from "./linalg";
import { tQuantile, tTwoSidedP } from "./dist";

export type ExecOutput =
  | { kind: "Summary"; stats: SummaryRow[] }
  | { kind: "Describe"; variables: Dataset["variables"] }
  | { kind: "Corr"; result: CorrResult }
  | { kind: "TTest"; result: TTestResult }
  | { kind: "Anova"; result: AnovaResult }
  | { kind: "Estimate"; id: string; label: string; result: EstimateResult; diagnostics: DiagnosticRow[] }
  | { kind: "EstimateList"; list: { id: string; label: string; command: string; n: number; fit: string }[] }
  | { kind: "Compare"; result: CompareResult }
  | { kind: "Message"; text: string }
  | { kind: "Notes"; notes: string[] }
  | { kind: "Help"; text: string };

export type EstimateResult =
  | { Ols: Regression }
  | { Glm: GlmResult }
  | { Panel: PanelResult }
  | { Iv: IvResult }
  | { Did: DidResult };

export interface IvResult {
  terms: string[];
  coef: number[];
  se: number[];
  z: number[];
  p: number[];
  ciLo: number[];
  ciHi: number[];
  n: number;
  rmse: number;
  firstStageF: number[];
  firstStagePartialR2: number[];
  firstStageTerms: string[];
  vcovLabel: string;
}

export interface DidResult {
  att: number;
  attSe: number;
  t: number;
  p: number;
  ciLo: number;
  ciHi: number;
  means: [number, number][];
  n: number;
  terms: string[];
  coef: number[];
  se: number[];
  pValues: number[];
  vcovLabel: string;
}

export interface SummaryRow {
  name: string;
  n: number;
  missing: number;
  mean: number | null;
  sd: number | null;
  min: number | null;
  max: number | null;
  median: number | null;
  q1: number | null;
  q3: number | null;
  skewness: number | null;
  kurtosis: number | null;
  unique: number;
}

export interface DiagnosticRow {
  name: string;
  status: "ok" | "warning" | "attention" | "info";
  detail: string;
}

export interface CompareResult {
  ids: string[];
  labels: string[];
  terms: string[];
  coef: (number | null)[][];
  se: (number | null)[][];
  p: (number | null)[][];
  n: number[];
  fit: string[];
}

export interface StoredEstimate {
  id: string;
  label: string;
  command: string;
  result: EstimateResult;
}

export function toVcovSpec(v: import("./parser").VcovOption): VcovSpec {
  if (v === "classical") return { type: "classical" };
  if (v === "robust") return { type: "hc1" };
  if (v === "hc2") return { type: "hc2" };
  if (v === "hc3") return { type: "hc3" };
  if (typeof v === "object" && "cluster" in v) return { type: "cluster" };
  if (typeof v === "object" && "hac" in v) return { type: "hac", lags: v.hac };
  return { type: "classical" };
}

export class Executor {
  dataset: Dataset;
  estimates: StoredEstimate[] = [];
  notes: string[] = [];
  private nextModel = 1;
  private history: string[] = [];

  constructor(dataset?: Dataset) {
    this.dataset = dataset ?? buildDemoDataset();
  }

  loadDemo(): void {
    this.dataset = buildDemoDataset();
  }

  loadCsv(text: string): { dataset: Dataset } {
    this.dataset = parseCsv(text);
    return { dataset: this.dataset };
  }

  get history_(): string[] {
    return this.history;
  }

  execute(source: string): ExecOutput {
    this.history.push(source);
    const cmd = parse(source);
    return this.executeCommand(cmd);
  }

  executeCommand(cmd: Command): ExecOutput {
    const df = this.dataset;
    const numericCols = Object.keys(df.numeric);
    switch (cmd.kind) {
      case "summarize": {
        const vars = cmd.variables.length > 0 ? cmd.variables : numericCols;
        const stats: SummaryRow[] = vars.map((name) => {
          const col = df.numeric[name];
          if (!col) throw new EngineError(`Variable '${name}' was not found in the dataset.`, `The dataset contains these variables: ${numericCols.join(", ")}.`, "Check the spelling of the variable name or run 'describe' to list variables.");
          const xs = col.filter((v): v is number => v !== null);
          const sorted = [...xs].sort((a, b) => a - b);
          return {
            name,
            n: xs.length,
            missing: col.length - xs.length,
            mean: xs.length ? mean(xs) : null,
            sd: xs.length > 1 ? Math.sqrt(variance(xs)) : null,
            min: sorted[0] ?? null,
            max: sorted[sorted.length - 1] ?? null,
            median: sorted.length ? quantileSorted(sorted, 0.5) : null,
            q1: sorted.length ? quantileSorted(sorted, 0.25) : null,
            q3: sorted.length ? quantileSorted(sorted, 0.75) : null,
            skewness: skewness(xs),
            kurtosis: kurtosis(xs),
            unique: new Set(xs).size,
          };
        });
        return { kind: "Summary", stats };
      }
      case "describe":
        return { kind: "Describe", variables: cmd.variables.length ? df.variables.filter((v) => cmd.variables.includes(v.name)) : df.variables };
      case "correlate": {
        const vars = cmd.variables.length > 0 ? cmd.variables : numericCols;
        if (vars.length < 2) throw new EngineError("correlate requires at least two variables.", "A correlation matrix needs two or more numeric variables.", "List at least two variables, for example: correlate wage education.");
        const idx = completeCases(this.dataset, vars);
        const cols = vars.map((v) => (df.numeric[v] as number[]).filter((_, i) => idx.has(i)));
        return { kind: "Corr", result: corrMatrix(cols, vars, cmd.spearman) };
      }
      case "ttest": {
        const form = cmd.form;
        if ("oneSample" in form) {
          const xs = numericCol(this.dataset, cmd.variable);
          return { kind: "TTest", result: tTestOneSample(xs, form.oneSample, cmd.variable) };
        }
        if ("byGroup" in form) {
          const gcol = [...(df.numeric[form.byGroup] ?? []), ...(df.text[form.byGroup] ?? [])];
          if (!df.numeric[form.byGroup] && !df.text[form.byGroup]) {
            throw new EngineError(`Variable '${form.byGroup}' was not found in the dataset.`, `The dataset contains these variables: ${numericCols.join(", ")}.`, "Check the grouping variable name.");
          }
          const y = df.numeric[cmd.variable] ?? [];
          const labels = gcol.map((v) => String(v ?? ""));
          const keys = [...new Set(labels.filter((l) => l !== "" && l !== "null"))].sort();
          if (keys.length !== 2) {
            throw new EngineError(`The grouping variable '${form.byGroup}' must define exactly two groups.`, `${keys.length} distinct values were found.`, "Choose a 0/1 indicator or a variable with exactly two values.");
          }
          const a: number[] = [];
          const b: number[] = [];
          for (let i = 0; i < labels.length; i++) {
            if (labels[i] === keys[1] && y[i] != null) a.push(y[i]!);
            else if (labels[i] === keys[0] && y[i] != null) b.push(y[i]!);
          }
          return { kind: "TTest", result: tTestTwoSample(a, b, form.welch, form.byGroup, cmd.variable) };
        }
        const idx = completeCases(this.dataset, [cmd.variable, form.twoVars]);
        const a = (df.numeric[cmd.variable] as number[]).filter((_, i) => idx.has(i));
        const b = (df.numeric[form.twoVars] as number[]).filter((_, i) => idx.has(i));
        return {
          kind: "TTest",
          result: form.paired ? tTestPaired(a, b, cmd.variable, form.twoVars) : tTestTwoSample(a, b, false, form.twoVars, cmd.variable),
        };
      }
      case "anova": {
        const idx = completeCases(this.dataset, [cmd.outcome, cmd.group]);
        const values = (df.numeric[cmd.outcome] as number[]).filter((_, i) => idx.has(i));
        const gcol = [...(df.numeric[cmd.group] ?? []), ...(df.text[cmd.group] ?? [])];
        const labels = gcol.map((v, i) => (idx.has(i) ? String(v ?? "") : "")).filter((l) => l !== "");
        return { kind: "Anova", result: oneWayAnova(values, labels, cmd.group, cmd.outcome) };
      }
      case "regress":
        return this.runRegression(cmd.outcome, cmd.predictors, cmd.vcov, render(cmd));
      case "xtreg": {
        const vars = [cmd.outcome, ...cmd.predictors, cmd.entity, ...(typeof cmd.vcov === "object" && "cluster" in cmd.vcov ? [cmd.vcov.cluster] : [])];
        const idx = completeCases(this.dataset, vars);
        const y = gather(df, cmd.outcome, idx);
        const xs = cmd.predictors.map((p) => gather(df, p, idx));
        const entity = gatherAsLabels(df, cmd.entity, idx);
        const spec = toVcovSpec(cmd.vcov);
        let clusterIds: (number | string)[] | undefined;
        if (typeof cmd.vcov === "object" && "cluster" in cmd.vcov) {
          clusterIds = gatherAsLabels(df, cmd.vcov.cluster, idx);
        }
        const x = transpose(xs);
        if (cmd.model === "fe") {
          const result = fixedEffects(y, x, entity, cmd.predictors, spec, clusterIds);
          return this.storeEstimate(render(cmd), { Panel: result }, `Panel: ${result.estimator}`);
        }
        // Pooled / between via OLS on the panel.
        const result = fitOls({ y, x, names: cmd.predictors, vcov: spec.type === "cluster" ? { type: "hc1" } : spec, estimatorLabel: cmd.model === "be" ? "Between" : "Pooled OLS" });
        return this.storeEstimate(render(cmd), { Ols: result }, cmd.model === "be" ? "Between (entity means)" : "Pooled OLS");
      }
      case "ivregress": {
        const vars = [cmd.outcome, ...cmd.endogenous, ...cmd.instruments, ...cmd.exogenous];
        const idx = completeCases(this.dataset, vars);
        const y = gather(df, cmd.outcome, idx);
        const e = transpose(cmd.endogenous.map((v) => gather(df, v, idx)));
        const xc = transpose(cmd.exogenous.map((v) => gather(df, v, idx)));
        // Instruments stay as columns (fit2sls expects column arrays).
        const zCols = cmd.instruments.map((v) => gather(df, v, idx));
        const names = [...cmd.endogenous, ...cmd.exogenous];
        const result = fit2sls(y, e, xc, zCols, names, toVcovSpec(cmd.vcov));
        return this.storeEstimate(render(cmd), { Iv: result }, "2SLS");
      }
      case "did": {
        const vars = [cmd.outcome, cmd.treat, cmd.time, ...cmd.controls, ...(typeof cmd.vcov === "object" && "cluster" in cmd.vcov ? [cmd.vcov.cluster] : [])];
        const idx = completeCases(this.dataset, vars);
        const y = gather(df, cmd.outcome, idx);
        const treat = gather(df, cmd.treat, idx);
        const post = gather(df, cmd.time, idx);
        for (let i = 0; i < treat.length; i++) {
          if (treat[i] !== 0 && treat[i] !== 1) {
            throw new EngineError(`The treat variable must be 0/1; value ${treat[i]} was found at observation ${i + 1}.`, "DID compares a treated group (1) against a control group (0).", "Recode the treatment indicator to 0/1 before running DID.");
          }
          if (post[i] !== 0 && post[i] !== 1) {
            throw new EngineError(`The time variable must be 0/1; value ${post[i]} was found at observation ${i + 1}.`, "DID compares periods before (0) and after (1) the treatment.", "Recode the post indicator to 0/1 before running DID.");
          }
        }
        const controls = transpose(cmd.controls.map((v) => gather(df, v, idx)));
        const spec = toVcovSpec(cmd.vcov);
        const clusterIds = typeof cmd.vcov === "object" && "cluster" in cmd.vcov ? gatherAsLabels(df, cmd.vcov.cluster, idx) : undefined;
        const result = fitDid(y, treat, post, controls, cmd.controls, spec, clusterIds);
        return this.storeEstimate(render(cmd), { Did: result }, "DID");
      }
      case "logit":
      case "probit":
      case "poisson": {
        const vars = [cmd.outcome, ...cmd.predictors];
        const idx = completeCases(this.dataset, vars);
        const y = gather(df, cmd.outcome, idx);
        const x = transpose(cmd.predictors.map((v) => gather(df, v, idx)));
        const family: Family = cmd.kind === "logit" ? "Logit" : cmd.kind === "probit" ? "Probit" : "Poisson";
        const result = fitGlm(y, x, cmd.predictors, family, cmd.robust);
        const diagnostics: DiagnosticRow[] = result.warnings.map((w) => ({ name: "Estimation warning", status: "warning", detail: w }));
        return this.storeEstimate(render(cmd), { Glm: result }, family === "Logit" ? "Logit" : family === "Probit" ? "Probit" : "Poisson", diagnostics);
      }
      case "generate": {
        if (df.numeric[cmd.name] || df.text[cmd.name]) {
          throw new EngineError(`A variable named '${cmd.name}' already exists.`, "generate creates a new variable; it never overwrites an existing one.", "Choose a new name, or use 'replace' to modify the existing variable.");
        }
        const values = evalExpr(this.dataset, cmd.expr);
        df.numeric[cmd.name] = values;
        df.variables.push({ name: cmd.name, label: "", storage: "Numeric", semantic: "Continuous" });
        return { kind: "Message", text: `Variable '${cmd.name}' created (${df.rows} values).` };
      }
      case "replace": {
        if (!df.numeric[cmd.name]) throw new EngineError(`Variable '${cmd.name}' was not found in the dataset.`, "replace modifies an existing numeric variable.", "Check the variable name with 'describe'.");
        const values = evalExpr(this.dataset, cmd.expr);
        const cond = cmd.cond;
        const condCol = cond ? df.numeric[cond.variable] : null;
        let changed = 0;
        for (let i = 0; i < df.rows; i++) {
          const keep = condCol ? condCol[i] !== null && condHolds(cond!, condCol[i]!) : true;
          if (keep && values[i] !== null) {
            df.numeric[cmd.name][i] = values[i];
            changed++;
          }
        }
        return { kind: "Message", text: `Variable '${cmd.name}' updated (${changed} values changed).` };
      }
      case "drop": {
        if (cmd.cond) {
          const col = df.numeric[cmd.cond.variable];
          if (!col) throw new EngineError(`Variable '${cmd.cond.variable}' was not found in the dataset.`, "Conditions apply to numeric variables.", "Check the variable name.");
          for (const key of Object.keys(df.numeric)) {
            df.numeric[key] = df.numeric[key]!.filter((_, i) => !(col[i] !== null && condHolds(cmd.cond!, col[i]!)));
          }
          for (const key of Object.keys(df.text)) {
            df.text[key] = df.text[key]!.filter((_, i) => !(col[i] !== null && condHolds(cmd.cond!, col[i]!)));
          }
          df.rows = df.numeric[Object.keys(df.numeric)[0]]?.length ?? 0;
          return { kind: "Message", text: "Rows not satisfying the condition were dropped." };
        }
        for (const v of cmd.variables ?? []) {
          if (!df.numeric[v] && !df.text[v]) throw new EngineError(`Variable '${v}' was not found in the dataset.`, "drop removes existing variables.", "Check the variable names.");
          delete df.numeric[v];
          delete df.text[v];
          df.variables = df.variables.filter((x) => x.name !== v);
        }
        return { kind: "Message", text: `Dropped ${(cmd.variables ?? []).length} variable(s).` };
      }
      case "keep": {
        if (cmd.cond) {
          const col = df.numeric[cmd.cond.variable];
          if (!col) throw new EngineError(`Variable '${cmd.cond.variable}' was not found in the dataset.`, "Conditions apply to numeric variables.", "Check the variable name.");
          for (const key of Object.keys(df.numeric)) {
            df.numeric[key] = df.numeric[key]!.filter((_, i) => col[i] !== null && condHolds(cmd.cond!, col[i]!));
          }
          for (const key of Object.keys(df.text)) {
            df.text[key] = df.text[key]!.filter((_, i) => col[i] !== null && condHolds(cmd.cond!, col[i]!));
          }
          df.rows = df.numeric[Object.keys(df.numeric)[0]]?.length ?? 0;
          return { kind: "Message", text: "Rows not satisfying the condition were removed." };
        }
        const keep = new Set(cmd.variables ?? []);
        for (const v of df.variables.map((v) => v.name)) {
          if (!keep.has(v)) {
            delete df.numeric[v];
            delete df.text[v];
          }
        }
        df.variables = df.variables.filter((v) => keep.has(v.name));
        return { kind: "Message", text: `Kept ${(cmd.variables ?? []).length} variable(s).` };
      }
      case "rename": {
        const v = df.variables.find((v) => v.name === cmd.from);
        if (!v) throw new EngineError(`Variable '${cmd.from}' was not found in the dataset.`, "rename renames existing variables.", "Check the variable name.");
        v.name = cmd.to;
        if (df.numeric[cmd.from]) {
          df.numeric[cmd.to] = df.numeric[cmd.from];
          delete df.numeric[cmd.from];
        }
        if (df.text[cmd.from]) {
          df.text[cmd.to] = df.text[cmd.from];
          delete df.text[cmd.from];
        }
        return { kind: "Message", text: `Renamed '${cmd.from}' to '${cmd.to}'.` };
      }
      case "label": {
        const v = df.variables.find((v) => v.name === cmd.variable);
        if (!v) throw new EngineError(`Variable '${cmd.variable}' was not found in the dataset.`, "Labels attach to existing variables.", "Check the variable name with 'describe'.");
        v.label = cmd.text;
        return { kind: "Message", text: `Label set for '${cmd.variable}'.` };
      }
      case "sort": {
        const col = df.numeric[cmd.variable];
        if (!col) throw new EngineError(`Variable '${cmd.variable}' was not found in the dataset.`, "sort orders rows by an existing numeric variable.", "Check the variable name.");
        const order = Array.from({ length: df.rows }, (_, i) => i).sort((a, b) => {
          const av = col[a];
          const bv = col[b];
          if (av === null && bv === null) return 0;
          if (av === null) return 1;
          if (bv === null) return -1;
          return cmd.descending ? bv - av : av - bv;
        });
        for (const key of Object.keys(df.numeric)) {
          const old = df.numeric[key]!;
          df.numeric[key] = order.map((i) => old[i]);
        }
        for (const key of Object.keys(df.text)) {
          const old = df.text[key]!;
          df.text[key] = order.map((i) => old[i]);
        }
        return { kind: "Message", text: `Sorted by '${cmd.variable}'${cmd.descending ? " (descending)" : ""}.` };
      }
      case "estimateList":
        return {
          kind: "EstimateList",
          list: this.estimates.map((e) => ({ id: e.id, label: e.label, command: e.command, n: estimateN(e.result), fit: fitLine(e.result) })),
        };
      case "estimateCompare":
        return { kind: "Compare", result: this.compare(cmd.ids) };
      case "note":
        this.notes.push(cmd.text);
        return { kind: "Message", text: "Note recorded." };
      case "notes":
        return { kind: "Notes", notes: this.notes };
      case "help":
        return { kind: "Help", text: helpText(cmd.command) };
    }
  }

  private runRegression(outcome: string, predictors: string[], vcov: import("./parser").VcovOption, command: string): ExecOutput {
    if (predictors.length === 0) {
      throw new EngineError("The regression has no predictors.", "regress requires at least one predictor besides the constant.", "Add one or more predictor variables to the command.");
    }
    const df = this.dataset;
    const clusterVar = typeof vcov === "object" && "cluster" in vcov ? vcov.cluster : null;
    const vars = [outcome, ...predictors];
    // The cluster variable is deliberately NOT part of the complete-case
    // filter: missing cluster ids must raise an actionable error.
    const idx = completeCases(this.dataset, vars);
    const y = gather(df, outcome, idx);
    const x = transpose(predictors.map((p) => gather(df, p, idx)));
    const spec = toVcovSpec(vcov);
    let clusters: (number | string)[] | undefined;
    if (clusterVar) {
      const col = df.numeric[clusterVar] ?? df.text[clusterVar];
      if (!col) throw new EngineError(`Variable '${clusterVar}' was not found in the dataset.`, "The cluster option references a variable that does not exist.", "Check the cluster variable name.");
      clusters = idxToArray(idx).map((i) => col[i] as number | string | null).map((v, j) => {
        if (v === null || v === undefined || v === "") {
          throw new EngineError(`The cluster variable '${clusterVar}' contains missing values.`, "Clustered standard errors require every estimation row to have a cluster identifier; missing identifiers cannot be assigned to a cluster.", "Resolve the missing cluster identifiers (fill, drop, or choose a different variable) before estimating clustered standard errors.");
        }
        return v as number | string;
      });
    }
    const result = fitOls({ y, x, names: predictors, vcov: spec, clusters });
    const diagnostics = regDiagnostics(result, x);
    return this.storeEstimate(command, { Ols: result }, `OLS (${result.vcovLabel})`, diagnostics);
  }

  private storeEstimate(command: string, result: EstimateResult, label: string, diagnostics: DiagnosticRow[] = []): ExecOutput {
    const id = `M${this.nextModel++}`;
    this.estimates.push({ id, label, command, result });
    return { kind: "Estimate", id, label, result, diagnostics };
  }

  compare(ids: string[]): CompareResult {
    const selected = ids.length ? ids.map((id) => this.estimates.find((e) => e.id.toLowerCase() === id.toLowerCase())!).filter(Boolean) : this.estimates;
    if (selected.length === 0) {
      throw new EngineError("No estimates are available to compare.", "estimate compare requires at least one stored model.", "Run a model first, then compare.");
    }
    const terms: string[] = [];
    for (const e of selected) {
      for (const t of estimateTerms(e.result)) {
        if (!terms.includes(t)) terms.push(t);
      }
    }
    return {
      ids: selected.map((e) => e.id),
      labels: selected.map((e) => e.label),
      terms,
      coef: selected.map((e) => terms.map((t) => {
        const i = estimateTerms(e.result).indexOf(t);
        return i >= 0 ? estimateCoef(e.result)[i] : null;
      })),
      se: selected.map((e) => terms.map((t) => {
        const i = estimateTerms(e.result).indexOf(t);
        return i >= 0 ? estimateSe(e.result)[i] : null;
      })),
      p: selected.map((e) => terms.map((t) => {
        const i = estimateTerms(e.result).indexOf(t);
        return i >= 0 ? estimateP(e.result)[i] : null;
      })),
      n: selected.map((e) => estimateN(e.result)),
      fit: selected.map((e) => fitLine(e.result)),
    };
  }

  /** Export the current comparison set as a publication table string. */
  exportTable(format: "md" | "csv" | "tex" | "html", ids: string[]): string {
    const c = this.compare(ids);
    const stars = (p: number | null) => (p === null ? "" : p < 0.001 ? "***" : p < 0.01 ? "**" : p < 0.05 ? "*" : "");
    const f = (v: number | null) => (v === null ? "" : Math.abs(v) >= 1e7 ? v.toExponential(2) : v.toFixed(3));
    if (format === "md") {
      let s = `| Term |${c.ids.map((i) => ` ${i} |`).join("")}\n|---|${c.ids.map(() => "---:|").join("")}\n`;
      c.terms.forEach((t, j) => {
        s += `| ${t} |${c.ids.map((_, m) => ` ${f(c.coef[m][j])}${stars(c.p[m][j])} |`).join("")}\n`;
        s += `|  |${c.ids.map((_, m) => ` (${f(c.se[m][j])}) |`).join("")}\n`;
      });
      s += `| N |${c.ids.map((_, m) => ` ${c.n[m]} |`).join("")}\n`;
      s += `\n*Significance: *** p<0.001, ** p<0.01, * p<0.05.*\n`;
      return s;
    }
    if (format === "csv") {
      let s = "term" + c.ids.map((i) => `,${i}_coef,${i}_se,${i}_p`).join("") + "\n";
      c.terms.forEach((t, j) => {
        s += t + c.ids.map((_, m) => `,${c.coef[m][j] ?? ""},${c.se[m][j] ?? ""},${c.p[m][j] ?? ""}`).join("") + "\n";
      });
      s += "N" + c.ids.map(() => ",,").join("") + "\n";
      return s;
    }
    if (format === "tex") {
      let s = `\\begin{tabular}{l${"c".repeat(c.ids.length)}}\n\\toprule\n & ${c.ids.map((i) => `(${i})`).join(" & ")} \\\\\n\\midrule\n`;
      c.terms.forEach((t, j) => {
        s += `${t.replace(/_/g, "\\_")}${c.ids.map((_, m) => ` & ${f(c.coef[m][j])}${stars(c.p[m][j])}`).join("")} \\\\\n`;
        s += `${c.ids.map(() => " & ").join("")}${c.ids.map((_, m) => `(${f(c.se[m][j])})`).join(" & ")} \\\\\n`;
      });
      s += `\\midrule\nN${c.ids.map((_, m) => ` & ${c.n[m]}`).join("")} \\\\\n\\bottomrule\n\\end{tabular}\n`;
      return s;
    }
    let s = `<table class="numeris-table">\n<thead><tr><th>Term</th>${c.ids.map((i) => `<th>${i}</th>`).join("")}</tr></thead>\n<tbody>\n`;
    c.terms.forEach((t, j) => {
      s += `<tr><th>${t}</th>${c.ids.map((_, m) => `<td>${f(c.coef[m][j])}${stars(c.p[m][j])}</td>`).join("")}</tr>\n<tr><td></td>${c.ids.map((_, m) => `<td>(${f(c.se[m][j])})</td>`).join("")}</tr>\n`;
    });
    s += `</tbody>\n</table>\n`;
    return s;
  }
}

/* ============ 2SLS (TS port) ============ */

function fit2sls(y: number[], xendog: number[][], xexog: number[][], instrumentCols: number[][], names: string[], spec: VcovSpec): IvResult {
  // Matrices are row-major: xendog[i] = observation i's endogenous values.
  const n = y.length;
  const e = xendog[0]?.length ?? 0;
  const c = xexog[0]?.length ?? 0;
  const colOf = (m: number[][], j: number): number[] => m.map((row) => row[j]);
  if (e === 0) throw new EngineError("The model specifies no endogenous regressors.", "ivregress requires at least one endogenous variable with instruments.", "Rewrite the command with an instrumented variable in parentheses.");
  const zCols: number[][] = [...instrumentCols, ...Array.from({ length: c }, (_, j) => colOf(xexog, j))];
  if (zCols.length < e) {
    throw new EngineError("The model fails the order condition: there are fewer instruments than endogenous variables.", `There are ${instrumentCols.length} excluded instruments but ${e} endogenous regressors.`, "Add at least as many instruments as endogenous variables.");
  }
  // Z = [instruments, exogenous controls, constant].
  const zfull: number[][] = [];
  for (let i = 0; i < n; i++) {
    zfull.push([...zCols.map((col) => col[i]), 1]);
  }
  const zk = zfull[0].length;
  const zqr = qrDecompose(zfull);
  // Xhat = projection of X on Z, column by column.
  const xhat: number[][] = Array.from({ length: n }, () => new Array(e + c + 1).fill(0));
  for (let j = 0; j < e + c + 1; j++) {
    const colj = j < e ? colOf(xendog, j) : j < e + c ? colOf(xexog, j - e) : new Array(n).fill(1);
    const gamma = qrSolve(zqr, colj);
    for (let i = 0; i < n; i++) xhat[i][j] = zfull[i].reduce((s, zv, t) => s + zv * gamma[t], 0);
  }
  const qr = qrDecompose(xhat);
  const beta = qrSolve(qr, y);
  // Structural design X = [endogenous, exogenous, constant] (unfitted).
  const xfull: number[][] = [];
  for (let i = 0; i < n; i++) {
    xfull.push([...xendog[i], ...xexog[i], 1]);
  }
  const residuals = y.map((v, i) => v - xfull[i].reduce((s, xv, j) => s + xv * beta[j], 0));
  const inv = xtxInv(qr);
  const se: number[] = [];
  const z: number[] = [];
  const p: number[] = [];
  const lo: number[] = [];
  const hi: number[] = [];
  const dfR = Math.max(n - zk, 1);
  const tc = tQuantile(0.975, dfR);
  if (spec.type !== "classical") {
    // Robust sandwich with Xhat.
    const meat: number[][] = Array.from({ length: inv.length }, () => new Array(inv.length).fill(0));
    for (let i = 0; i < n; i++) {
      for (let a = 0; a < inv.length; a++) {
        for (let b = a; b < inv.length; b++) meat[a][b] += residuals[i] * residuals[i] * xhat[i][a] * xhat[i][b];
      }
    }
    for (let a = 0; a < inv.length; a++) for (let b = a + 1; b < inv.length; b++) meat[b][a] = meat[a][b];
    const v = matMul3(matMul3(inv, meat), inv);
    for (let j = 0; j < inv.length; j++) {
      const s = Math.sqrt(Math.max(v[j][j], 0));
      const zv = beta[j] / s;
      se.push(s);
      z.push(zv);
      p.push(tTwoSidedP(zv, dfR));
      lo.push(beta[j] - tc * s);
      hi.push(beta[j] + tc * s);
    }
  } else {
    const sigma2 = residuals.reduce((s, u) => s + u * u, 0) / dfR;
    for (let j = 0; j < inv.length; j++) {
      const s = Math.sqrt(Math.max(inv[j][j] * sigma2, 0));
      const zv = beta[j] / s;
      se.push(s);
      z.push(zv);
      p.push(tTwoSidedP(zv, dfR));
      lo.push(beta[j] - tc * s);
      hi.push(beta[j] + tc * s);
    }
  }
  // First-stage F for each endogenous regressor: full model (instruments
  // + controls + constant) vs restricted (controls + constant only).
  const fsF: number[] = [];
  const fsR2: number[] = [];
  for (let j = 0; j < e; j++) {
    const endogCol = colOf(xendog, j);
    // zfull already contains the constant column: suppress fitOls's own.
    const full = fitOls({ y: endogCol, x: zfull, names: [], vcov: { type: "classical" }, noConstant: true });
    const restricted = fitOls({ y: endogCol, x: zfull.map((row) => row.slice(instrumentCols.length)), names: [], vcov: { type: "classical" }, noConstant: true });
    const ssrFull = full.residuals.reduce((s, u) => s + u * u, 0);
    const ssrR = restricted.residuals.reduce((s, u) => s + u * u, 0);
    const df1 = instrumentCols.length;
    const df2 = Math.max(n - zk, 1);
    const f = ((ssrR - ssrFull) / df1) / (ssrFull / df2);
    fsF.push(f);
    fsR2.push(ssrR > 0 ? (ssrR - ssrFull) / ssrR : NaN);
  }
  return {
    terms: [...names, "_cons"],
    coef: beta,
    se,
    z,
    p,
    ciLo: lo,
    ciHi: hi,
    n,
    rmse: Math.sqrt(residuals.reduce((s, u) => s + u * u, 0) / n),
    firstStageF: fsF,
    firstStagePartialR2: fsR2,
    firstStageTerms: [...names.slice(0, e)],
    vcovLabel: spec.type === "classical" ? "Classical" : "Robust",
  };
}

function matMul3(a: number[][], b: number[][]): number[][] {
  const n = a.length;
  const out = Array.from({ length: n }, () => new Array(n).fill(0));
  for (let i = 0; i < n; i++) {
    for (let t = 0; t < n; t++) {
      const av = a[i][t];
      if (av === 0) continue;
      for (let j = 0; j < n; j++) out[i][j] += av * b[t][j];
    }
  }
  return out;
}

/* ============ DID (TS port) ============ */

function fitDid(
  y: number[],
  treat: number[],
  post: number[],
  controls: number[][],
  controlNames: string[],
  spec: VcovSpec,
  clusterIds?: (number | string)[],
): DidResult {
  const n = y.length;
  const rows: number[][] = [];
  for (let i = 0; i < n; i++) {
    rows.push([treat[i], post[i], treat[i] * post[i], ...controls.map((c) => c[i])]);
  }
  const names = ["treat", "post", "treat#post", ...controlNames];
  const reg = fitOls({ y, x: rows, names, vcov: spec, clusters: clusterIds, estimatorLabel: "DID" });
  const sums = [0, 0, 0, 0];
  const counts = [0, 0, 0, 0];
  for (let i = 0; i < n; i++) {
    const cell = treat[i] * 2 + post[i];
    sums[cell] += y[i];
    counts[cell]++;
  }
  const means: [number, number][] = [0, 1, 2, 3].map((cell) => [counts[cell] > 0 ? sums[cell] / counts[cell] : NaN, counts[cell]]);
  for (let j = 0; j < 4; j++) {
    if (counts[j] === 0) {
      throw new EngineError("One of the four treatment-by-period cells is empty.", `The DID design requires observations in every (treatment, period) cell; cell ${j} has none.`, "Check the treatment and time variables so all four cells contain data.");
    }
  }
  // fitOls prepends the constant: [const, treat, post, treat#post, …controls]
  // — the interaction (ATT) is term index 3.
  return {
    att: reg.coef[3],
    attSe: reg.se[3],
    t: reg.t[3],
    p: reg.p[3],
    ciLo: reg.ciLo[3],
    ciHi: reg.ciHi[3],
    means,
    n,
    terms: reg.terms,
    coef: reg.coef,
    se: reg.se,
    pValues: reg.p,
    vcovLabel: reg.vcovLabel,
  };
}

/* ============ Helpers ============ */

function completeCases(ds: Dataset, vars: string[]): Set<number> {
  const idx = new Set<number>();
  const n = ds.rows;
  for (let i = 0; i < n; i++) {
    let ok = true;
    for (const v of vars) {
      const num = ds.numeric[v];
      const txt = ds.text[v];
      if (num) {
        if (num[i] === null || num[i] === undefined) {
          ok = false;
          break;
        }
      } else if (txt) {
        if (!txt[i]) {
          ok = false;
          break;
        }
      } else {
        throw new EngineError(`Variable '${v}' was not found in the dataset.`, `The dataset contains these variables: ${Object.keys(ds.numeric).join(", ")}.`, "Check the spelling of the variable name or run 'describe' to list variables.");
      }
    }
    if (ok) idx.add(i);
  }
  return idx;
}

function idxToArray(idx: Set<number>): number[] {
  return [...idx];
}

function gather(ds: Dataset, name: string, idx: Set<number>): number[] {
  const col = ds.numeric[name];
  if (!col) {
    throw new EngineError(`Variable '${name}' was not found or is not numeric.`, "This operation requires a numeric variable.", "Check the variable name and type with 'describe'.");
  }
  return idxToArray(idx).map((i) => col[i] as number);
}

function gatherAsLabels(ds: Dataset, name: string, idx: Set<number>): (number | string)[] {
  const num = ds.numeric[name];
  const txt = ds.text[name];
  if (!num && !txt) {
    throw new EngineError(`Variable '${name}' was not found in the dataset.`, "Panel estimation needs an entity identifier.", "Check the variable name.");
  }
  return idxToArray(idx).map((i) => (num ? num[i] : txt[i]) as number | string);
}

function numericCol(ds: Dataset, name: string): number[] {
  const col = ds.numeric[name];
  if (!col) throw new EngineError(`Variable '${name}' was not found or is not numeric.`, "This operation requires a numeric variable.", "Check the variable name and type with 'describe'.");
  return col.filter((v): v is number => v !== null);
}

function transpose(cols: number[][]): number[][] {
  const n = cols[0]?.length ?? 0;
  const out: number[][] = [];
  for (let i = 0; i < n; i++) out.push(cols.map((c) => c[i]));
  return out;
}

function condHolds(cond: { op: "==" | "!=" | ">" | ">=" | "<" | "<="; value: number }, x: number): boolean {
  switch (cond.op) {
    case "==": return x === cond.value;
    case "!=": return x !== cond.value;
    case ">": return x > cond.value;
    case ">=": return x >= cond.value;
    case "<": return x < cond.value;
    case "<=": return x <= cond.value;
  }
}

function evalExpr(ds: Dataset, expr: Expr): (number | null)[] {
  const n = ds.rows;
  const out: (number | null)[] = new Array(n).fill(null);
  const evalNode = (e: Expr): (number | null)[] => {
    switch (e.type) {
      case "num":
        return new Array(n).fill(e.v);
      case "var": {
        const col = ds.numeric[e.v];
        if (!col) throw new EngineError(`Variable '${e.v}' was not found or is not numeric.`, "Expressions use numeric variables.", "Check the variable name.");
        return [...col];
      }
      case "neg":
        return evalNode(e.v).map((v) => (v === null ? null : -v));
      case "bin":
        return combine(evalNode(e.l), evalNode(e.r), e.op);
      case "call": {
        const a = evalNode(e.arg);
        if (e.fn === "rank") {
          const ranks = rankAverage(a.filter((v): v is number => v !== null));
          let it = 0;
          return a.map((v) => (v === null ? null : ranks[it++]));
        }
        if (e.fn === "standardize") {
          const xs = a.filter((v): v is number => v !== null);
          const m = xs.length ? mean(xs) : 0;
          const s = xs.length > 1 ? Math.sqrt(variance(xs)) : 0;
          return a.map((v) => (v === null || s === 0 ? null : (v - m) / s));
        }
        return a.map((v) => {
          if (v === null) return null;
          const r = e.fn === "ln" ? Math.log(v) : e.fn === "log" ? Math.log10(v) : e.fn === "exp" ? Math.exp(v) : e.fn === "sqrt" ? Math.sqrt(v) : e.fn === "abs" ? Math.abs(v) : Math.round(v);
          return Number.isFinite(r) ? r : null;
        });
      }
    }
  };
  const result = evalNode(expr);
  result.forEach((v, i) => (out[i] = v));
  return out;
}

function combine(a: (number | null)[], b: (number | null)[], op: "+" | "-" | "*" | "/" | "^"): (number | null)[] {
  return a.map((va, i) => {
    const vb = b[i];
    if (va === null || vb === null) return null;
    switch (op) {
      case "+": return va + vb;
      case "-": return va - vb;
      case "*": return va * vb;
      case "/": return vb === 0 ? null : va / vb;
      case "^": return Math.pow(va, vb);
    }
  });
}

export function skewness(xs: number[]): number | null {
  const n = xs.length;
  if (n < 3) return null;
  const m = mean(xs);
  let m2 = 0;
  let m3 = 0;
  for (const x of xs) {
    const d = x - m;
    m2 += d * d;
    m3 += d * d * d;
  }
  m2 /= n;
  m3 /= n;
  const m232 = m2 * Math.sqrt(m2);
  if (m232 <= 0) return null;
  return (n * m3) / ((n - 1) * (n - 2) * m232);
}

export function kurtosis(xs: number[]): number | null {
  const n = xs.length;
  if (n < 4) return null;
  const m = mean(xs);
  let m2 = 0;
  let m4 = 0;
  for (const x of xs) {
    const d = x - m;
    m2 += d * d;
    m4 += d * d * d * d;
  }
  m2 /= n;
  m4 /= n;
  if (m2 <= 0) return null;
  const g2raw = m4 / (m2 * m2) - 3;
  return ((n - 1) / ((n - 2) * (n - 3))) * ((n + 1) * g2raw + 6);
}

function estimateTerms(r: EstimateResult): string[] {
  return "Ols" in r ? r.Ols.terms : "Glm" in r ? r.Glm.terms : "Panel" in r ? r.Panel.terms : "Iv" in r ? r.Iv.terms : r.Did.terms;
}

function estimateCoef(r: EstimateResult): number[] {
  return "Ols" in r ? r.Ols.coef : "Glm" in r ? r.Glm.coef : "Panel" in r ? r.Panel.coef : "Iv" in r ? r.Iv.coef : r.Did.coef;
}

function estimateSe(r: EstimateResult): number[] {
  return "Ols" in r ? r.Ols.se : "Glm" in r ? r.Glm.se : "Panel" in r ? r.Panel.se : "Iv" in r ? r.Iv.se : r.Did.se;
}

function estimateP(r: EstimateResult): number[] {
  return "Ols" in r ? r.Ols.p : "Glm" in r ? r.Glm.p : "Panel" in r ? r.Panel.p : "Iv" in r ? r.Iv.p : r.Did.pValues;
}

export function estimateN(r: EstimateResult): number {
  return "Ols" in r ? r.Ols.n : "Glm" in r ? r.Glm.n : "Panel" in r ? r.Panel.n : "Iv" in r ? r.Iv.n : r.Did.n;
}

function fitLine(r: EstimateResult): string {
  if ("Ols" in r) return `N = ${r.Ols.n}, R² = ${r.Ols.r2.toFixed(3)}`;
  if ("Glm" in r) return `N = ${r.Glm.n}, pseudo-R² = ${r.Glm.pseudoR2.toFixed(3)}`;
  if ("Panel" in r) return `N = ${r.Panel.n}, entities = ${r.Panel.g}, R²(within) = ${r.Panel.r2Within.toFixed(3)}`;
  if ("Iv" in r) return `N = ${r.Iv.n}`;
  return `N = ${r.Did.n}`;
}

function regDiagnostics(reg: import("./stats").Regression, xRaw: number[][]): DiagnosticRow[] {
  const items: DiagnosticRow[] = [];
  items.push({ name: "Sample size", status: "ok", detail: `${reg.n} observations, ${reg.k} parameters, ${reg.dfR} residual degrees of freedom.` });
  try {
    const vifs = vif(xRaw, reg.terms.slice(1));
    if (vifs.length > 0) {
      const max = Math.max(...vifs.map((v) => v.vif));
      const worst = vifs.reduce((a, b) => (b.vif > a.vif ? b : a));
      items.push({
        name: "Multicollinearity (VIF)",
        status: max < 5 ? "ok" : max < 10 ? "warning" : "attention",
        detail: `Maximum VIF is ${max.toFixed(2)} (${worst.variable}). Values above 10 indicate problematic collinearity.`,
      });
    }
  } catch {
    /* diagnostics are best-effort */
  }
  items.push({ name: "Estimator", status: "info", detail: `${reg.estimatorLabel} with ${reg.vcovLabel.toLowerCase()} covariance, computed by QR decomposition.` });
  return items;
}

function helpText(command: string | null): string {
  const general = `Numeris command language — quick reference

Data:      describe · summarize [vars] · correlate vars [, spearman]
Tests:     ttest x == 20 · ttest x, by(g) [welch] · ttest a == b, paired · anova y g
Models:    regress y x1 x2, [robust|hc2|hc3|cluster(v)|vce(hac L)]
           xtreg y x, fe entity(v) · ivregress 2sls y (endog = z) [controls]
           did y [controls], treat(t) time(p) [robust|cluster(v)]
           logit y xs, robust · probit y xs · poisson y xs
Data ops:  generate name = expr · replace name = expr [if cond]
           drop/keep vars | if cond · rename a b · label variable v "text" · sort v [desc]
Research:  estimate list · estimate compare m1 m2 · note "text" · notes

Functions in generate/replace: ln log exp sqrt abs round rank standardize
Comments: // or lines starting with *
Demo data: 600-observation wage panel (firms 1–40, 2018–2022), deterministic.`;
  if (!command) return general;
  switch (command.toLowerCase()) {
    case "regress":
      return "regress depvar indepvars, [robust|hc2|hc3|cluster(varname)|vce(hac L)]\n\nFits OLS by QR decomposition. Default standard errors are classical; 'robust' selects HC1; cluster(varname) computes one-way cluster-robust standard errors with the G/(G-1)·(n-1)/(n-k) correction.";
    case "xtreg":
      return "xtreg depvar indepvars, fe entity(panelvar)\n\nFixed-effects (within) estimator. Singleton entities are dropped with a report; degrees of freedom follow n − g − k.";
    case "ivregress":
      return "ivregress 2sls depvar (endogenous = instruments) [exogenous], [robust]\n\nTwo-stage least squares. Reports first-stage F and partial R² for each endogenous regressor; the order condition is checked before estimation.";
    case "did":
      return "did depvar [controls], treat(tvar) time(pvar) [robust|cluster(varname)]\n\nDifference-in-differences with the interaction term treat#post. treat and time must be 0/1.";
    case "generate":
      return "generate newvar = expression\n\nOperators: + - * / ^ and parentheses. Functions: ln log exp sqrt abs round rank standardize. Missing values propagate.";
    default:
      return `No help entry for '${command}'.\n\n${general}`;
  }
}
