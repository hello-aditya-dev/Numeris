<div align="center">

# Numeris

**Professional statistical software. Built locally. Computed deterministically. Reproducible by design.**

Cross-platform desktop statistical research environment for
Economics · Political Science · Sociology · Public Policy · Finance ·
Education · Psychology · Epidemiology · Business Research · General Statistics

Windows · macOS · Linux

</div>

---

> **No AI. No subscription. No cloud requirement. No usage meter. No advertising.**
> Your data stays on your machine.

## What Numeris is

Numeris is a local-first statistical research environment. It combines data
management, deterministic statistical computation, econometrics, causal
inference, visualization, publication reporting and reproducibility in one
professional desktop application.

A statistical result in Numeris is produced by deterministic code from an
explicit model specification and a dataset — never by a language model. The
same dataset version + same specification + same software version + same
declared random seed yields reproducible results within documented numerical
tolerances.

The experience target is a serious research instrument:

**Apple software + VS Code + Bloomberg + spreadsheet + statistical laboratory.**

## Core philosophy

| Principle | Meaning |
|---|---|
| Local-first | Research data, projects and computation stay on your machine |
| Deterministic | Every estimator is deterministic, documented, tested code |
| Reproducible | GUI actions and commands record to the same specification; projects replay |
| One engine | Ten disciplines use one shared statistical engine with discipline-aware workflows |
| No AI | No LLM, no chatbot, no AI-generated coefficients, p-values or conclusions |
| No subscription | One-time license for the 1.x series; free edition remains |
| Honest scope | Unsupported methods are labeled unsupported — never faked |

## The three pillars

### 1. Data Workbench

- CSV / JSON / Parquet import-export pipeline with schema detection and validation
- Virtualized data editor with keyboard navigation, sorting, filtering
- Variable inspector: type, label, missingness, moments, quantiles, frequencies
- Data Quality Lab: missingness, duplicates, constants, outliers, structure checks
- Deterministic transformations (`generate`, `replace`, `recode`, `standardize`,
  `winsorize`, `log`, `lag` …), every step recorded and replayable

### 2. Econometric & Statistical Core

Implemented in Rust, validated numerically:

- Descriptive statistics — mean, median, variance, SD, quantiles, skewness, kurtosis
- Hypothesis tests — one-sample / two-sample / paired t (incl. Welch), one-way ANOVA,
  Pearson & Spearman correlation tests
- Linear models — OLS (QR-based), WLS, IV/2SLS
- Variance estimators — classical, HC0–HC3 robust, one-way cluster, HAC (Newey–West)
- Panel — pooled OLS, fixed effects (within), between estimator
- Causal toolkit — DID with parallel-trends diagnostics, event-study structure
- Limited dependent variables — logit (Newton–Raphson), probit, Poisson (IRLS)
- Diagnostics — VIF, Breusch–Pagan, RESET, Durbin–Watson, leverage, Cook's distance
- Distributions — Normal, t, F, χ² CDF/PDF/quantiles (validated against reference tables)

Every estimator follows one contract: input validation → deterministic fit →
structured result object → diagnostics → documented assumptions → tests.

### 3. Reproducible Research

- `.numeris` project directory — inspectable, open-standard files (JSON + CSV + Parquet)
- Research registry — immutable analysis records with dataset version, command,
  specification, timestamp, software version
- Model comparison across specifications
- Publication tables (Markdown / CSV / LaTeX / HTML)
- Research notes (Markdown)
- Project replay — re-run every recorded command and verify results against stored
  references under documented tolerances
- Replication package export (`replication/` directory: data, scripts, outputs,
  metadata, environment)

## GUI ⇄ command equivalence

The GUI and the command language compile to the same internal `ModelSpecification`.
Configuring this in the Regression form:

```
Outcome: wage
Predictors: education, experience, female
SE: Robust
```

is exactly equivalent to executing:

```
regress wage education experience female, robust
```

There is no second, hidden execution path. Every GUI action renders its canonical
command; every command maps to the same AST.

### Command language

```text
use "wage_survey.csv"
summarize wage education experience
correlate wage education experience
ttest wage, by(female)
generate logwage = ln(wage)
regress wage education experience female
regress wage education experience female, robust
regress wage education experience female, cluster(firm)
xtreg wage education experience, fe entity(firm)
ivregress 2sls wage (education = distance), robust
did wage, treat(treated) time(post) entity(firm) cluster(firm)
logit employed education experience female
probit employed education experience
poisson visits education insurance
```

## Getting started

### Prerequisites

- [Rust](https://rustup.rs) (stable toolchain) — the statistical engine and desktop shell
- [Node.js](https://nodejs.org) ≥ 20 or [Bun](https://bun.sh) ≥ 1.1 — the frontend toolchain
- Platform dependencies for [Tauri 2](https://tauri.app): WebView2 (Windows),
  WKWebView (macOS), webkit2gtk 4.1 (Linux)

### Build and run from source

```bash
git clone https://github.com/hello-aditya-dev/Numeris.git
cd Numeris

# 1. Run the statistical engine test suite (includes NIST StRD validation)
cargo test --all

# 2. Install frontend dependencies
cd apps/desktop
bun install        # or: npm install

# 3. Run the desktop app in development mode
bun run tauri dev  # or: npm run tauri dev

# 4. Build a production bundle for your platform
bun run tauri build
```

Installers land in `apps/desktop/src-tauri/target/release/bundle/`:
Windows (MSI/NSIS), macOS (DMG/app), Linux (AppImage/deb).

### Verify the engine

```bash
cargo test -p numeris-stats golden   # numerical validation suite
cargo test -p numeris-command         # DSL/parser equivalence tests
```

## Repository layout

```text
apps/desktop/          Tauri 2 desktop application (Rust shell + React frontend)
crates/numeris-core/   DataFrame, CSV reader, descriptives, distributions, error model
crates/numeris-stats/  Estimators, variance estimators, tests, diagnostics
crates/numeris-command/ Command DSL: lexer → parser → AST → executor
crates/numeris-project/ .numeris project format, research registry, project replay
validation/golden/    Golden datasets + expected values (incl. NIST StRD Longley)
docs/                 Architecture, command reference, developer guide
scripts/              Verification scripts
.github/workflows/    CI (fmt + clippy + test matrix) and release pipelines
```

## Architecture

```text
┌──────────────────────────────────────────────────────────────┐
│ Tauri 2 desktop shell (windows, menus, dialogs, updater)      │
├──────────────────────────────────────────────────────────────┤
│ React + TypeScript frontend                                   │
│  design system → screens → command preview                    │
│  (no statistical algorithms in the UI)                         │
├──────────────────────── Tauri IPC ────────────────────────────┤
│ Rust application services                                     │
│  numeris-command  lexer/parser → AST → ModelSpecification     │
│  numeris-project  .numeris format, registry, replay           │
│  numeris-stats    OLS/WLS/IV/panel/logit/probit/Poisson       │
│                   vcov: classical, HC0–HC3, cluster, HAC      │
│  numeris-core     DataFrame, CSV, descriptives, distributions │
└──────────────────────────────────────────────────────────────┘
```

The UI never contains statistical algorithms. It sends specifications to the
Rust engine and renders structured result objects.

## Numerical validation

- **NIST StRD** — the Longley regression dataset (notoriously ill-conditioned) is
  validated against certified NIST reference values.
- **Golden datasets** — `validation/golden/` holds datasets with expected
  coefficients, standard errors and statistics, each with method-appropriate
  tolerances (stable closed-form scalar statistics ~1e-12; well-conditioned OLS
  ~1e-10; iterative MLE ~1e-6).
- **Reference anchors** — distribution functions are pinned against published
  table values (t, F, χ², Normal quantiles).
- **Property tests** — invariance, symmetry and algebraic identities.

See `docs/ARCHITECTURE.md` § Validation for the full strategy.

## Keyboard-first

| Shortcut | Action |
|---|---|
| `Ctrl/Cmd + K` | Command palette |
| `Ctrl/Cmd + P` | Search project |
| `Ctrl/Cmd + S` | Save |
| `Ctrl/Cmd + Enter` | Run |
| `Ctrl/Cmd + /` | Comment (script editor / console) |
| `Ctrl/Cmd + Z` / `Shift + Z` | Undo / Redo |
| `Ctrl/Cmd + F` | Find |
| `Esc` | Close / cancel |

## Licensing

- **First 1,000 verified users** — full V1, free (verified email entitlement).
- **After** — Free edition + **$29 one-time** Full license for the 1.x series.
- One active device per license/email, with legitimate device transfer.
- Core statistical work runs fully offline after activation. No hostile DRM.
- See `LICENSE` for the full source-available license text.

## Privacy

No dataset upload. No telemetry by default. No background data collection.
Network activity is limited to license activation and optional update checks.
Statistical computation runs entirely on your machine.

## Contributing & support

- Bug reports and feature requests: [GitHub Issues](https://github.com/hello-aditya-dev/Numeris/issues)
- Questions and ideas: [GitHub Discussions](https://github.com/hello-aditya-dev/Numeris/discussions)
- Bug reports never attach research data automatically.

## Status

V1 is under active development. See `CHANGELOG.md`, `KNOWN_LIMITATIONS.md` and
`BUILD_NOTES.md` for exactly what is implemented, what is validated, and what
remains. Unsupported methods are labeled unsupported — never faked.

---

<div align="center">

**Software doesn't need AI to be intelligent.**

*No AI. No black box. No subscription. No cloud requirement. Excellent software.*

</div>
