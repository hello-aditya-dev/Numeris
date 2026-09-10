# Numeris — Developer Guide

How to set up, build, test and extend Numeris. Companion documents:
`docs/ARCHITECTURE.md` (system design), `docs/COMMAND_REFERENCE.md` (DSL),
`docs/DESIGN_SYSTEM.md` (UI reference), `BUILD_NOTES.md` (decision log).

---

## 1. Prerequisites

| Tool | Version | Used for |
|---|---|---|
| [Rust](https://rustup.rs) (stable) | latest stable | engine crates, Tauri shell |
| [Bun](https://bun.sh) ≥ 1.1 (or Node ≥ 20) | 1.1+ | frontend install/build; CI uses bun |
| Tauri 2 platform dependencies | — | WebView2 (Windows, auto-installed), WKWebView (macOS, built in), webkit2gtk 4.1 + libayatana-appindicator3 + librsvg2 (Linux) |

Linux system packages for Tauri 2 (Ubuntu/Debian):

```bash
sudo apt-get install -y build-essential curl wget file \
  libwebkit2gtk-4.1-dev libssl-dev libxdo-dev \
  libayatana-appindicator3-dev librsvg2-dev
```

## 2. Set up

```bash
git clone https://github.com/hello-aditya-dev/Numeris.git
cd Numeris

# verify the toolchains
cargo --version          # stable
bun --version            # or: node --version
```

The Rust workspace manifest is at the repository root and covers
`crates/numeris-*` and `apps/desktop/src-tauri`; plain `cargo` commands work
from the repo root.

## 3. Build, test, run

```bash
# 1. Run the full engine test suite (includes NIST StRD Longley + golden
#    fixtures in numeris-stats)
cargo test --all

# 2. Frontend: install dependencies then typecheck + build
cd apps/desktop
bun install
bunx tsc --noEmit
bunx vite build

# 3. Run the desktop app in development mode
bun run tauri dev

# 4. Build a production bundle for your platform
bun run tauri build
```

Installers land in `apps/desktop/src-tauri/target/release/bundle/`.

One-shot local verification (everything CI checks, minus the OS matrix):

```bash
scripts/verify.sh
```

## 4. Repository layout and ownership

```text
apps/desktop/src          React frontend           (frontend engineer)
apps/desktop/src-tauri    Tauri shell              (engine/desktop engineer)
crates/numeris-core       core engine              (engine engineer)
crates/numeris-stats      estimators + validation  (engine engineer)
crates/numeris-command    command DSL              (engine engineer)
crates/numeris-project    project format/replay    (engine engineer)
validation/golden/        golden + NIST fixtures   (engine engineer)
docs/                     documentation            (docs engineer)
scripts/                  tooling                  (docs engineer)
.github/workflows/        CI + release             (docs engineer)
```

The UI contains no statistical algorithms. The frontend sends specifications
across Tauri IPC and renders `ModelResult` objects.

## 5. Rust conventions

- `cargo fmt` is the only formatter; `cargo clippy --all-targets -- -D
  warnings` is the only linter — both are CI gates.
- `cargo test --all --locked` — CI builds exactly the committed `Cargo.lock`.
  When adding/updating a dependency, commit the updated lockfile in the same
  change.
- No `unsafe` in statistical code without a written justification in the PR.
- Public items carry doc comments; estimators document their assumptions and
  numerical behavior.
- Errors use the `NumerisError` model in `numeris-core` — map every failure to
  one of the outcome classes (invalid request / insufficient data /
  non-identification / numerical failure / convergence failure / result with
  warnings / valid result). Never `panic!` on user data; never stringify
  errors into the UI.
- The engine crates (`numeris-core`, `numeris-stats`) depend on
  `serde`/`serde_json` **only**. Adding any third-party numerical/statistical
  dependency requires an explicit decision in `BUILD_NOTES.md` plus a license
  audit update (`THIRD-PARTY-NOTICES.txt`, `DEPENDENCY-MANIFEST.json`).

## 6. Frontend conventions

- Package manager: bun (npm also works locally; CI uses bun).
- `bunx tsc --noEmit` must pass with zero errors (CI gate).
- Build with Vite: `bunx vite build` (CI gate).
- Use `Nx*` design-system components — never raw colors, spacing or radii
  (see §8 below and `docs/DESIGN_SYSTEM.md`).
- State via zustand; tabular data via TanStack Table.

## 7. How to add a new estimator — the seven quality gates

An estimator is not "done" because it runs once. All seven gates below must
pass before an estimator is declared production-ready; the checklist is
enforced at PR review.

**Gate 1 — Mathematical implementation.** The method is specified (with a
written reference to the mathematical definition) and independently
implemented in `crates/numeris-stats` behind the estimator contract
(`validate → fit → diagnostics → predict`). No vendored or copied numerical
code.

**Gate 2 — Numerical validation.** Golden datasets pass. Add a fixture under
`validation/golden/` (see §9) with expected coefficients, standard errors,
test statistics, intervals, warnings and convergence outcome, plus a
justified per-method tolerance. NIST StRD where applicable.

**Gate 3 — Edge cases.** Missing data, zero variance, singular matrices,
separation, empty subsets and the other known failure modes of the method
are handled explicitly as outcome classes — each with a test that proves the
error/warning is produced.

**Gate 4 — UX integration.** The estimator is reachable consistently through
both the command DSL (`numeris-command`) and the analysis forms, both
compiling to the same `ModelSpecification`; the canonical command preview is
implemented; results render through the standard `ModelResult` path.

**Gate 5 — Reproducibility.** The analysis can be re-run from the stored
specification and matches within the documented tolerance (project replay
regenerates it); the research registry records it; `estimate list` /
`estimate compare` / `export table` work with it.

**Gate 6 — Documentation.** Purpose, syntax, arguments, options, examples,
assumptions and diagnostics are documented in
`docs/COMMAND_REFERENCE.md`; numerical behavior and tolerances are recorded
with the golden fixture.

**Gate 7 — Release regression.** The full statistical test suite passes on
Windows, macOS and Linux (the CI matrix is the enforcement mechanism).

Suggested order of work: Gate 1 (with unit tests) → Gate 2 → Gate 3 →
Gate 5 → Gate 4 → Gate 6, and Gate 7 continuously via CI.

## 8. Design-system rules for developers

- **Use `Nx*` components.** Before creating any new component, answer the
  design review gate: *can an existing component solve this?* If yes, reuse
  it; if no, document why a new component is necessary.
- **Never write raw values in screens.** No `#2563eb`, no `13px`, no
  `11px radius`. Use tokens: `var(--color-accent-primary)`,
  `var(--space-md)`, `var(--radius-md)` (full token list in
  `docs/DESIGN_SYSTEM.md`).
- **No cards for everything.** Results, comparisons and summaries are dense
  tables and editorial structures, not dashboard cards.
- **Tabular numerals** for all statistical values.
- **Screen QC checklist** — run before shipping any screen:

  - *Consistency:* uses existing components? existing spacing? existing
    typography? existing semantic colors? same button hierarchy?
  - *Information:* is the user's primary task visible immediately? is
    unnecessary UI removed?
  - *Research workflow:* what dataset is active? what analysis context is
    active? can the user reproduce the operation?
  - *Accessibility:* entire workflow keyboard-operable? focus visible?
    errors understandable? is color the only signal anywhere?
  - *Platform:* does macOS behave like macOS? Windows/Linux naturally?
    shared semantics preserved?

## 9. How to add golden validation fixtures

Golden fixtures live under `validation/golden/`, organized by method
category (`ols/`, `robust/`, `cluster/`, `panel/`, `iv/`, `did/`, `logit/`,
`probit/`, `count/`, …). A fixture consists of:

```text
validation/golden/<category>/<name>/
├── input.csv            # the input dataset
└── expected.json        # expected values + tolerance
```

`expected.json` must define, per the validation policy:

- input reference (and dataset version)
- expected coefficients / results
- expected standard errors
- expected test statistics
- expected intervals
- expected warnings
- expected convergence outcome
- **numerical tolerance** — chosen per the estimator's test class:

| Test class | Starting tolerance |
|---|---|
| Stable closed-form scalar statistics | ~1e-12 to 1e-14 where justified |
| Well-conditioned OLS coefficients | ~1e-10 |
| Ill-conditioned problems (e.g. NIST Longley) | method-specific, documented |
| Iterative MLE models | ~1e-6 relative/absolute where justified |

Rules:

1. Expected values are produced by a trusted reference implementation (R /
   Python) or derived mathematically, then hand-reviewed. Reference runs are
   validation inputs only — never runtime dependencies.
2. **Never "fix" a failing fixture by regenerating expected values to match
   Numeris output.** A fixture failure means either an engine bug or a
   tolerance that needs mathematical justification — both require review.
3. Register the fixture in the estimator's test module so `cargo test`
   picks it up.
4. Record the tolerance rationale (conditioning, method class) as a comment
   in `expected.json`.

The NIST StRD Longley fixture (`validation/golden/`) is the model example:
certified NIST reference values, deliberately ill-conditioned data, and a
documented looser tolerance than the well-conditioned OLS fixtures.

## 10. Code style

- Rust: rustfmt formatting; clippy with `-D warnings`; meaningful names over
  comments; comments explain *why*, code shows *how*.
- TypeScript: strict mode; no `any` in new code; `Nx` prefix for components;
  feature files mirror screen names.
- Documentation is independently written — never copy another vendor's
  documentation, wording, screenshots, icons or proprietary examples.

## 11. Commit conventions

Conventional Commits, imperative mood, one logical change per commit:

```text
feat(stats): add HC3 variance estimator for OLS
fix(command): reject vce(hac N) without explicit lag count
docs(arch): document replay sequence and tolerances
ci: add windows-latest to the test matrix
test(stats): add separation edge cases for logit
refactor(core): extract quantile selection helpers
chore(deps): bump vite to 6.0.5 (lockfile updated)
```

- Scope: `core`, `stats`, `command`, `project`, `desktop`, `ui`, `docs`,
  `ci`, `deps`, `chore`.
- Breaking changes: add a `BREAKING CHANGE:` footer.
- **Never commit secrets or tokens** (including the owner's GitHub token).
- Never commit generated build output (`target/`, `dist/`, `node_modules/`).

## 12. CHANGELOG / BUILD_NOTES discipline

- `CHANGELOG.md` — user-facing changes, Keep a Changelog format
  (`Added` / `Changed` / `Fixed` / `Removed` / `Security`). Every PR that
  changes observable behavior updates it in the same PR.
- `BUILD_NOTES.md` — engineering decision log. Every deviation from the
  authoritative specifications, every dependency decision, every rename, and
  every documented trade-off gets a numbered entry (see `BUILD_NOTES.md` for
  the format).
- Update both documents in the same change as the code they describe —
  retroactive documentation drifts.

## 13. CI and release

- Every PR runs `cargo fmt --check`, `cargo clippy -D warnings`,
  `cargo test --all --locked` on ubuntu/macos/windows, plus the frontend
  typecheck and build (`.github/workflows/ci.yml`).
- Release artifacts are built by pushing a `v*` tag; the workflow creates a
  **draft** GitHub release with per-platform installers
  (`.github/workflows/release.yml`). Publishing requires signing secrets to
  be configured — see the comments in that file.
- Statistical correctness is a release blocker: a failing golden test fails
  the release, full stop.
