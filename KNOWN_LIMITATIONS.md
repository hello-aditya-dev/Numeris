# Known Limitations

Numeris follows one rule without exception: **unsupported means
unsupported.** Nothing in this list is hidden behind a half-working
implementation or silently degraded behavior. Requesting any item below
produces an explicit error that says the method is not yet supported.

This file is the honest scope boundary for **1.0.0-dev**. It shrinks as
features pass their quality gates (`docs/DEVELOPER_GUIDE.md` §7).

---

## Console / session

- **Single working dataset per session.** The console binds one dataset at a
  time in the development preview. `use ..., clear` swaps it; `merge`,
  `append`, `join` across datasets are not implemented yet (the `.numeris`
  project format itself can register multiple datasets).

## Import / export formats

- **Import:** CSV (RFC 4180) and JSON are supported now. XLSX, DTA, SAV,
  SAS7BDAT and Parquet are planned but **stubbed** behind explicit
  "not yet supported" errors.
- **Export:** CSV and JSON are supported now. XLSX, DTA, SAV, SAS7BDAT and
  Parquet exports are planned but stubbed behind the same explicit errors.

## Estimation / inference

- **HAC bandwidth selection is not automatic.** `vce(hac N)` requires the
  user to specify the lag count N. Automatic bandwidth selection (e.g.
  Newey–West plug-in rules) is not implemented; guessing a bandwidth for
  the user would be exactly the kind of silent statistical decision
  Numeris refuses to make.
- **No two-way clustering yet.** One-way cluster-robust standard errors are
  implemented; two-way (and higher) clustering is targeted for V1.1.
- **No time-series ARIMA/VAR in this build.** Time-series models were
  declared post-V1-core; they are marked unsupported and return explicit
  errors (no partial implementations, no silent fallbacks).
- **No post-V1-core method families in this build.** The following are
  declared post-V1-core modules and are marked **unsupported in
  1.0.0-dev**, each with an explicit "not yet supported" error:
  survival models, multilevel/mixed models, survey estimation, multiple
  imputation, Bayesian methods, machine-learning workflows, psychometrics,
  and meta-analysis.
- **Panel models:** fixed effects (within) and between estimators only.
  Random effects, first differences and two-way fixed effects are not
  implemented in this build.

## Licensing / activation

- **Activation service is a local stub.** In the development preview,
  activation runs in offline development mode and does not contact a
  server; server-backed verification, payment processing and device
  management are pending (design: `security/LICENSING_DESIGN.md`).
- **macOS notarization and Windows code signing are not configured yet**
  — signing secrets must be set up before any public release (see the
  checklist in `.github/workflows/release.yml`).
- **The auto-updater requires signing keys** before it can distribute
  verifiable updates. Update checks remain optional and off/on in Settings.

## User interface

- **Density mode toggle is not yet implemented.** The design system
  specifies Comfortable/Compact table density (and preserves accessibility
  minimums in compact mode); the setting is not yet exposed in the UI.
