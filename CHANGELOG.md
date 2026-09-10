# Changelog

All notable changes to Numeris are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [1.0.0-dev] - 2025-01-01

First development milestone of the Numeris V1 desktop statistical research
environment. Numeris is local-first, deterministic and reproducible by
design: no AI, no subscription, no cloud requirement, no usage meter.

### Added

- **Rust statistical engine** (pure Rust; only `serde`/`serde_json`
  dependencies; QR decomposition, distributions and CSV parsing implemented
  in-repo):
  - Descriptive statistics — mean, median, variance, SD, quantiles,
    skewness, kurtosis
  - Hypothesis tests — one-sample, two-sample, paired and Welch t tests;
    one-way ANOVA; Pearson and Spearman correlation
  - Linear models — OLS (QR/Householder), WLS, 2SLS instrumental variables
  - Variance estimators — classical, robust HC0–HC3, one-way cluster,
    HAC (Newey–West, user-specified lags)
  - Panel estimators — fixed effects (within), between
  - Difference-in-differences
  - Limited dependent variables — logit (Newton–Raphson), probit,
    Poisson (IRLS)
  - Diagnostics — VIF, Breusch–Pagan, RESET, Durbin–Watson, leverage,
    Cook's distance
  - Distributions — Normal, t, F, χ² CDFs and quantiles
- **Command DSL** with a real parser and AST: source → lexer → AST →
  validation → `ModelSpecification` → execution → result; canonical command
  rendering with GUI ⇄ command equivalence. Commands implemented: `use`,
  `import`, `summarize`, `describe`, `correlate`, `ttest`, `generate`,
  `replace`, `drop`, `keep`, `rename`, `label variable`, `sort`, `regress`,
  `xtreg`, `ivregress`, `did`, `logit`, `probit`, `poisson`,
  `estimate list`, `estimate compare`, `note`, `help`, `export table`,
  `export data` (see `docs/COMMAND_REFERENCE.md`).
- **`.numeris` project format** — inspectable directory of open-standard
  files (JSON/CSV) with `project.json`, `environment.json`, data, analysis,
  tables, figures, scripts, notes, reports and logs — plus the **research
  registry** (immutable analysis records) and the **replay engine**
  (re-run project, compare against stored references under documented
  tolerances).
- **Tauri 2 desktop shell** (`apps/desktop/src-tauri`) with React +
  TypeScript + Vite frontend, zustand state, TanStack Table data grid, and
  least-privilege capability allowlists.
- **React design system** — monochrome-first token system (light + dark),
  `Nx*` component inventory, keyboard-first interaction, error format
  (What / Why / What to do), and the full screen set for the 15 golden
  screens (Project Home, Data Editor, Variable Inspector, Regression Form,
  Regression Results, Model Comparison, Analysis Tree, Figure View, Report
  View, Command Palette, Settings, License, Error State, Empty State,
  Dark Mode) — see `docs/DESIGN_SYSTEM.md`.
- **NIST StRD Longley validation** — the ill-conditioned Longley regression
  dataset validated against certified NIST reference values.
- **Golden validation fixtures** (`validation/golden/`) — per-estimator
  expected values with method-appropriate, documented tolerances
  (~1e-12–1e-14 stable scalars, ~1e-10 well-conditioned OLS, ~1e-6
  iterative MLE).
- **CI matrix** (`.github/workflows/ci.yml`) — `cargo fmt --check`,
  `cargo clippy -D warnings`, `cargo test --all --locked` on ubuntu-latest,
  macos-latest and windows-latest, plus frontend typecheck (`tsc --noEmit`)
  and production build (`vite build`).
- **Release pipeline scaffolding** (`.github/workflows/release.yml`) —
  tauri-action builds for Windows (MSI/NSIS x64), macOS (arm64 + x64 DMG)
  and Linux (AppImage + deb) into a draft GitHub release on `v*` tags, with
  documented signing/notarization prerequisites (macOS Developer ID +
  notarization, Windows code signing, Ed25519 updater keys).
- **License and dependency manifests** — `THIRD-PARTY-NOTICES.txt`,
  `DEPENDENCY-MANIFEST.json` (machine-readable, permissive-only inventory)
  and the `LICENSES/` audit trail (MIT, Apache-2.0 texts) per the
  engineering specification §5.1.
- **Documentation set** — `docs/ARCHITECTURE.md`,
  `docs/COMMAND_REFERENCE.md`, `docs/DEVELOPER_GUIDE.md`,
  `docs/DESIGN_SYSTEM.md`, `INSTALL.md`, `KNOWN_LIMITATIONS.md`,
  `BUILD_NOTES.md`, `security/LICENSING_DESIGN.md`.
- **Developer tooling** — `scripts/verify.sh` one-shot local verification
  (fmt, clippy, tests, typecheck, build).

### Notes

- The product was renamed **EconoLab → Numeris** and the project format
  `.ecolab` → `.numeris` by owner decision before the first commit; the
  authoritative specifications retain the old names semantically
  (see `BUILD_NOTES.md`).
- Unsupported methods are labeled unsupported — never faked. See
  `KNOWN_LIMITATIONS.md` for the exact scope boundary of this milestone.
