// Bridge to the Rust engine over Tauri IPC.
// The frontend contains NO statistical algorithms — it sends specifications
// to the engine and renders structured results.

import { invoke } from "@tauri-apps/api/core";

/** Mirrors numeris-command::ExecOutput (serialized as JSON). */
export type ExecOutput =
  | { DatasetChanged: { message: string } }
  | { DataLoaded: { message: string; report: ImportReport; variables: VariableInfo[] } }
  | { Summary: SummaryStats[] }
  | { Describe: VariableInfo[] }
  | { Frequency: { variable: string; rows: FrequencyRow[] } }
  | { Corr: CorrResult }
  | { TTest: TTestResult }
  | { Anova: AnovaResult }
  | { Estimate: { id: string; label: string; result: EstimateResult; diagnostics: DiagnosticItem[] } }
  | { EstimateList: EstimateSummary[] }
  | { Compare: ComparisonResult }
  | { Notes: string[] }
  | { Message: string }
  | { Help: string };

export interface ImportReport {
  rows: number;
  columns: number;
  delimiter: string;
  issues: string[];
}

export interface VariableInfo {
  name: string;
  label: string;
  storage: "Numeric" | "Text";
  semantic: "Continuous" | "Categorical" | "Identifier" | "Temporal" | "Unknown";
}

export interface SummaryStats {
  name: string;
  n: number;
  missing: number;
  mean: number | null;
  variance: number | null;
  sd: number | null;
  min: number | null;
  max: number | null;
  median: number | null;
  q1: number | null;
  q3: number | null;
  skewness: number | null;
  kurtosis: number | null;
  unique: number;
  sum: number | null;
}

export interface FrequencyRow {
  value: string;
  count: number;
  percent: number;
  cumulative_percent: number;
}

export interface CorrResult {
  variables: string[];
  values: (number | null)[][];
  p_values: (number | null)[][];
  n: number;
  method: string;
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
  p_two_sided: number;
  ci_lo: number;
  ci_hi: number;
}

export interface AnovaResult {
  variable: string;
  by: string;
  groups: { label: string; n: number; mean: number; sd: number | null }[];
  n: number;
  df_between: number;
  df_within: number;
  f: number;
  p: number;
  eta_squared: number;
  grand_mean: number;
}

export interface DiagnosticItem {
  name: string;
  status: "ok" | "warning" | "attention" | "info";
  detail: string;
}

/** Regression result (mirrors numeris-stats::Regression). */
export interface Regression {
  terms: string[];
  coef: number[];
  se: number[];
  t: number[];
  p: number[];
  ci_lo: number[];
  ci_hi: number[];
  n: number;
  k: number;
  df_r: number;
  r2: number;
  adj_r2: number;
  f: number | null;
  f_p: number | null;
  f_df: [number, number] | null;
  rmse: number;
  loglik: number;
  aic: number;
  bic: number;
  residuals: number[];
  fitted: number[];
  vcov_label: string;
  estimator_label: string;
}

export interface GlmResult {
  family: "Logit" | "Probit" | "Poisson";
  terms: string[];
  coef: number[];
  se: number[];
  z: number[];
  p: number[];
  n: number;
  k: number;
  loglik: number;
  pseudo_r2_mcfadden: number;
  aic: number;
  bic: number;
  iterations: number;
  converged: boolean;
  vcov_label: string;
  warnings: string[];
}

export interface PanelResult {
  estimator: string;
  terms: string[];
  coef: number[];
  se: number[];
  t: number[];
  p: number[];
  n: number;
  g: number;
  k: number;
  df_r: number;
  dropped_singletons: number;
  r2_within: number;
  sigma: number;
  vcov_label: string;
}

export interface IvResult {
  terms: string[];
  coef: number[];
  se: number[];
  z: number[];
  p: number[];
  n: number;
  rmse: number;
  first_stage_f: number[];
  first_stage_partial_r2: number[];
  first_stage_terms: string[];
  vcov_label: string;
}

export interface DidResult {
  att: number;
  att_se: number;
  t: number;
  p: number;
  ci_lo: number;
  ci_hi: number;
  means: [number, number][];
  n: number;
  terms: string[];
  coef: number[];
  se: number[];
  p_values: number[];
}

export type EstimateResult =
  | { Ols: Regression }
  | { Glm: GlmResult }
  | { Panel: PanelResult }
  | { Iv: IvResult }
  | { Did: DidResult };

export interface EstimateSummary {
  id: string;
  label: string;
  command: string;
  kind: string;
  n: number;
  fit: string;
}

export interface ComparisonResult {
  ids: string[];
  labels: string[];
  terms: string[];
  coef: (number | null)[][];
  se: (number | null)[][];
  p: (number | null)[][];
  n: number[];
  fit: string[];
}

/** The what/why/action error contract. */
export interface ApiError {
  what: string;
  why: string;
  action: string;
}

export interface DatasetPreview {
  variables: VariableInfo[];
  rows: [ColumnData][];
  n_rows: number;
}

export type ColumnData =
  | { Numeric: (number | null)[] }
  | { Text: (string | null)[] };

export interface ReproducibilityReport {
  project: string;
  software_version: string;
  commands_replayed: number;
  reproduced: number;
  failed: number;
  steps: { seq: number; command: string; status: string; detail: string }[];
  all_reproduced: boolean;
}

export async function runCommand(source: string): Promise<ExecOutput> {
  const json = await invoke<string>("run_command", { source });
  return JSON.parse(json) as ExecOutput;
}

export async function importCsv(path: string): Promise<ExecOutput> {
  const json = await invoke<string>("import_csv", { path });
  return JSON.parse(json) as ExecOutput;
}

export async function datasetPreview(offset: number, limit: number): Promise<DatasetPreview> {
  return invoke<DatasetPreview>("dataset_preview", { offset, limit });
}

export async function variableProfile(name: string): Promise<{ stats: SummaryStats; frequencies: FrequencyRow[] }> {
  const json = await invoke<string>("variable_profile", { name });
  return JSON.parse(json);
}

export async function createProject(title: string, discipline: string, directory: string): Promise<string> {
  return invoke<string>("create_project", { title, discipline, directory });
}

export async function openProject(directory: string): Promise<string> {
  return invoke<string>("open_project", { directory });
}

export async function saveDatasetToProject(name: string): Promise<string> {
  return invoke<string>("save_dataset_to_project", { name });
}

export async function replayProject(): Promise<ReproducibilityReport> {
  const json = await invoke<string>("replay_project");
  return JSON.parse(json);
}
