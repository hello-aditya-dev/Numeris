# BUILD_NOTES — Numeris Engineering Decision Log

This is the record of substantial engineering decisions and deviations,
per the build kit's CHANGE_CONTROL rule: *record substantial deviations in
BUILD_NOTES.md*. Every entry notes what was decided, why, and which
specification section it relates to. The authoritative specifications name
the product "EconoLab"; the product is **Numeris** (see D-001). Unless a
decision below says otherwise, the specifications apply as written.

Entry format: `D-### — date — title`.

---

## D-001 — 2025-01-01 — Rename EconoLab → Numeris

**Decision.** The product is named **Numeris** throughout the repository,
application, documentation, CI, manifests and UI. The authoritative
specifications (EconoLab V1 Master Engineering Specification, EconoLab V1
Design System) keep their original names but are applied with a consistent
naming map.

**Why.** Owner decision before implementation began. The rename is a naming
change, not a semantic one — nothing else about the specification changes.

**Mapping (applies everywhere):**

| Spec name | Numeris name |
|---|---|
| EconoLab | Numeris |
| `.ecolab` project format | `.numeris` |
| crates `ecolab-*` | crates `numeris-*` |
| UI components `Ec*` | UI components `Nx*` |
| Numeris Software License (was: the EconoLab license) | `LICENSE` at repo root |

**Spec references.** Whole-document rename; no section semantics altered.

## D-002 — 2025-01-01 — Project format extension `.ecolab` → `.numeris`

**Decision.** The project directory format specified in §13 is implemented
as `project.numeris/` with the exact internal structure specified
(`project.json`, `data/`, `analysis/`, `outputs/`, `tables/`, `figures/`,
`scripts/`, `notes/`, `reports/`, `logs/`, `environment.json`).

**Why.** Follows D-001. The format remains inspectable open-standard files;
no opaque single-file database is used for the research record (§13, §46
rule 10).

## D-003 — 2025-01-01 — Pure-Rust, zero-dependency numerical core

**Decision.** The statistical engine (numeris-core, numeris-stats)
implements its own QR decomposition (Householder), statistical distribution
functions (Normal, t, F, χ² CDFs/quantiles) and CSV parser (RFC 4180) in
this repository. The engine crates' only third-party dependencies are
`serde` and `serde_json`.

**Why.** The specification (§5.1) chose "faer / carefully reviewed
numerical crates", Polars and DuckDB. Deferring those: (a) auditability —
every numerical line that produces a coefficient is reviewable in-repo,
which is the product's core trust claim; (b) license hygiene — the
proprietary core cannot accidentally acquire a copyleft or
commercially-ambiguous obligation through a transitive dependency graph
(§5.1 license rule); (c) the V1 method inventory (OLS/WLS/IV/panel/GLM) is
fully covered by an in-repo QR-based implementation, validated against
NIST StRD; (d) a smaller dependency surface makes the Numeris-specific
numerical-correctness policy (§22, per-estimator tolerances) directly
enforceable.

**Deviation status.** Documented deviation from §5.1's chosen-stack table.
The spec's "carefully reviewed" escape hatch exists precisely for this
class of decision; per CHANGE_CONTROL the deviation is recorded here.
faer/Polars/DuckDB are **deferred until integration review** — if
performance or format needs outgrow the in-repo implementations in V1.1+,
each adoption gets its own decision entry plus the §5.1 license audit
(THIRD-PARTY-NOTICES.txt / DEPENDENCY-MANIFEST.json updates in the same
change).

## D-004 — 2025-01-01 — Interactive web preview harness (Next.js) for the build sandbox

**Decision.** An interactive Next.js preview exists at `src/app/page.tsx` of
the build sandbox project so the owner can exercise the Numeris UI —
screens, command console, estimation flows — in a browser Preview Panel
before the desktop bundle is produced.

**What this is.** A **demonstration harness only, not a SaaS product.**
The desktop application remains the product; the harness exists because the
build environment cannot run Tauri/WebView natively, and the owner's
review loop needs to reach the UI before bundling.

**What this is not.** It does not change the product constitution: Numeris
is not SaaS (§2), the harness is not deployed, and the desktop app remains
the only product execution path.

## D-005 — 2025-01-01 — Tauri icons generated programmatically

**Decision.** The Tauri application icons are generated programmatically
(scripted from the Numeris mark), not hand-drawn asset imports.

**Why.** Reproducibility and license hygiene: no third-party icon assets
enter the repository, and icon regeneration is deterministic. §38 forbids
copying proprietary icons/assets; generating them removes the question
entirely.

## D-006 — 2025-01-01 — serde/serde_json as the only engine dependencies

**Decision.** Confirmed and pinned: `numeris-core` and `numeris-stats`
depend on exactly `serde` and `serde_json` (both MIT OR Apache-2.0), and
nothing else. The desktop shell adds `tauri`/`tauri-build` (MIT OR
Apache-2.0); the frontend adds React/zustand/TanStack/Vite/TypeScript
(MIT/Apache-2.0).

**Why.** §46 rule 1 (do not invent dependencies) and §5.1 (every dependency
justified, pinned, license-audited). The complete audited inventory lives in
`THIRD-PARTY-NOTICES.txt` and `DEPENDENCY-MANIFEST.json`; all licenses are
permissive; there is no copyleft dependency in the build graph. Any future
addition to the engine crates requires a new decision entry here plus
manifest updates.

## D-007 — 2025-01-01 — TypeScript preview engine mirrors the Rust engine

**Decision.** The browser preview harness (D-004) runs a TypeScript port of
the statistical engine that mirrors the Rust engine
**algorithm-for-algorithm** (same QR-based estimators, same variance
formulas, same distribution functions).

**Why.** The harness must produce the same numbers the product produces,
or the owner's review would be misleading. This is the **only sanctioned
duplication** of engine logic in the entire project: spec rule §46.3 says
"do not duplicate the statistical engine in the frontend", and it holds —
the duplication lives in the sandbox-only demonstration harness
(`src/app/page.tsx`), never in `apps/desktop/src`, which talks to the Rust
engine over Tauri IPC exclusively. When the harness is retired, the port
retires with it.

**Constraint.** Any divergence between the Rust engine and the TS mirror
found during review is a bug in the mirror (or in both) and is fixed
against the Rust engine as the source of truth, under the same golden
fixtures.

---

*New entries are appended with the next decision number and date. Update
`CHANGELOG.md` in the same change when a decision alters observable
behavior.*
