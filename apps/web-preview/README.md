# Numeris web preview harness

Interactive browser build of Numeris used to exercise the full product
workflow (data workbench, command console, analysis forms, results,
comparison, graphics, tables, notes, registry) before desktop bundling.

**What this is**

- A faithful TypeScript port of the Rust engine (`lib/numeris/`), algorithm
  for algorithm: Householder QR, classical/HC0–HC3/cluster/HAC covariance,
  2SLS with first-stage diagnostics, fixed effects, DID, logit/probit/
  Poisson IRLS, t-tests, ANOVA, correlations, distribution functions.
- The complete UI, built from the same design system as the desktop app.
- A deterministic synthetic wage panel (40 firms × 2018–2022) plus
  client-side CSV import.

**What this is not**

- The product. The desktop application (`apps/desktop`) is the product;
  this harness exists so the workflow can be demonstrated in any browser
  without installing anything. It runs entirely client-side — research
  data never leaves the user's machine here either.

**Validation**

`validation/longley-check.ts` runs the NIST StRD Longley regression through
the ported engine and checks every certified coefficient and standard
error:

```bash
bun validation/longley-check.ts
```

All seven coefficients match the certified values to ~1e-14 relative
error (see `validation/golden/` at the repository root for the source
data and expected values used by the Rust engine's own test suite).

**Documented deviation** — recorded in `BUILD_NOTES.md`: the desktop
product uses the Rust engine as its single execution path; this harness
mirrors those algorithms in TypeScript so the product can run in a
browser. The Rust engine remains the source of truth and both are held
to the same NIST reference values.

**Embedding** — `app/` contains a Next.js App Router page and layout that
mount the harness; `components/NumerisApp.tsx` is self-contained apart
from the `lib/numeris` engine and `components/numeris.css`.
