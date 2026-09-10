// Numeris web-preview engine — statistical estimators.
// Faithful port of crates/numeris-stats (OLS/WLS with classical, HC0–HC3,
// cluster and HAC covariance; t-tests; ANOVA; correlation; logit/probit/
// Poisson IRLS; fixed effects; DID; diagnostics).

import { fCdf, tQuantile, tTwoSidedP, chi2Cdf, normalCdf, normalPdf, normalQuantile } from "./dist";
import { type Matrix, leverage, matVec, qrDecompose, qrRankDeficient, qrSolve, withConstant, xtxInv } from "./linalg";

/* ============ Variance estimators ============ */

export type VcovSpec =
  | { type: "classical" }
  | { type: "hc0" }
  | { type: "hc1" }
  | { type: "hc2" }
  | { type: "hc3" }
  | { type: "cluster" }
  | { type: "hac"; lags: number };

export function vcovLabel(spec: VcovSpec): string {
  switch (spec.type) {
    case "classical": return "Classical";
    case "hc0": return "HC0 robust";
    case "hc1": return "HC1 robust";
    case "hc2": return "HC2 robust";
    case "hc3": return "HC3 robust";
    case "cluster": return "Cluster-robust";
    case "hac": return "Newey–West HAC";
  }
}

export class EngineError extends Error {
  constructor(
    public what: string,
    public why: string,
    public action: string,
  ) {
    super(what);
  }
}

export interface Regression {
  terms: string[];
  coef: number[];
  se: number[];
  t: number[];
  p: number[];
  ciLo: number[];
  ciHi: number[];
  n: number;
  k: number;
  dfR: number;
  r2: number;
  adjR2: number;
  f: number | null;
  fP: number | null;
  rmse: number;
  loglik: number;
  aic: number;
  bic: number;
  residuals: number[];
  fitted: number[];
  vcovLabel: string;
  estimatorLabel: string;
}

export interface OlsInput {
  y: number[];
  x: Matrix;
  names: string[];
  vcov: VcovSpec;
  clusters?: (number | string)[];
  weights?: number[];
  noConstant?: boolean;
  estimatorLabel?: string;
}

export function fitOls(input: OlsInput): Regression {
  const { y: yRaw, x: xRaw, names } = input;
  const n0 = yRaw.length;
  let y = yRaw;
  let x = xRaw;
  let isWls = false;
  if (input.weights) {
    const w = input.weights;
    if (w.length !== n0) throw new EngineError("Weights vector length does not match the estimation sample.", "Internal error in WLS assembly.", "Please report this as a bug.");
    if (w.some((v) => !(Number.isFinite(v) && v > 0))) {
      throw new EngineError("All weights must be positive and finite.", "The weight variable contains zero, negative, or missing values.", "Inspect the weight variable and choose weights that are strictly positive.");
    }
    const sw = w.map(Math.sqrt);
    y = yRaw.map((v, i) => v * sw[i]);
    x = xRaw.map((row) => row.map((v, j) => v * sw[j]));
    isWls = true;
  }
  const constant = !input.noConstant;
  const design = constant ? withConstant(x) : x;
  const terms = constant ? ["_cons", ...names] : [...names];
  const n = design.length;
  const k = design[0].length;
  if (n <= k) {
    throw new EngineError(`The estimation sample (${n} observations) is too small for ${k} parameters.`, "Linear regression requires more observations than estimated parameters.", "Use fewer predictors or a larger sample.");
  }
  const qr = qrDecompose(design);
  const deficient = qrRankDeficient(qr);
  if (deficient.length > 0) {
    const bad = deficient.map((i) => terms[i]).join(", ");
    throw new EngineError("The regression is not identified: some predictors are perfectly collinear with the others.", `The following terms are redundant in the design matrix: ${bad}. Exact linear dependence makes the coefficients non-estimable.`, "Drop one of the collinear variables, or inspect collinearity via diagnostics.");
  }
  const beta = qrSolve(qr, y);
  const fitted = matVec(design, beta);
  const residuals = y.map((v, i) => v - fitted[i]);
  const inv = xtxInv(qr);
  const lev = leverage(design, inv);
  const ssr = residuals.reduce((s, u) => s + u * u, 0);
  const dfR = n - k;
  const sigma2 = ssr / dfR;
  const rmse = Math.sqrt(sigma2);
  const ybar = y.reduce((s, v) => s + v, 0) / n;
  const sst = y.reduce((s, v) => s + (v - ybar) * (v - ybar), 0);
  const r2 = sst > 0 ? Math.min(Math.max(1 - ssr / sst, -1e-12), 1) : ssr < 1e-14 ? 1 : 0;
  const adjR2 = sst > 0 ? 1 - (1 - r2) * ((n - 1) / dfR) : 0;
  let f: number | null = null;
  let fP: number | null = null;
  if (constant && k > 1 && r2 < 1) {
    f = (r2 / (k - 1)) / ((1 - r2) / dfR);
    if (Number.isFinite(f) && f >= 0) fP = 1 - fCdf(f, k - 1, dfR);
    else { f = null; }
  }
  const sigma2Mle = ssr > 0 ? ssr / n : 1e-300;
  const loglik = -0.5 * n * (Math.log(2 * Math.PI) + Math.log(sigma2Mle) + 1);
  const aic = -2 * loglik + 2 * k;
  const bic = -2 * loglik + Math.log(n) * k;

  // Covariance of beta-hat.
  const v = vcovMatrix(design, residuals, inv, lev, input.clusters, input.vcov, n, k, dfR);
  const tcrit = tQuantile(0.975, dfR);
  const se: number[] = [];
  const tstat: number[] = [];
  const pval: number[] = [];
  const lo: number[] = [];
  const hi: number[] = [];
  for (let j = 0; j < k; j++) {
    const s = Math.sqrt(Math.max(v[j][j], 0));
    const t = s > 0 ? beta[j] / s : NaN;
    se.push(s);
    tstat.push(t);
    pval.push(s > 0 ? tTwoSidedP(t, dfR) : NaN);
    lo.push(beta[j] - tcrit * s);
    hi.push(beta[j] + tcrit * s);
  }
  return {
    terms,
    coef: beta,
    se,
    t: tstat,
    p: pval,
    ciLo: lo,
    ciHi: hi,
    n,
    k,
    dfR,
    r2,
    adjR2,
    f,
    fP,
    rmse,
    loglik,
    aic,
    bic,
    residuals,
    fitted,
    vcovLabel: vcovLabel(input.vcov),
    estimatorLabel: isWls && !input.estimatorLabel ? "WLS" : input.estimatorLabel ?? "OLS",
  };
}

function vcovMatrix(
  design: Matrix,
  residuals: number[],
  inv: Matrix,
  lev: number[],
  clusters: (number | string)[] | undefined,
  spec: VcovSpec,
  n: number,
  k: number,
  dfR: number,
): Matrix {
  const km = inv.length;
  const meat: Matrix = Array.from({ length: km }, () => new Array(km).fill(0));
  const addOuter = (i: number, wu2: number) => {
    for (let a = 0; a < km; a++) {
      if (design[i][a] === 0) continue;
      for (let b = a; b < km; b++) meat[a][b] += wu2 * design[i][a] * design[i][b];
    }
  };
  const symmetrize = () => {
    for (let a = 0; a < km; a++) for (let b = a + 1; b < km; b++) meat[b][a] = meat[a][b];
  };
  switch (spec.type) {
    case "classical":
      return inv.map((row) => row.map((v) => v * (residuals.reduce((s, u) => s + u * u, 0) / dfR)));
    case "hc0":
    case "hc1":
    case "hc2":
    case "hc3": {
      for (let i = 0; i < n; i++) {
        const h = Math.min(Math.max(lev[i], 0), 0.999999);
        const w = spec.type === "hc0" || spec.type === "hc1" ? 1 : spec.type === "hc2" ? 1 / (1 - h) : 1 / ((1 - h) * (1 - h));
        addOuter(i, w * residuals[i] * residuals[i]);
      }
      if (spec.type === "hc1") for (let a = 0; a < km; a++) for (let b = 0; b < km; b++) meat[a][b] *= n / dfR;
      symmetrize();
      break;
    }
    case "cluster": {
      if (!clusters) throw new EngineError("Clustered standard errors require a cluster variable.", "The cluster() option was not given a variable to cluster on.", "Add the cluster option, for example: regress y x, cluster(firm).");
      const groupMap = new Map<string | number, number[]>();
      clusters.forEach((c, i) => {
        if (!groupMap.has(c)) groupMap.set(c, []);
        groupMap.get(c)!.push(i);
      });
      const g = groupMap.size;
      if (g < 2) {
        throw new EngineError("Clustered standard errors require at least two clusters.", "All estimation rows belong to a single cluster, so between-cluster variation cannot be estimated.", "Choose a cluster variable with more groups, or use robust standard errors instead.");
      }
      for (const rows of groupMap.values()) {
        const scores = new Array(km).fill(0);
        for (const i of rows) for (let a = 0; a < km; a++) scores[a] += design[i][a] * residuals[i];
        for (let a = 0; a < km; a++) for (let b = a; b < km; b++) meat[a][b] += scores[a] * scores[b];
      }
      symmetrize();
      const c = (g / (g - 1)) * ((n - 1) / dfR);
      for (let a = 0; a < km; a++) for (let b = 0; b < km; b++) meat[a][b] *= c;
      break;
    }
    case "hac": {
      const l = spec.lags;
      if (l + 1 >= n) throw new EngineError(`HAC lag order ${l} is too large for ${n} observations.`, "The Bartlett kernel requires the lag order to be smaller than the sample size.", "Choose a smaller lag order, for example floor(n^(1/4)) as a starting point.");
      // Gamma_0.
      for (let i = 0; i < n; i++) addOuter(i, residuals[i] * residuals[i]);
      for (let lag = 1; lag <= l; lag++) {
        const w = 1 - lag / (l + 1);
        for (let t = lag; t < n; t++) {
          const wu = residuals[t] * residuals[t - lag];
          for (let a = 0; a < km; a++) {
            for (let b = 0; b < km; b++) {
              meat[a][b] += w * wu * design[t][a] * design[t - lag][b];
            }
          }
        }
      }
      for (let a = 0; a < km; a++) for (let b = 0; b < km; b++) meat[a][b] *= n / dfR;
      break;
    }
  }
  const left = matMulAB(inv, meat);
  return matMulAB(left, inv);
}

function matMulAB(a: Matrix, b: Matrix): Matrix {
  const n = a.length;
  const p = b[0].length;
  const kk = b.length;
  const out: Matrix = Array.from({ length: n }, () => new Array(p).fill(0));
  for (let i = 0; i < n; i++) {
    for (let t = 0; t < kk; t++) {
      const av = a[i][t];
      if (av === 0) continue;
      for (let j = 0; j < p; j++) out[i][j] += av * b[t][j];
    }
  }
  return out;
}

/* ============ t-tests, ANOVA, correlation ============ */

export function mean(xs: number[]): number {
  return xs.reduce((s, v) => s + v, 0) / xs.length;
}

export function variance(xs: number[]): number {
  const m = mean(xs);
  return xs.reduce((s, v) => s + (v - m) * (v - m), 0) / (xs.length - 1);
}

export function quantileSorted(sorted: number[], p: number): number {
  const n = sorted.length;
  if (n === 0) return NaN;
  if (n === 1) return sorted[0];
  const h = (n - 1) * Math.min(Math.max(p, 0), 1);
  const lo = Math.floor(h);
  const hi = Math.min(lo + 1, n - 1);
  return sorted[lo] + (h - lo) * (sorted[hi] - sorted[lo]);
}

export interface TTestResult {
  kind: "OneSample" | "Paired" | "TwoSamplePooled" | "Welch";
  variable: string;
  by: string | null;
  mu0: number;
  n1: number;
  n2: number | null;
  mean1: number;
  mean2: number | null;
  sd1: number;
  sd2: number | null;
  diff: number;
  se: number;
  t: number;
  df: number;
  p: number;
  ciLo: number;
  ciHi: number;
}

export function tTestOneSample(xs: number[], mu0: number, variable: string): TTestResult {
  const n = xs.length;
  if (n < 2) throw new EngineError(`A t-test requires at least two observations; '${variable}' has ${n} non-missing values.`, "The sample standard deviation is undefined with fewer than two observations.", "Inspect the variable for missing values, then rerun the test.");
  const m = mean(xs);
  const s = Math.sqrt(variance(xs));
  if (s === 0) throw new EngineError(`The t-test is undefined for '${variable}' because the sample variance is zero.`, "With no variation in the data, the standard error of the mean is zero and the t statistic cannot be computed.", "Check whether the variable is constant in this sample; a t-test of a constant is not meaningful.");
  const se = s / Math.sqrt(n);
  const t = (m - mu0) / se;
  const df = n - 1;
  const p = tTwoSidedP(t, df);
  const tc = tQuantile(0.975, df);
  return { kind: "OneSample", variable, by: null, mu0, n1: n, n2: null, mean1: m, mean2: null, sd1: s, sd2: null, diff: m, se, t, df, p, ciLo: m - tc * se, ciHi: m + tc * se };
}

export function tTestTwoSample(a: number[], b: number[], welch: boolean, by: string, variable: string): TTestResult {
  if (a.length < 2 || b.length < 2) {
    throw new EngineError(`A two-sample t-test requires at least two observations per group; group sizes are ${a.length} and ${b.length}.`, "The group standard deviations are undefined with fewer than two observations.", "Check the grouping variable and the outcome for missing values, then rerun.");
  }
  const n1 = a.length;
  const n2 = b.length;
  const m1 = mean(a);
  const m2 = mean(b);
  const s1 = Math.sqrt(variance(a));
  const s2 = Math.sqrt(variance(b));
  let se: number;
  let df: number;
  if (welch) {
    const v1 = (s1 * s1) / n1;
    const v2 = (s2 * s2) / n2;
    se = Math.sqrt(v1 + v2);
    df = ((v1 + v2) * (v1 + v2)) / ((v1 * v1) / (n1 - 1) + (v2 * v2) / (n2 - 1));
  } else {
    const sp2 = ((n1 - 1) * s1 * s1 + (n2 - 1) * s2 * s2) / (n1 + n2 - 2);
    se = Math.sqrt(sp2) * Math.sqrt(1 / n1 + 1 / n2);
    df = n1 + n2 - 2;
  }
  if (se === 0) {
    throw new EngineError(`The t-test for '${variable}' by '${by}' is undefined because the pooled standard error is zero.`, "At least one group is constant, so there is no within-group variation to estimate the standard error.", "Check the outcome values within each group; a t-test requires variation in both groups.");
  }
  const diff = m1 - m2;
  const t = diff / se;
  const p = tTwoSidedP(t, df);
  const tc = tQuantile(0.975, df);
  return {
    kind: welch ? "Welch" : "TwoSamplePooled",
    variable,
    by,
    mu0: 0,
    n1,
    n2,
    mean1: m1,
    mean2: m2,
    sd1: s1,
    sd2: s2,
    diff,
    se,
    t,
    df,
    p,
    ciLo: diff - tc * se,
    ciHi: diff + tc * se,
  };
}

export function tTestPaired(a: number[], b: number[], varA: string, varB: string): TTestResult {
  const d = a.map((v, i) => v - b[i]);
  const r = tTestOneSample(d, 0, `${varA} - ${varB}`);
  return { ...r, kind: "Paired", variable: `${varA} - ${varB}`, mean2: mean(b), sd2: Math.sqrt(variance(b)), n2: b.length };
}

export interface AnovaResult {
  variable: string;
  by: string;
  groups: { label: string; n: number; mean: number; sd: number | null }[];
  n: number;
  dfB: number;
  dfW: number;
  f: number;
  p: number;
  eta2: number;
  grandMean: number;
}

export function oneWayAnova(values: number[], labels: string[], by: string, variable: string): AnovaResult {
  const unique = [...new Set(labels)].sort();
  const g = unique.length;
  if (g < 2) throw new EngineError(`ANOVA requires at least two groups; '${by}' defines ${g} group.`, "With a single group there is no between-group variation to test.", "Choose a grouping variable that defines at least two groups.");
  const n = values.length;
  const grand = mean(values);
  let ssb = 0;
  let ssw = 0;
  const groups: AnovaResult["groups"] = [];
  for (const label of unique) {
    const ys = values.filter((_, i) => labels[i] === label);
    const gm = mean(ys);
    const gsd = ys.length > 1 ? Math.sqrt(variance(ys)) : null;
    ssb += ys.length * (gm - grand) * (gm - grand);
    ssw += ys.reduce((s, v) => s + (v - gm) * (v - gm), 0);
    groups.push({ label, n: ys.length, mean: gm, sd: gsd });
  }
  const dfB = g - 1;
  const dfW = n - g;
  if (dfW === 0) throw new EngineError("ANOVA has no within-group degrees of freedom.", `Each of the ${g} groups contains a single observation, so the error variance cannot be estimated.`, "Use a sample with more than one observation per group.");
  const msw = ssw / dfW;
  if (msw <= 0) throw new EngineError("ANOVA is undefined because the within-group variance is zero.", "All observations within each group are identical, so the F statistic cannot be computed.", "Check the outcome variable; ANOVA requires within-group variation.");
  const f = ssb / dfB / msw;
  return { variable, by, groups, n, dfB, dfW, f, p: 1 - fCdf(f, dfB, dfW), eta2: ssb + ssw > 0 ? ssb / (ssb + ssw) : 0, grandMean: grand };
}

export function rankAverage(xs: number[]): number[] {
  const n = xs.length;
  const idx = Array.from({ length: n }, (_, i) => i).sort((a, b) => xs[a] - xs[b]);
  const ranks = new Array(n).fill(0);
  let i = 0;
  while (i < n) {
    let j = i;
    while (j + 1 < n && xs[idx[j + 1]] === xs[idx[i]]) j++;
    const avg = (i + j) / 2 + 1;
    for (let p = i; p <= j; p++) ranks[idx[p]] = avg;
    i = j + 1;
  }
  return ranks;
}

export function pearson(xs: number[], ys: number[]): number | null {
  if (xs.length !== ys.length || xs.length < 2) return null;
  const mx = mean(xs);
  const my = mean(ys);
  let sxy = 0;
  let sxx = 0;
  let syy = 0;
  for (let i = 0; i < xs.length; i++) {
    const dx = xs[i] - mx;
    const dy = ys[i] - my;
    sxy += dx * dy;
    sxx += dx * dx;
    syy += dy * dy;
  }
  if (sxx <= 0 || syy <= 0) return null;
  return Math.min(Math.max(sxy / Math.sqrt(sxx * syy), -1), 1);
}

export interface CorrResult {
  variables: string[];
  values: (number | null)[][];
  pValues: (number | null)[][];
  n: number;
  method: string;
}

export function corrMatrix(columns: number[][], names: string[], spearman: boolean): CorrResult {
  let work = columns;
  if (spearman) work = columns.map(rankAverage);
  const k = names.length;
  const values: (number | null)[][] = Array.from({ length: k }, () => new Array(k).fill(null));
  const pValues: (number | null)[][] = Array.from({ length: k }, () => new Array(k).fill(null));
  const n = work[0]?.length ?? 0;
  for (let a = 0; a < k; a++) {
    values[a][a] = 1;
    for (let b = a + 1; b < k; b++) {
      const r = pearson(work[a], work[b]);
      values[a][b] = r;
      values[b][a] = r;
      if (r !== null && Math.abs(r) < 1 && n > 2) {
        const t = r * Math.sqrt((n - 2) / (1 - r * r));
        const p = tTwoSidedP(t, n - 2);
        pValues[a][b] = p;
        pValues[b][a] = p;
      } else if (r !== null && Math.abs(Math.abs(r) - 1) < 1e-12) {
        pValues[a][b] = 0;
        pValues[b][a] = 0;
      }
    }
  }
  return { variables: names, values, pValues, n, method: spearman ? "Spearman" : "Pearson" };
}

/* ============ GLM (logit / probit / Poisson) ============ */

export type Family = "Logit" | "Probit" | "Poisson";

export interface GlmResult {
  family: Family;
  terms: string[];
  coef: number[];
  se: number[];
  z: number[];
  p: number[];
  ciLo: number[];
  ciHi: number[];
  n: number;
  k: number;
  loglik: number;
  pseudoR2: number;
  aic: number;
  bic: number;
  iterations: number;
  converged: boolean;
  vcovLabel: string;
  warnings: string[];
}

export function fitGlm(y: number[], xs: Matrix, names: string[], family: Family, robust: boolean): GlmResult {
  const n = y.length;
  const k = xs[0]?.length ?? 0;
  if (n < k + 2) throw new EngineError(`The estimation sample (${n} observations) is too small for ${k + 1} predictors.`, "Iterative estimation requires more observations than parameters.", "Use fewer predictors or a larger sample.");
  for (let i = 0; i < n; i++) {
    if (family !== "Poisson") {
      if (y[i] !== 0 && y[i] !== 1) {
        throw new EngineError(`The outcome must be 0/1; value ${y[i]} was found at observation ${i + 1}.`, family === "Probit" ? "Probit regression models a binary outcome." : "Logistic regression models a binary outcome.", "Recode the outcome to 0/1, or use a different model family.");
      }
    } else if (!(y[i] >= 0 && Number.isFinite(y[i])) || y[i] % 1 !== 0) {
      throw new EngineError(`The outcome must be a non-negative integer count; value ${y[i]} was found at observation ${i + 1}.`, "Poisson regression models count data.", "Check the outcome variable, or use a model appropriate for continuous outcomes.");
    }
  }
  const design: Matrix = xs.map((row) => [1, ...row]);
  const kk = design[0].length;
  const terms = ["_cons", ...names];
  const ybar = mean(y);
  let beta = new Array(kk).fill(0);
  beta[0] = family === "Logit" ? Math.log(ybar / (1 - ybar)) : family === "Probit" ? normalQuantile(Math.min(Math.max(ybar, 1e-6), 1 - 1e-6)) : Math.log(Math.max(ybar, 1e-6));
  const warnings: string[] = [];
  const w = new Array(n).fill(0);
  const z = new Array(n).fill(0);
  const eta = new Array(n).fill(0);
  const mu = new Array(n).fill(0);
  let iterations = 0;
  let converged = false;
  let prevLl: number | null = null;
  const maxIter = 200;

  while (true) {
    iterations++;
    for (let i = 0; i < n; i++) {
      eta[i] = design[i].reduce((s, a, j) => s + a * beta[j], 0);
      if (family === "Logit") {
        const e = Math.min(Math.max(eta[i], -30), 30);
        mu[i] = 1 / (1 + Math.exp(-e));
        const m = Math.min(Math.max(mu[i], 1e-10), 1 - 1e-10);
        w[i] = m * (1 - m);
        z[i] = e + (y[i] - mu[i]) / (m * (1 - m));
      } else if (family === "Probit") {
        mu[i] = normalCdf(eta[i]);
        const m = Math.min(Math.max(mu[i], 1e-10), 1 - 1e-10);
        const phi = normalPdf(eta[i]);
        const denom = m * (1 - m);
        w[i] = phi > 1e-300 ? (phi * phi) / denom : 1e-10;
        z[i] = eta[i] + ((y[i] - mu[i]) * denom) / Math.max(phi, 1e-300);
      } else {
        const e = Math.min(eta[i], 30);
        mu[i] = Math.exp(e);
        w[i] = mu[i];
        z[i] = e + (y[i] - mu[i]) / mu[i];
      }
      if (!Number.isFinite(w[i]) || w[i] <= 0) w[i] = 1e-10;
      if (!Number.isFinite(z[i])) {
        throw new EngineError("The iterative estimation encountered invalid intermediate values.", "Extreme outcomes or extreme predictor values produced non-finite working responses.", "Check the outcome and predictors for extreme values; consider rescaling large variables.");
      }
    }
    if (family !== "Poisson" && eta.some((e) => Math.abs(e) > 20) && !warnings.some((x) => x.includes("separation"))) {
      warnings.push("Quasi-complete separation may be present: the linear predictor is very large in magnitude, so coefficients may diverge.");
    }
    const sqrtw = w.map(Math.sqrt);
    const xw = design.map((row, i) => row.map((v) => v * sqrtw[i]));
    const zw = z.map((v, i) => v * sqrtw[i]);
    const qr = qrDecompose(xw);
    if (qrRankDeficient(qr).length > 0) {
      throw new EngineError("The model is not identified: predictors are perfectly collinear.", "The weighted design matrix is rank deficient.", "Drop one of the collinear predictors and refit.");
    }
    const newBeta = qrSolve(qr, zw);
    const delta = Math.max(...newBeta.map((v, j) => Math.abs(v - beta[j])));
    beta = newBeta;
    const ll = glmLoglik(mu, eta, y, family);
    if (delta < 1e-10) {
      converged = true;
      break;
    }
    if (prevLl !== null && Math.abs(ll - prevLl) < 1e-10 * (1 + Math.abs(ll))) {
      converged = true;
      break;
    }
    prevLl = ll;
    if (iterations >= maxIter) {
      warnings.push(`The ${familyLabel(family).toLowerCase()} did not converge within ${maxIter} iterations; coefficients may be unreliable. This often indicates separation or an oversized model.`);
      break;
    }
  }
  const ll = glmLoglik(mu, eta, y, family);
  // Null likelihood.
  const ll0 = family === "Poisson" ? y.reduce((s, v) => s + v * Math.log(Math.max(ybar, 1e-10)) - Math.max(ybar, 1e-10) - lnFact(v), 0) : n * (ybar * Math.log(Math.min(Math.max(ybar, 1e-10), 1)) + (1 - ybar) * Math.log(Math.min(Math.max(1 - ybar, 1e-10), 1)));
  const pseudoR2 = Math.abs(ll0) > 1e-12 ? 1 - ll / ll0 : 0;
  // Bread at convergence.
  const sqrtw2 = w.map(Math.sqrt);
  const xw2 = design.map((row, i) => row.map((v) => v * sqrtw2[i]));
  const qr2 = qrDecompose(xw2);
  const bread = xtxInv(qr2);
  let v = bread;
  if (robust) {
    const meat: Matrix = Array.from({ length: kk }, () => new Array(kk).fill(0));
    for (let i = 0; i < n; i++) {
      const u2 = (y[i] - mu[i]) * (y[i] - mu[i]);
      for (let a = 0; a < kk; a++) {
        if (design[i][a] === 0) continue;
        for (let b = a; b < kk; b++) meat[a][b] += u2 * design[i][a] * design[i][b];
      }
    }
    for (let a = 0; a < kk; a++) for (let b = a + 1; b < kk; b++) meat[b][a] = meat[a][b];
    v = matMulAB(matMulAB(bread, meat), bread);
  }
  const zc = 1.959963984540054;
  const se: number[] = [];
  const zstat: number[] = [];
  const pval: number[] = [];
  const lo: number[] = [];
  const hi: number[] = [];
  for (let j = 0; j < kk; j++) {
    const s = Math.sqrt(Math.max(v[j][j], 0));
    const z = s > 0 ? beta[j] / s : NaN;
    se.push(s);
    zstat.push(z);
    pval.push(s > 0 ? 2 * (1 - normalCdf(Math.abs(z))) : NaN);
    lo.push(beta[j] - zc * s);
    hi.push(beta[j] + zc * s);
  }
  return {
    family,
    terms,
    coef: beta,
    se,
    z: zstat,
    p: pval,
    ciLo: lo,
    ciHi: hi,
    n,
    k: kk,
    loglik: ll,
    pseudoR2,
    aic: -2 * ll + 2 * kk,
    bic: -2 * ll + Math.log(n) * kk,
    iterations,
    converged,
    vcovLabel: robust ? "Robust" : "Classical",
    warnings,
  };
}

function familyLabel(f: Family): string {
  return f === "Logit" ? "Logistic regression" : f === "Probit" ? "Probit regression" : "Poisson regression";
}

function lnFact(v: number): number {
  // ln(v!) for integer v.
  let s = 0;
  for (let i = 2; i <= v; i++) s += Math.log(i);
  return s;
}

function glmLoglik(mu: number[], eta: number[], y: number[], family: Family): number {
  if (family === "Poisson") {
    let ll = 0;
    for (let i = 0; i < y.length; i++) ll += y[i] * eta[i] - mu[i] - lnFact(y[i]);
    return ll;
  }
  let ll = 0;
  for (let i = 0; i < y.length; i++) {
    ll += y[i] * Math.log(Math.min(Math.max(mu[i], 1e-300), 1)) + (1 - y[i]) * Math.log(Math.min(Math.max(1 - mu[i], 1e-300), 1));
  }
  return ll;
}

/* ============ Panel: fixed effects ============ */

export interface PanelResult {
  estimator: string;
  terms: string[];
  coef: number[];
  se: number[];
  t: number[];
  p: number[];
  ciLo: number[];
  ciHi: number[];
  n: number;
  g: number;
  droppedSingletons: number;
  k: number;
  dfR: number;
  r2Within: number;
  sigma: number;
  vcovLabel: string;
}

export function fixedEffects(
  y: number[],
  xs: Matrix,
  entity: (number | string)[],
  names: string[],
  spec: VcovSpec,
  clusterIds?: (number | string)[],
): PanelResult {
  const n = y.length;
  const k = xs[0]?.length ?? 0;
  if (k === 0) throw new EngineError("The fixed-effects model has no predictors.", "xtreg requires at least one regressor besides the entity identifier.", "Add a predictor to the model.");
  const groups = new Map<string | number, number[]>();
  for (let i = 0; i < n; i++) {
    if (!groups.has(entity[i])) groups.set(entity[i], []);
    groups.get(entity[i])!.push(i);
  }
  let dropped = 0;
  for (const rows of groups.values()) if (rows.length < 2) dropped++;
  const usedGroups = groups.size - dropped;
  if (usedGroups < 2) {
    throw new EngineError("The fixed-effects estimator requires at least two entities with two or more observations each.", `After dropping singleton entities, ${usedGroups} usable entities remain.`, "Check the entity variable; xtreg, fe needs repeated observations within entities.");
  }
  const yDm: number[] = [];
  const xDm: number[][] = [];
  const entityUsed: (number | string)[] = [];
  for (const [e, rows] of groups) {
    if (rows.length < 2) continue;
    const ybar = rows.reduce((s, i) => s + y[i], 0) / rows.length;
    const xbars = Array.from({ length: k }, (_, j) => rows.reduce((s, i) => s + xs[i][j], 0) / rows.length);
    for (const i of rows) {
      yDm.push(y[i] - ybar);
      xDm.push(xs[i].map((v, j) => v - xbars[j]));
      entityUsed.push(e);
    }
  }
  const nUsed = yDm.length;
  const dfR = nUsed - usedGroups - k;
  if (dfR <= 0) {
    throw new EngineError("The fixed-effects model has no residual degrees of freedom.", `With ${nUsed} observations, ${usedGroups} entities and ${k} regressors, no degrees of freedom remain.`, "Use a larger panel or fewer regressors.");
  }
  const qr = qrDecompose(xDm);
  const deficient = qrRankDeficient(qr);
  if (deficient.length > 0) {
    const bad = deficient.map((i) => names[i]).join(", ");
    throw new EngineError("The fixed-effects model is not identified: time-invariant regressors are absorbed by the entity effects.", `After within-transformation, these regressors have no variation: ${bad}.`, "Remove time-invariant regressors from the model; their effects cannot be separated from the entity fixed effects.");
  }
  const beta = qrSolve(qr, yDm);
  const fitted = matVec(xDm, beta);
  const residuals = yDm.map((v, i) => v - fitted[i]);
  const ssr = residuals.reduce((s, u) => s + u * u, 0);
  const sst = yDm.reduce((s, v) => s + v * v, 0);
  const r2w = sst > 0 ? Math.min(Math.max(1 - ssr / sst, 0), 1) : 0;
  const sigma2 = ssr / dfR;
  const inv = xtxInv(qr);
  const lev = leverage(xDm, inv);
  const v =
    spec.type === "classical"
      ? inv.map((row) => row.map((c) => c * sigma2))
      : vcovMatrix(xDm, residuals, inv, lev, spec.type === "cluster" ? (clusterIds !== undefined ? clusterIds : entityUsed) : undefined, spec, nUsed, k, dfR);
  const tc = tQuantile(0.975, dfR);
  const se: number[] = [];
  const t: number[] = [];
  const p: number[] = [];
  const lo: number[] = [];
  const hi: number[] = [];
  for (let j = 0; j < k; j++) {
    const s = Math.sqrt(Math.max(v[j][j], 0));
    const tv = s > 0 ? beta[j] / s : NaN;
    se.push(s);
    t.push(tv);
    p.push(s > 0 ? tTwoSidedP(tv, dfR) : NaN);
    lo.push(beta[j] - tc * s);
    hi.push(beta[j] + tc * s);
  }
  return {
    estimator: "Fixed effects (within)",
    terms: [...names],
    coef: beta,
    se,
    t,
    p,
    ciLo: lo,
    ciHi: hi,
    n: nUsed,
    g: usedGroups,
    droppedSingletons: dropped,
    k,
    dfR,
    r2Within: r2w,
    sigma: Math.sqrt(sigma2),
    vcovLabel: vcovLabel(spec),
  };
}

/* ============ Diagnostics ============ */

export function vif(x: Matrix, names: string[]): { variable: string; vif: number; r2: number }[] {
  const out: { variable: string; vif: number; r2: number }[] = [];
  const nCols = x.length > 0 ? x[0].length : 0;
  for (let j = 0; j < nCols; j++) {
    const yj = x.map((row) => row[j]);
    const others = x.map((row) => row.filter((_, c) => c !== j));
    const design = withConstant(others);
    try {
      const qr = qrDecompose(design);
      if (qrRankDeficient(qr).length > 0) continue;
      const beta = qrSolve(qr, yj);
      const fitted = matVec(design, beta);
      const ssr = yj.reduce((s, v, i) => s + (v - fitted[i]) * (v - fitted[i]), 0);
      const ybar = mean(yj);
      const sst = yj.reduce((s, v) => s + (v - ybar) * (v - ybar), 0);
      if (sst <= 0) continue;
      const r2 = Math.min(Math.max(1 - ssr / sst, 0), 1 - 1e-12);
      if (r2 < 1) out.push({ variable: names[j], vif: 1 / (1 - r2), r2 });
    } catch {
      /* skip */
    }
  }
  return out;
}
