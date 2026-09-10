# Numeris — Command Reference

The Numeris command language is an independently implemented DSL. It follows
familiar statistical conventions, but it is not a compatibility layer for any
other product; the syntax and semantics below are the complete, authoritative
definition.

Conventions used in this reference:

- `varlist` — one or more variable names separated by spaces
- `varname` — a single variable name
- `#` — a number
- `[ ]` — optional element
- `( )` — literal parentheses where shown
- `cmd ⇥` blocks are examples you can paste into the console
- Command keywords are matched case-insensitively; variable names are
  case-sensitive

Commands are executed from the Console or a saved script. Every executed
command is recorded in the research registry and replayed by project re-run
(see `docs/ARCHITECTURE.md` §8).

**Index:** use · import · summarize · describe · correlate · ttest · generate ·
replace · drop · keep · rename · label variable · sort · regress · xtreg ·
ivregress · did · logit · probit · poisson · estimate list · estimate compare ·
note · help · export table · export data

---

## use

### Purpose
Load a dataset from disk into the active session as the working dataset.

### Syntax
```
use "filename" [, clear]
```

### Arguments
- `"filename"` — quoted path. Relative paths resolve against the active
  project's `data/` directory; absolute paths are honored.

### Options
| Option | Description |
|---|---|
| `clear` | Discard the current working dataset before loading. Without it, `use` refuses to overwrite a modified dataset in memory. |

### Examples
```
use "wage_survey.csv"
use "panel_data.json", clear
use "/home/research/replication/data/cps.csv"
```

### Output
Confirmation listing the number of observations and variables, and the
detected format.

### Assumptions
The file must exist and be readable. V1 supports **CSV** (RFC 4180, including
quoted fields and embedded delimiters/newlines) and **JSON**. Other formats
(XLSX, DTA, SAV, SAS7BDAT, Parquet) are not yet supported and fail with an
explicit "not yet supported" error — see `KNOWN_LIMITATIONS.md`.

### Related commands
import, describe, summarize, export data

---

## import

### Purpose
Alias of `use`. Provided because both verbs are natural to users; the two
commands compile to the same AST node.

### Syntax
```
import "filename" [, clear]
```

See **use** for arguments, options, examples and assumptions.

### Related commands
use, export data

---

## summarize

### Purpose
Compute descriptive statistics for the listed variables.

### Syntax
```
summarize [varlist] [, detail]
```

### Arguments
- `varlist` — variables to summarize. Omitting the list summarizes every
  variable in the working dataset.

### Options
| Option | Description |
|---|---|
| `detail` | Add skewness, kurtosis and the 1st/5th/25th/50th/75th/95th/99th percentiles. |

### Examples
```
summarize wage education experience
summarize wage, detail
summarize
```

### Output
Per variable: N (non-missing), mean, standard deviation (sample, n−1),
variance, minimum, maximum; with `detail`, also skewness, kurtosis and
percentiles. Missing values are excluded listwise per variable.

### Diagnostics
Zero-variance variables are reported; constant variables are flagged for the
attention of the researcher (they do not fail the command).

### Related commands
describe, correlate, ttest

---

## describe

### Purpose
Inspect the structure of the working dataset: variables and their metadata.

### Syntax
```
describe [varlist]
```

### Arguments
- `varlist` — restrict the listing to the named variables. Omit for all.

### Examples
```
describe
describe wage education female
```

### Output
Dataset summary (observations, variables) and, per variable: name, label
(if set), storage type (numeric/string), semantic type, missing count and
percentage, unique-value count.

### Related commands
summarize, label variable, rename

---

## correlate

### Purpose
Compute a correlation matrix for the listed variables.

### Syntax
```
correlate varlist [, spearman]
```

### Arguments
- `varlist` — two or more variables.

### Options
| Option | Description |
|---|---|
| `spearman` | Spearman rank correlation (average ranks for ties) instead of Pearson product-moment. |

### Examples
```
correlate wage education experience
correlate wage education experience, spearman
```

### Output
The correlation matrix (tabular numerals, aligned) with the number of
observations used per pair.

### Assumptions
Pearson: interval-scale variables, roughly linear association. Spearman:
ordinal scale is sufficient; captures monotone association.

### Diagnostics
Pearson correlations are computed on pairwise-complete observations; when
pairwise N differs from the full-sample N the output notes the deletion.
A zero-variance variable is an error ("cannot correlate a constant
variable"), not a silent `NaN`.

### Related commands
summarize, regress

---

## ttest

### Purpose
Perform a t test on a variable's mean — one-sample, two-sample (by group), or
paired — with equal-variance or Welch degrees of freedom.

### Syntax
```
ttest varname == #                      (one-sample)

ttest varname, by(groupvar) [welch]     (two-sample)

ttest varname1 varname2                 (paired)
```

### Arguments
- `varname` — the analysis variable (must be numeric)
- `#` — the hypothesized mean (one-sample form)
- `by(groupvar)` — binary grouping variable (two-sample form)
- `varname1 varname2` — the two measures being compared (paired form);
  rows with either value missing are dropped

### Options
| Option | Description |
|---|---|
| `welch` | Two-sample test using Welch–Satterthwaite degrees of freedom (unequal variances not assumed). Without it, the classical pooled-variance two-sample t test is used. |

### Examples
```
ttest wage == 20
ttest wage, by(female)
ttest wage, by(female) welch
ttest wage_before wage_after
```

### Output
Group (or variable) means, standard deviations and N; the t statistic,
degrees of freedom, two-sided p-value and a 95% confidence interval for the
mean difference.

### Assumptions
Independence of observations. The classical t test assumes approximate
normality (or large N for the CLT to dominate); `welch` additionally does
not assume equal group variances. The paired form assumes paired
observations.

### Diagnostics
Small-sample warnings (N < 30 per group) suggest checking normality; groups
with severely unbalanced N are noted when Welch is not used.

### Related commands
summarize, correlate, regress

---

## generate

### Purpose
Create a new variable from an expression.

### Syntax
```
generate newvar = exp
```

### Arguments
- `newvar` — the new variable name. Must not already exist; use `replace`
  to modify an existing variable.
- `exp` — an arithmetic expression over existing variables, constants,
  parentheses, and the functions below.

### Functions
| Function | Description |
|---|---|
| `ln(x)` | Natural (base-e) logarithm; nonpositive values produce missing |
| `log(x)` | Base-10 logarithm; nonpositive values produce missing |
| `exp(x)` | Exponential |
| `sqrt(x)` | Square root; negative values produce missing |
| `abs(x)` | Absolute value |
| `round(x)` | Round to the nearest integer |
| `rank(x)` | Rank within the variable, ties receive the average rank |
| `standardize(x)` | z-scores using the variable's sample mean and SD (n−1) |

Operators: `+ - * / ^` and parentheses. Missing operands propagate to
missing results.

### Examples
```
generate logwage = ln(wage)
generate wage_thousands = wage / 1000
generate z_experience = standardize(experience)
generate expscore = exp(0.05 * experience)
generate educ_rank = rank(education)
generate wage2 = round(wage)
```

### Output
Confirmation with the number of non-missing values generated.

### Assumptions
`generate` is recorded as a replayable transformation: re-running a project
re-executes it against the recorded dataset version.

### Related commands
replace, drop, keep, rename

---

## replace

### Purpose
Change the values of an existing variable.

### Syntax
```
replace varname = exp [if exp]
```

### Arguments
- `varname` — an existing variable
- `exp` — expression as in `generate`
- `if exp` — restrict the replacement to rows satisfying the condition
  (e.g. `if year == 2020`). Without `if`, all rows are replaced.

### Examples
```
replace wage = wage * 1.1
replace wage = ln(wage) if year >= 2010
replace z = standardize(x)
```

### Output
The number of real (non-missing-to-real or changed) replacements made.

### Related commands
generate, drop, keep

---

## drop

### Purpose
Remove variables or observations from the working dataset.

### Syntax
```
drop varlist            (remove variables)

drop if exp             (remove observations)
```

### Arguments
- `varlist` — variables to remove. Dropping every variable is an error.
- `if exp` — remove rows satisfying the condition.

### Examples
```
drop tempvar aux_score
drop if year < 2000
```

### Output
Dataset dimensions after the operation.

### Related commands
keep, generate, replace

---

## keep

### Purpose
Keep only the named variables, or only the observations satisfying a
condition; everything else is removed.

### Syntax
```
keep varlist            (keep variables)

keep if exp             (keep observations)
```

### Arguments
- `varlist` — variables to retain
- `if exp` — rows to retain

### Examples
```
keep wage education experience female
keep if treated == 1
```

### Output
Dataset dimensions after the operation.

### Related commands
drop

---

## rename

### Purpose
Rename a variable. Labels and data are preserved.

### Syntax
```
rename oldvar newvar
```

### Arguments
- `oldvar` — existing variable
- `newvar` — new name; must not collide with an existing variable

### Examples
```
rename wage hourly_wage
```

### Related commands
label variable, describe

---

## label variable

### Purpose
Attach a human-readable label to a variable.

### Syntax
```
label variable varname "label text"
```

### Arguments
- `varname` — the variable
- `"label text"` — the label (shown in the variable inspector, result
  headers and exports)

### Examples
```
label variable wage "Hourly wage, 2024 local currency"
label variable female "Respondent is female (1 = yes)"
```

### Output
Confirmation. A repeated `label variable` replaces the previous label.

### Related commands
describe, rename, note

---

## sort

### Purpose
Order the observations of the working dataset.

### Syntax
```
sort varlist
```

### Arguments
- `varlist` — keys, in priority order. Ascending. Rows that tie on all keys
  keep their original relative order (stable, deterministic sort), so replay
  is exact.

### Examples
```
sort firm year
sort female wage
```

### Output
Confirmation with the number of rows sorted.

### Related commands
generate, describe

---

## regress

### Purpose
Ordinary least squares (OLS) linear regression, estimated by QR
decomposition (Householder), with a choice of variance estimators.

### Syntax
```
regress depvar [indepvars] [, noconstant robust hc2 hc3
                       cluster(varname) vce(hac N)]
```

### Arguments
- `depvar` — numeric outcome variable
- `indepvars` — numeric predictor variables. An intercept is included
  unless `noconstant` is given.

### Options
| Option | Description |
|---|---|
| `robust` | Heteroskedasticity-robust standard errors (HC1 form: the HC0 sandwich with the n/(n−k) finite-sample correction — the applied-econometrics default). |
| `hc2` | HC2 robust standard errors (leverage-adjusted sandwich). |
| `hc3` | HC3 robust standard errors (recommended for small samples under heteroskedasticity). |
| `cluster(varname)` | One-way cluster-robust standard errors. The cluster variable must not contain missing identifiers among used rows. |
| `vce(hac N)` | Newey–West heteroskedasticity- and autocorrelation-consistent (HAC) standard errors with the Bartlett kernel and **exactly N lags**. N is required: automatic bandwidth selection is not implemented in V1 (`KNOWN_LIMITATIONS.md`). |
| `noconstant` | Suppress the intercept. |

Variance options are mutually exclusive (except `cluster` and `vce(hac N)`
which are alternatives to each other); classical (homoskedasticity-assuming)
standard errors are the default.

### Examples
```
regress wage education experience female
regress wage education experience female, robust
regress wage education experience female, cluster(firm)
regress wage education experience female, hc3
regress wage education experience, vce(hac 4)
regress wage education experience female, noconstant
```

### Output
Coefficient table — coefficient, standard error, t statistic, p-value and
95% confidence interval per term (tabular numerals, aligned) — plus N,
degrees of freedom, R², adjusted R² and the overall F test.

### Assumptions
OLS point estimates assume a linear conditional mean and no perfect
multicollinearity among the included regressors. Classical standard errors
add i.i.d. homoskedastic errors; `robust`/`hc2`/`hc3` relax homoskedasticity;
`cluster(varname)` relaxes independence within clusters (requires many
clusters for reliable inference); `vce(hac N)` accommodates
autocorrelation up to lag N.

### Diagnostics
Computed with the fit and shown in the Diagnostics section of the result:
VIF (multicollinearity), Breusch–Pagan (heteroskedasticity), RESET
(specification error), Durbin–Watson (first-order autocorrelation), leverage
(hat values) and Cook's distance (influential observations). Perfectly
collinear terms are omitted and reported ("omitted due to collinearity")
rather than dropped silently.

### Related commands
xtreg, ivregress, did, logit, estimate compare

---

## xtreg

### Purpose
Panel-data regression — fixed effects (within) or between estimator.

### Syntax
```
xtreg depvar indepvars, fe entity(varname) [cluster(varname)]

xtreg depvar indepvars, be entity(varname)
```

### Arguments
- `depvar`, `indepvars` — as in `regress`
- `entity(varname)` — the panel entity identifier (required). Entities with
  a single observation ("singletons") are flagged.

### Options
| Option | Description |
|---|---|
| `fe` | Fixed-effects (within) estimator: entity means are demeaned out; the intercept represents the average entity effect. |
| `be` | Between estimator: regression of entity means. |
| `cluster(varname)` | One-way cluster-robust standard errors — usually `cluster(entityvar)`, the recommended pairing with `fe`. |

### Examples
```
xtreg wage education experience, fe entity(firm)
xtreg wage education experience, fe entity(firm) cluster(firm)
xtreg wage education experience, be entity(firm)
```

### Output
Coefficient table as in `regress`, with within/between R², entity count,
observations, and — for `fe` — the share of variance due to entity effects
when defined.

### Assumptions
`fe` (within) requires strict exogeneity of regressors conditional on the
entity effect and time-invariant regressors are absorbed (and reported as
omitted). `be` uses only cross-entity variation and ignores within-entity
information. Two-way fixed effects are not implemented in V1 (declared in
`KNOWN_LIMITATIONS.md`).

### Diagnostics
Singleton-entity warning (singletons bias cluster-robust inference downward
and are reported); omitted time-invariant regressors are listed.

### Related commands
regress, did, ivregress

---

## ivregress

### Purpose
Instrumental-variables regression via two-stage least squares (2SLS).

### Syntax
```
ivregress 2sls depvar (endogvar = instrument_list) [exog_vars] [, robust]
```

### Arguments
- `depvar` — outcome
- `(endogvar = instrument_list)` — the endogenous regressor and its excluded
  instruments, in literal parentheses
- `exog_vars` — included (exogenous) control variables entering both stages

### Options
| Option | Description |
|---|---|
| `robust` | Heteroskedasticity-robust (HC1) standard errors. |

### Examples
```
ivregress 2sls wage (education = distance), robust
ivregress 2sls wage (education = distance) experience female, robust
```

### Output
Second-stage coefficient table (2SLS estimates), first-stage summary per
endogenous regressor, N and first-stage F statistic.

### Assumptions
Instruments must be relevant (correlated with the endogenous regressor after
controls) and exogenous (uncorrelated with the structural error). The rank
condition for identification must hold. 2SLS is consistent but not unbiased
under correct specification.

### Diagnostics
The first-stage F statistic is reported as a weak-instrument heuristic;
specific critical values (e.g. Stock–Yogo thresholds) are not claimed.
Underidentification and collinearity are reported as explicit errors, never
as silent zero coefficients.

### Related commands
regress, xtreg

---

## did

### Purpose
Difference-in-differences estimation of a treatment effect.

### Syntax
```
did depvar, treat(varname) time(varname) [entity(varname)]
    [cluster(varname)]
```

### Arguments
- `depvar` — outcome variable
- `treat(varname)` — treatment-group indicator (1 = treated group)
- `time(varname)` — post-period indicator (1 = after)
- `entity(varname)` — optional panel entity; when given, entity fixed
  effects are absorbed (the generalized/multi-period DID form)
- `cluster(varname)` — one-way cluster-robust standard errors, typically
  on the entity

### Examples
```
did wage, treat(treated) time(post) cluster(firm)
did wage, treat(treated) time(post) entity(firm) cluster(firm)
```

### Output
The DID coefficient (the interaction of treatment group and post-period —
the estimated average treatment effect on the treated under the design's
assumptions), its standard error, p-value and confidence interval; group
and period means; entity and period counts when `entity()` is present.

### Assumptions
The parallel-trends assumption: absent treatment, treated and control
groups would have followed parallel outcome paths. Repeated
cross-sections are supported when `entity()` is omitted. Selecting a DID
design never *establishes* causality by itself — the output always presents
the identifying assumption alongside the estimate.

### Diagnostics
When pre-treatment periods are available, a pre-trend contrast is reported
as a diagnostic of the parallel-trends assumption. Missing treatment or
time indicators are an explicit error.

### Related commands
xtreg, regress, estimate compare

---

## logit

### Purpose
Logistic regression for binary outcomes — maximum likelihood via
Newton–Raphson.

### Syntax
```
logit depvar indepvars [, robust]
```

### Arguments
- `depvar` — binary outcome coded 0/1 (any other values are an error)
- `indepvars` — predictors; an intercept is always included

### Options
| Option | Description |
|---|---|
| `robust` | Heteroskedasticity-robust (Huber/White, HC1-style) standard errors. Without it, inverse-information (model-based) standard errors are reported. |

### Examples
```
logit employed education experience female
logit employed education experience female, robust
```

### Output
Coefficient table on the log-odds scale (coefficient, SE, z, p, 95% CI),
log-likelihood, iterations to convergence, McFadden pseudo-R², N and the
0/1 counts of the outcome.

### Assumptions
Independence of observations; correct specification of the logit link;
no perfect prediction of the outcome.

### Diagnostics
Perfect separation (a linear combination of predictors fully predicts the
outcome) is detected and reported as a convergence warning with the
offending direction — never as silently huge coefficients. Non-convergence
within the iteration limit is an explicit failure.

### Related commands
probit, poisson, regress

---

## probit

### Purpose
Probit regression for binary outcomes — maximum likelihood with the standard
normal CDF link, via Newton–Raphson.

### Syntax
```
probit depvar indepvars [, robust]
```

### Arguments
As for **logit**.

### Options
| Option | Description |
|---|---|
| `robust` | Heteroskedasticity-robust (Huber/White, HC1-style) standard errors. |

### Examples
```
probit employed education experience
probit employed education experience female, robust
```

### Output
Coefficient table on the latent-variable (z) scale, log-likelihood,
iterations, McFadden pseudo-R², N, outcome counts.

### Assumptions
As for `logit`, with the normal CDF link (coefficients are in z-units;
marginal effects differ from logit even for identical fitted probabilities).

### Diagnostics
Perfect-separation detection and explicit non-convergence failure, as in
`logit`.

### Related commands
logit, regress

---

## poisson

### Purpose
Poisson count regression — maximum likelihood via iteratively reweighted
least squares (IRLS).

### Syntax
```
poisson depvar indepvars [, robust]
```

### Arguments
- `depvar` — nonnegative count outcome (negative values are an error;
  non-integer values are flagged)
- `indepvars` — predictors; an intercept is always included

### Options
| Option | Description |
|---|---|
| `robust` | Heteroskedasticity-robust standard errors (strongly recommended for count data — see Diagnostics). |

### Examples
```
poisson visits education insurance
poisson visits education insurance, robust
```

### Output
Coefficient table (log-incidence-rate scale), log-likelihood, iterations,
McFadden pseudo-R², N.

### Assumptions
Conditional mean equals conditional variance (equidispersion). The model is
consistent for the conditional mean under correct specification even when
the variance is misspecified, provided `robust` standard errors are used.

### Diagnostics
The Pearson dispersion statistic is reported; values well above 1 signal
overdispersion and are called out with a recommendation (robust SEs;
negative binomial models are a post-V1 method and are labeled unsupported
in V1 — `KNOWN_LIMITATIONS.md`).

### Related commands
logit, probit, regress

---

## estimate list

### Purpose
List stored estimation results — every successful estimation command stores
its result (as `Model 1`, `Model 2`, … or a name you give via `note`).

### Syntax
```
estimate list [namelist]
```

### Arguments
- `namelist` — names of stored estimates. Omit to list all stored
  estimates.

### Examples
```
estimate list
estimate list model2 model3
```

### Output
For each stored estimate: the command that produced it, the specification
(estimator, variance estimator, options), N, fit statistics and the
coefficient table.

### Related commands
estimate compare, note, export table

---

## estimate compare

### Purpose
Compare stored estimates side by side.

### Syntax
```
estimate compare namelist
```

### Arguments
- `namelist` — two or more stored estimate names.

### Examples
```
estimate compare model1 model2 model3
```

### Output
A dense comparison table: coefficients with standard errors in parentheses,
N, R² / pseudo-R², fixed-effects and clustering indicators per model.
Statistics that are not comparable across the listed estimators (e.g.
comparing OLS R² with logit pseudo-R² without labeling) are marked
explicitly — incompatible model statistics are never presented as
equivalent.

### Related commands
estimate list, export table, regress

---

## note

### Purpose
Attach a research note to the most recent analysis record, or to a named
stored estimate. Notes are Markdown, recorded in the research registry and
included in exports and the replication package.

### Syntax
```
note "text"                        (attach to the most recent analysis)

note namelist "text"               (attach to a named estimate)
```

### Arguments
- `namelist` — stored estimate name(s)
- `"text"` — the note text (Markdown supported)

### Examples
```
note "Baseline specification follows Card (1999) Table 3."
note model2 "Clustered by firm per referee report #1."
```

### Output
Confirmation with the target record id.

### Related commands
estimate list, label variable, export table

---

## help

### Purpose
Open the built-in documentation.

### Syntax
```
help [command]
```

### Arguments
- `command` — a command name. Omit to open the reference index.

### Examples
```
help regress
help ttest
help
```

### Output
The documentation viewer opens on the requested command page (the same
content as `docs/COMMAND_REFERENCE.md`), inside the application.

### Related commands
(all commands)

---

## export table

### Purpose
Export publication tables from stored estimates.

### Syntax
```
export table namelist using "filename" [, format(fmt)]
```

### Arguments
- `namelist` — stored estimate names to include (columns)
- `"filename"` — output path (relative paths resolve to the project's
  `tables/` directory)

### Options
| Option | Description |
|---|---|
| `format(markdown)` | GitHub-flavored Markdown table |
| `format(csv)` | Comma-separated values |
| `format(latex)` | LaTeX booktabs-style table |
| `format(html)` | Standalone HTML table |

When `format()` is omitted, the format is inferred from the file extension
(`.md`, `.csv`, `.tex`, `.html`).

### Examples
```
export table model1 model2 using "baseline.tex", format(latex)
export table model1 model2 model3 using "baseline.csv"
```

### Output
A coefficient table with standard errors (or confidence intervals),
significance stars (`*** p < .001, ** p < .01, * p < .05` — presentation
only; exact p-values always remain in the underlying result objects), N,
fit statistics and fixed-effects/clustering indicators. Every cell is
generated from structured results — no hand-copied statistics.

### Related commands
estimate compare, estimate list, export data

---

## export data

### Purpose
Write the working dataset to disk.

### Syntax
```
export data using "filename" [, format(fmt)]
```

### Arguments
- `"filename"` — output path (relative paths resolve to the project's
  `outputs/` directory)

### Options
| Option | Description |
|---|---|
| `format(csv)` | RFC 4180 CSV (default for `.csv`) |
| `format(json)` | JSON records (default for `.json`) |

### Examples
```
export data using "cleaned.csv"
export data using "cleaned.json", format(json)
```

### Output
The number of rows and columns written.

### Assumptions
V1 exports **CSV and JSON only**. XLSX, DTA, SAV, SAS7BDAT and Parquet are
planned but stubbed: requesting them returns an explicit "not yet
supported" error listing the formats that work (`KNOWN_LIMITATIONS.md`).

### Related commands
use, import, export table

---

## Command families not in V1

Methods outside this reference are **unsupported in 1.0.0-dev and are not
pretended to exist**. Requesting or scripting them produces explicit
"not yet supported" errors. See `KNOWN_LIMITATIONS.md` for the full honest
list (two-way clustering, ARIMA/VAR, survival models, multilevel models,
survey estimation, multiple imputation, Bayesian methods, ML workflows,
psychometrics, meta-analysis).
