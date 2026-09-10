// Numeris web-preview engine — distributions.
// Faithful TypeScript port of crates/numeris-core/src/dist.rs (same
// algorithms: Lanczos log-gamma, series/continued-fraction incomplete
// gamma, continued-fraction incomplete beta, Acklam inverse normal).

const FPMIN = 1e-300;
const EPS = 1e-15;

/** Log-gamma via Lanczos approximation (g = 7, n = 9). */
export function lnGamma(x: number): number {
  if (x < 0.5) {
    const s = Math.PI / Math.sin(Math.PI * x);
    return Math.log(s) - lnGamma(1 - x);
  }
  const G = 7;
  const COEF = [
    0.99999999999980993, 676.5203681218851, -1259.1392167224028,
    771.32342877765313, -176.61502916214059, 12.507343278686905,
    -0.13857109526572012, 9.9843695780195716e-6, 1.5056327351493116e-7,
  ];
  const z = x - 1;
  let series = COEF[0];
  for (let i = 1; i < 9; i++) series += COEF[i] / (z + i);
  const t = z + G + 0.5;
  return 0.5 * Math.log(2 * Math.PI) + (z + 0.5) * Math.log(t) - t + Math.log(series);
}

/** Regularized lower incomplete gamma P(a, x). */
export function gammaP(a: number, x: number): number {
  if (a <= 0 || x < 0) return NaN;
  if (x === 0) return 0;
  const front = Math.exp(-x + a * Math.log(x) - lnGamma(a));
  if (x < a + 1) {
    let ap = a;
    let sum = 1 / a;
    let del = sum;
    for (let i = 0; i < 1000; i++) {
      ap += 1;
      del *= x / ap;
      sum += del;
      if (Math.abs(del) < Math.abs(sum) * EPS) break;
    }
    return sum * front;
  }
  let b = x + 1 - a;
  let c = 1 / FPMIN;
  let d = 1 / b;
  let h = d;
  for (let i = 1; i <= 1000; i++) {
    const an = -i * (i - a);
    b += 2;
    d = an * d + b;
    if (Math.abs(d) < FPMIN) d = FPMIN;
    c = b + an / c;
    if (Math.abs(c) < FPMIN) c = FPMIN;
    d = 1 / d;
    const del = d * c;
    h *= del;
    if (Math.abs(del - 1) < EPS) break;
  }
  return 1 - front * h;
}

/** Continued fraction core for the regularized incomplete beta. */
function betaCF(a: number, b: number, x: number): number {
  const qab = a + b;
  const qap = a + 1;
  const qam = a - 1;
  let c = 1;
  let d = 1 - (qab * x) / qap;
  if (Math.abs(d) < FPMIN) d = FPMIN;
  d = 1 / d;
  let h = d;
  for (let m = 1; m <= 500; m++) {
    const m2 = 2 * m;
    let aa = (m * (b - m) * x) / ((qam + m2) * (a + m2));
    d = 1 + aa * d;
    if (Math.abs(d) < FPMIN) d = FPMIN;
    c = 1 + aa / c;
    if (Math.abs(c) < FPMIN) c = FPMIN;
    d = 1 / d;
    h *= d * c;
    aa = (-(a + m) * (qab + m) * x) / ((a + m2) * (qap + m2));
    d = 1 + aa * d;
    if (Math.abs(d) < FPMIN) d = FPMIN;
    c = 1 + aa / c;
    if (Math.abs(c) < FPMIN) c = FPMIN;
    d = 1 / d;
    const del = d * c;
    h *= del;
    if (Math.abs(del - 1) < EPS) break;
  }
  return h;
}

/** Regularized incomplete beta I_x(a, b). */
export function betaReg(a: number, b: number, x: number): number {
  if (a <= 0 || b <= 0 || x < 0 || x > 1) return NaN;
  if (x === 0) return 0;
  if (x === 1) return 1;
  const front = Math.exp(lnGamma(a + b) - lnGamma(a) - lnGamma(b) + a * Math.log(x) + b * Math.log(1 - x));
  if (x < (a + 1) / (a + b + 2)) return (front * betaCF(a, b, x)) / a;
  return 1 - (front * betaCF(b, a, 1 - x)) / b;
}

export function normalPdf(x: number): number {
  return Math.exp(-0.5 * x * x) / Math.sqrt(2 * Math.PI);
}

export function normalCdf(x: number): number {
  if (Number.isNaN(x)) return NaN;
  if (x >= 0) return 0.5 * (1 + gammaP(0.5, 0.5 * x * x));
  return 0.5 * (1 - gammaP(0.5, 0.5 * x * x));
}

/** Standard normal quantile: Acklam approximation + one Halley refinement. */
export function normalQuantile(p: number): number {
  if (p <= 0 || p >= 1 || Number.isNaN(p)) return NaN;
  const A = [-3.969683028665376e1, 2.209460984245205e2, -2.759285104469687e2, 1.38357751867269e2, -3.066479806614716e1, 2.506628277459239];
  const B = [-5.447609879822406e1, 1.615858368580409e2, -1.556989798598866e2, 6.680131188771972e1, -1.328068155288572e1];
  const C = [-7.784894002430293e-3, -3.223964580411365e-1, -2.400758277161838, -2.549732539343734, 4.374664141464968, 2.938163982698783];
  const D = [7.784695709041462e-3, 3.224671290700398e-1, 2.445134137142996, 3.754408661907416];
  const P_LOW = 0.02425;
  let x: number;
  if (p < P_LOW) {
    const q = Math.sqrt(-2 * Math.log(p));
    x = (((((C[0] * q + C[1]) * q + C[2]) * q + C[3]) * q + C[4]) * q + C[5]) / ((((D[0] * q + D[1]) * q + D[2]) * q + D[3]) * q + 1);
  } else if (p <= 1 - P_LOW) {
    const q = p - 0.5;
    const r = q * q;
    x = (((((A[0] * r + A[1]) * r + A[2]) * r + A[3]) * r + A[4]) * r + A[5]) * q / (((((B[0] * r + B[1]) * r + B[2]) * r + B[3]) * r + B[4]) * r + 1);
  } else {
    const q = Math.sqrt(-2 * Math.log(1 - p));
    x = -(((((C[0] * q + C[1]) * q + C[2]) * q + C[3]) * q + C[4]) * q + C[5]) / ((((D[0] * q + D[1]) * q + D[2]) * q + D[3]) * q + 1);
  }
  const e = normalCdf(x) - p;
  const u = e * Math.sqrt(2 * Math.PI) * Math.exp(0.5 * x * x);
  x -= u / (1 + 0.5 * x * u);
  return x;
}

export function tCdf(t: number, df: number): number {
  if (df <= 0 || Number.isNaN(t)) return NaN;
  if (t === Infinity) return 1;
  if (t === -Infinity) return 0;
  const x = df / (df + t * t);
  const pTwo = betaReg(df / 2, 0.5, x);
  return t >= 0 ? 1 - 0.5 * pTwo : 0.5 * pTwo;
}

export function fCdf(f: number, d1: number, d2: number): number {
  if (d1 <= 0 || d2 <= 0 || Number.isNaN(f)) return NaN;
  if (f <= 0) return 0;
  const x = (d1 * f) / (d1 * f + d2);
  return betaReg(d1 / 2, d2 / 2, x);
}

export function chi2Cdf(x: number, df: number): number {
  if (df <= 0 || Number.isNaN(x)) return NaN;
  if (x <= 0) return 0;
  return gammaP(df / 2, x / 2);
}

function quantileByBisection(p: number, cdf: (x: number) => number): number {
  let lo = -1;
  let hi = 1;
  if (cdf(hi) < p) {
    let up = hi;
    for (let i = 0; i < 200; i++) {
      up *= 2;
      if (cdf(up) >= p || up > 1e300) break;
      lo = up;
    }
    hi = up;
  }
  for (let i = 0; i < 200; i++) {
    const mid = 0.5 * (lo + hi);
    if (cdf(mid) < p) lo = mid;
    else hi = mid;
    if (Math.abs(hi - lo) < 1e-12 * Math.max(Math.abs(hi), 1)) break;
  }
  return 0.5 * (lo + hi);
}

export function tQuantile(p: number, df: number): number {
  if (p <= 0 || p >= 1 || df <= 0) return NaN;
  if (p === 0.5) return 0;
  if (p > 0.5) return quantileByBisection(p, (x) => tCdf(x, df));
  return -quantileByBisection(1 - p, (x) => tCdf(x, df));
}

export function fQuantile(p: number, d1: number, d2: number): number {
  return quantileByBisection(p, (x) => fCdf(x, d1, d2));
}

export function chi2Quantile(p: number, df: number): number {
  return quantileByBisection(p, (x) => chi2Cdf(x, df));
}

export function tTwoSidedP(t: number, df: number): number {
  const p = tCdf(Math.abs(t), df);
  return Math.min(Math.max(2 * (1 - p), 0), 1);
}
