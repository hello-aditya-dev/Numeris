# Numeris — Architecture

**Product:** Numeris — local-first statistical research environment
**Status:** V1 development (`1.0.0-dev`)
**Platforms:** Windows, macOS, Linux

This document is the engineering architecture reference for Numeris. It
describes the system as implemented: layering, crate map, estimator contract,
canonical object model, command pipeline, validation strategy, numerical
correctness policy, reproducibility engine, project format, security
architecture, and performance architecture.

Design principles (product constitution, abbreviated):

- **No AI.** A statistical result is produced by deterministic code from an
  explicit model specification and a dataset — never by a language model.
- **Local-first.** Research data, projects and computation stay on the user's
  machine. Licensing and optional update checks are the only network services.
- **Deterministic and reproducible.** Same dataset version + same
  specification + same software version + same declared seed ⇒ same results
  within documented numerical tolerances.
- **Honest scope.** Unsupported methods are labeled unsupported — never faked.

---

## 1. Layered architecture

The application is a strict layered system. Each layer only talks to the layer
directly beneath it:

```text
┌──────────────────────────────────────────────────────────────────────┐
│  React + TypeScript frontend (apps/desktop/src)                       │
│  Nx* design-system components → screens → analysis forms →            │
│  command preview + result rendering.                                  │
│  CONTAINS NO STATISTICAL ALGORITHMS.                                  │
├────────────────────────── Tauri IPC ─────────────────────────────────┤
│  Rust application services (apps/desktop/src-tauri)                  │
│  Tauri 2 shell: windows, menus, dialogs, filesystem, updater.         │
│  Invokes engine crates; owns capability/permission allowlists.        │
├──────────────────────────────────────────────────────────────────────┤
│  numeris-command   lexer → parser → AST → validation →               │
│                     ModelSpecification / CommandSpec → execution       │
│  numeris-project   .numeris format, research registry, replay         │
├──────────────────────────────────────────────────────────────────────┤
│  numeris-stats     estimators + variance estimators + diagnostics     │
├──────────────────────────────────────────────────────────────────────┤
│  numeris-core      DataFrame, CSV (RFC 4180) reader, descriptives,    │
│                     distributions, error model                        │
└──────────────────────────────────────────────────────────────────────┘
```

The frontend never computes statistics. It sends specifications across Tauri
IPC and renders structured `ModelResult` objects. The GUI and the command
language compile to the same internal specification — there is no second,
hidden execution path.

## 2. Crate map and dependency-direction rules

```text
numeris-core
    ↑
numeris-stats
    ↑
numeris-command
    ↑
numeris-project
    ↑
apps/desktop/src-tauri
```

Rules:

1. Dependencies point **upward only** in that chain (a crate may only depend on
   crates below it). The workspace dependency graph must stay acyclic.
2. `numeris-core` depends on nothing but `serde`/`serde_json`.
3. `numeris-stats` depends on `numeris-core` only.
4. `numeris-command` depends on `numeris-core` + `numeris-stats`.
5. `numeris-project` depends on the three engine crates.
6. The desktop shell (`apps/desktop/src-tauri`) depends on all four.
7. The UI (React) contains no statistical algorithms. It cannot import or
   re-implement engine logic; the TypeScript preview engine used in the browser
   harness is a documented, sandbox-only exception (see `BUILD_NOTES.md`).

| Crate | Responsibility |
|---|---|
| `numeris-core` | `DataFrame` and column model, RFC 4180 CSV reader, descriptive statistics (mean, median, variance, SD, quantiles, skewness, kurtosis), distributions (Normal / t / F / chi-square CDFs and quantiles), the `NumerisError` error model |
| `numeris-stats` | Estimators (OLS via QR, WLS, 2SLS/IV, panel within/between, DID, logit, probit, Poisson), variance estimators (classical, HC0–HC3, one-way cluster, HAC Newey–West), diagnostics (VIF, Breusch–Pagan, RESET, Durbin–Watson, leverage, Cook's D), hypothesis tests (t-tests incl. Welch, one-way ANOVA, Pearson/Spearman), golden/NIST validation suite |
| `numeris-command` | Command DSL: lexer → parser → AST → validation → executor; canonical command rendering (GUI ⇄ command equivalence) |
| `numeris-project` | `.numeris` project directory format, research registry, environment manifest, replay engine |

The engine crates depend on **`serde` + `serde_json` only**. All numerical
routines — QR decomposition, distribution functions, the CSV parser — are
independent in-repo implementations (rationale: `BUILD_NOTES.md`, decision
D-003).

## 3. The estimator contract

Every estimator in `numeris-stats` implements one contract:

```rust
pub trait Estimator {
    type Spec;    // e.g. RegressionSpec, PanelSpec, IvSpec, GlmSpec …
    type Result;  // e.g. RegressionResult …  (always ModelResult-compatible)

    fn validate(&self, data: &Dataset, spec: &Self::Spec)
        -> Result<ValidationReport, NumerisError>;

    fn fit(&self, data: &Dataset, spec: &Self::Spec)
        -> Result<Self::Result, NumerisError>;

    fn diagnostics(&self, result: &Self::Result) -> Vec<Diagnostic>;

    fn predict(&self, result: &Self::Result, new_data: &Dataset)
        -> Result<PredictionResult, NumerisError>;
}
```

The lifecycle is always **validate → fit → diagnostics → predict**. Validation
never silently fixes a specification; estimation never runs on an unvalidated
specification; diagnostics are computed from the fitted result, never
re-estimated; prediction is a pure function of the result and new data.

Every estimator must distinguish — and every call site must handle — these
outcome classes:

| Outcome class | Meaning |
|---|---|
| Invalid request | The specification is malformed (unknown variable, wrong types, contradictory options) |
| Insufficient data | Fewer usable observations than parameters, empty subsets, degenerate groups |
| Non-identification | Rank failure, collinear/omitted regressors, underidentified IV |
| Numerical failure | Decomposition failure, non-finite intermediate values |
| Convergence failure | Iterative (IRLS/Newton) estimator did not converge within limits |
| Result with warnings | Valid estimate + structural warnings (separation flagged, overdispersion, singleton clusters …) |
| Valid result | Clean estimate |

Warnings are surfaced in the result, never hidden inside a generic success.

## 4. Canonical object model

Everything important is a typed object. Core types: `Project`, `Dataset`,
`DatasetVersion`, `Variable`, `Transformation`, `ModelSpecification`,
`ModelResult`, `Diagnostic`, `PredictionResult`, `Figure`, `Table`, `Report`,
`ResearchNote`, `AnalysisRecord`, `ExecutionRecord`, `EnvironmentManifest`.

### 4.1 ModelSpecification

The single source of truth for an estimation. Both the GUI form and the
command parser produce this object; nothing else estimates anything.

```json
{
  "id": "model-001",
  "outcome": "wage",
  "predictors": ["education", "experience", "female"],
  "estimator": "ols",
  "variance_estimator": "robust",
  "fixed_effects": [],
  "clusters": [],
  "weights": null,
  "subset": null,
  "options": {},
  "seed": null
}
```

Field notes:

- `estimator` — `ols` | `wls` | `2sls` | `xtreg_fe` | `xtreg_be` | `did` |
  `logit` | `probit` | `poisson`
- `variance_estimator` — `classical` | `robust` (HC1) | `hc2` | `hc3` |
  `cluster` | `hac` (with lag count in `options`)
- `fixed_effects` — panel/DID entity (and time) identifiers to absorb
- `clusters` — one-way cluster variable(s)
- `weights` — WLS weight variable when present
- `subset` — row-selection expression (replayed deterministically)
- `options` — estimator-specific options (e.g. `noconstant`, `hac_lags`)
- `seed` — declared random seed where applicable (V1 estimators are
  deterministic; the field exists so future seeded methods record it)

### 4.2 ModelResult

```json
{
  "coefficients": {},
  "standard_errors": {},
  "confidence_intervals": {},
  "p_values": {},
  "test_statistics": {},
  "n_observations": 0,
  "degrees_of_freedom": {},
  "fit_statistics": {},
  "covariance_info": {},
  "convergence": {},
  "diagnostics": [],
  "metadata": {}
}
```

- `coefficients`, `standard_errors`, `confidence_intervals`, `p_values`,
  `test_statistics` — maps keyed by term name (plus intercept)
- `n_observations` — rows used after listwise deletion, subsetting
- `degrees_of_freedom` — model/residual (and rank info where relevant)
- `fit_statistics` — R², adjusted R², F, log-likelihood, pseudo-R² as
  applicable per estimator; never invented for estimators where they are not
  defined
- `covariance_info` — which variance estimator was used, cluster count,
  HAC lags, singleton count
- `convergence` — iterative estimators: iterations, converged flag
- `diagnostics` — computed `Diagnostic` objects (§3)
- `metadata` — dataset id/version, specification id, software version,
  timings

The same `ModelResult` powers the Results UI, model comparison, coefficient
tables, publication tables, exports and replication packages. No screen
recomputes or hand-copies statistics.

## 5. Command pipeline

Commands are an independently implemented DSL (familiar conventions, no
vendor compatibility claims). The pipeline is a real parser with an AST:

```text
source text
    → lexer                (tokens: identifiers, numbers, strings,
                            operators, punctuation)
    → parser               (AST — one node type per command family)
    → validation           (variables exist, types, option combinations)
    → ModelSpecification / CommandSpec
    → execution            (dispatch to engine estimators / data ops)
    → Result               (ModelResult / dataset mutation / export)
```

Guarantees:

- **GUI ⇄ command equivalence.** The analysis form
  `Outcome: wage; Predictors: education, experience, female; SE: Robust`
  compiles to the *same* internal specification as
  `regress wage education experience female, robust`. The GUI always shows
  the canonical command; editing the command updates the form.
- Every executed command is recorded verbatim in the research registry with
  its compiled specification, so projects replay exactly.
- Parse errors carry position information and are rendered in the
  What/Why/What-to-do error format (see `docs/DESIGN_SYSTEM.md`).

See `docs/COMMAND_REFERENCE.md` for the full command surface.

## 6. Validation strategy

Statistical correctness is the primary technical moat. Validation is layered:

1. **NIST StRD (Longley).** The Longley regression dataset — notoriously
   ill-conditioned — is validated against the certified NIST reference values
   for coefficients and standard errors. This exercises the QR solver where
   naive normal-equation implementations lose precision.
2. **Golden datasets.** `validation/golden/` holds datasets with expected
   values for every estimator, produced by trusted reference implementations
   and reviewed by hand. Each fixture defines: input, expected
   coefficients/results, expected SEs, expected test statistics, expected
   intervals, expected warnings, expected convergence outcome, and the
   numerical tolerance.
3. **Reference anchors.** Distribution functions (Normal, t, F, chi-square
   CDFs/quantiles) are pinned against published table values.
4. **Property tests.** Invariance, symmetry and algebraic identities (e.g.
   within-estimator equals demeaned OLS; 2SLS reduces to OLS when the
   instrument set contains the regressor).
5. **Unit + edge-case + failure-mode tests.** Missing data, zero variance,
   singular matrices, separation, empty subsets.
6. **Output schema tests.** Result objects serialize to the documented shape.

**Important limitation:** NIST StRD does not validate the entirety of Numeris.
It covers specific statistical areas. It is one component of the validation
system, not the whole system. Reference implementations (R/Python) are used
only as independent validation references — never as hidden runtime
dependencies.

An estimator is not production-ready until all seven quality gates pass
(`docs/DEVELOPER_GUIDE.md` §7): mathematical implementation, numerical
validation, edge cases, UX integration, reproducibility, documentation,
release regression.

## 7. Numerical correctness policy

There is **no single universal tolerance**. Tolerances are documented by
estimator and test class:

| Test class | Starting target |
|---|---|
| Stable closed-form scalar statistics | ~1e-12 to 1e-14 where justified |
| Well-conditioned OLS coefficients | ~1e-10 |
| Ill-conditioned numerical problems | method-specific; document condition sensitivity |
| Iterative MLE models | ~1e-6 relative/absolute where justified |
| Bayesian fixed-seed summaries | method-specific; sampling-aware |
| Bootstrap | deterministic only when resampling seed and algorithm are fixed |

These are **engineering starting points**, not universal mathematical
guarantees. Each estimator documents its own numerical behavior next to its
golden fixture (Longley, for example, is explicitly ill-conditioned and its
fixture carries a looser, documented tolerance than the well-conditioned OLS
fixtures). Numerical correctness is a release blocker: a UI that "works" does
not compensate for a failing golden test.

## 8. Reproducibility engine

Every project supports **Re-run project**. The replay sequence:

1. Load project metadata (`project.json`).
2. Load dataset versions (content-addressed into `data/`).
3. Replay transformations (`generate`, `replace`, `drop`, …) in recorded
   order against the recorded dataset version.
4. Reconstruct model specifications from the research registry.
5. Execute analyses (estimators run deterministically; seeds recorded where
   applicable).
6. Rebuild tables and figures.
7. Compare results to stored references under documented tolerances (§7).
8. Produce a reproducibility report (match / mismatch per analysis, with
   tolerances).

The environment manifest (`environment.json`) records:

- Numeris version and build identifier
- dependency manifest (see `DEPENDENCY-MANIFEST.json`)
- platform and architecture
- relevant computation settings
- random seeds where applicable

Analysis records are immutable-by-history: changing a model creates a new
specification; history is never silently overwritten.

## 9. Project format — `.numeris`

A project is a directory, not an opaque database:

```text
myresearch.numeris/
├── project.json      # project id, title, discipline, creator, software
│                     # version, preferences, dataset/analysis/report registries
├── environment.json  # environment manifest (§8)
├── data/             # imported datasets + versioned copies
├── analysis/         # analysis records (spec + result per record)
├── outputs/          # exported outputs
├── tables/           # publication tables
├── figures/          # exported figures
├── scripts/          # saved command scripts
├── notes/            # research notes (Markdown)
├── reports/          # generated reports
└── logs/             # replay/execution logs
```

`project.json` records: project ID, title, discipline, creator, software
version, project preferences, dataset registry, analysis registry, report
registry. All files are inspectable open-standard formats (JSON, CSV, …);
no opaque single-file database is required for the research record.

## 10. Security architecture

Defaults (spec §23, §37):

- **No dataset upload. No telemetry. No background analytics. No advertising
  tracking.** Network activity is limited to license activation and optional
  update checks.
- **Least-privilege Tauri capabilities.** `apps/desktop/src-tauri/capabilities/
  default.json` allowlists exactly the IPC commands and permissions the shell
  needs. The UI is never granted unrestricted native capabilities when
  narrower permissions suffice.
- **Allowlisted filesystem scopes.** File dialogs and dataset reads are scoped
  to user-selected paths plus the active `.numeris` project directory.
- **Signed licenses, public-key-only verification.** License leases are signed
  with Ed25519. The application ships **only the public verification key**;
  the private signing key never ships and is never committed. Local license
  material is stored encrypted (OS keychain where available). Full design:
  `security/LICENSING_DESIGN.md`.
- **Signed releases and signed update artifacts.** The updater verifies
  signatures before installing; the private update key lives only in CI
  secrets (`.github/workflows/release.yml` documents the required secrets).
- **Protected CI secrets; no hard-coded private keys** anywhere in the repo.
- **No research data transmission by default.** The license server never
  receives datasets or statistical output (spec §30).

## 11. Performance architecture

Principles:

- lazy data access — preview is not the same as loading the entire dataset
- virtualized grids for data viewing/editing
- streaming where supported
- columnar operations (compute over columns, not per-row DOM walks)
- parallel computation only where numerically safe
- caching of unchanged results
- avoid duplicate materialization
- incremental rendering

Engineering targets (benchmark goals, not user-facing guarantees):

| Operation | Target |
|---|---|
| Startup | < 2 s on modern hardware where feasible |
| 10k × 50 preview | near-instant |
| Filtering | interactive |
| Common descriptive statistics | < 1 s on typical datasets |
| Ordinary OLS | sub-second |
| Typical project load | < 2 s |

## 12. Where things live

```text
apps/desktop/src          React frontend (Nx* design system, screens)
apps/desktop/src-tauri    Tauri 2 shell (main.rs, lib.rs, tauri.conf.json,
                           capabilities/default.json)
crates/numeris-core        core engine
crates/numeris-stats       estimators, vcov, diagnostics, validation suite
crates/numeris-command     command DSL
crates/numeris-project    project format, registry, replay
validation/golden/         golden + NIST StRD fixtures
docs/                     this documentation set
security/                 licensing/security designs
scripts/                  verify.sh and other tooling
.github/workflows/        CI (ci.yml) and release (release.yml)
```

Related documents: `docs/COMMAND_REFERENCE.md` (command DSL),
`docs/DEVELOPER_GUIDE.md` (contributing), `docs/DESIGN_SYSTEM.md` (UI
reference), `security/LICENSING_DESIGN.md` (licensing),
`KNOWN_LIMITATIONS.md` (honest scope), `BUILD_NOTES.md` (engineering decision
log).
