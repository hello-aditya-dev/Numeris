// Numeris web-preview engine — linear algebra.
// Faithful port of crates/numeris-stats/src/linalg.rs: Householder QR,
// back substitution, (X'X)^{-1}, rank detection.

export type Matrix = number[][]; // rows

export function matFromCols(cols: number[][]): Matrix {
  const n = cols[0]?.length ?? 0;
  const m: Matrix = [];
  for (let i = 0; i < n; i++) m.push(cols.map((c) => c[i]));
  return m;
}

export function withConstant(x: Matrix): Matrix {
  return x.map((row) => [1, ...row]);
}

export interface QR {
  n: number;
  k: number;
  reflectors: number[][];
  r: number[][];
}

export function qrDecompose(a: Matrix): QR {
  const n = a.length;
  const k = a[0]?.length ?? 0;
  if (n < k) throw new Error("The model has more parameters than observations.");
  const work: Matrix = a.map((r) => [...r]);
  const reflectors: number[][] = [];
  const r: number[][] = Array.from({ length: k }, () => new Array(k).fill(0));

  for (let j = 0; j < k; j++) {
    const x: number[] = [];
    for (let i = j; i < n; i++) x.push(work[i][j]);
    const normX = Math.sqrt(x.reduce((s, v) => s + v * v, 0));
    const alpha = x[0] >= 0 ? normX : -normX;
    const v = [...x];
    v[0] += alpha;
    const vnorm = Math.sqrt(v.reduce((s, z) => s + z * z, 0));
    if (vnorm > 0) {
      for (let i = 0; i < v.length; i++) v[i] /= vnorm;
    }
    reflectors.push(v);
    for (let col = j; col < k; col++) {
      let dot = 0;
      for (let i = j; i < n; i++) dot += reflectors[j][i - j] * work[i][col];
      const s = 2 * dot;
      for (let i = j; i < n; i++) work[i][col] -= s * reflectors[j][i - j];
    }
  }
  let maxDiag = 0;
  for (let i = 0; i < k; i++) {
    for (let j = i; j < k; j++) r[i][j] = work[i][j];
    maxDiag = Math.max(maxDiag, Math.abs(r[i][i]));
  }
  return { n, k, reflectors, r, maxDiag } as QR & { maxDiag: number };
}

export function qrRankDeficient(qr: QR): number[] {
  const tol = (qr as QR & { maxDiag: number }).maxDiag * qr.k * Number.EPSILON * 4;
  const bad: number[] = [];
  for (let i = 0; i < qr.k; i++) {
    if (Math.abs(qr.r[i][i]) <= tol || !Number.isFinite(qr.r[i][i])) bad.push(i);
  }
  return bad;
}

export function qrQty(qr: QR, y: number[]): number[] {
  const v = [...y];
  for (let j = 0; j < qr.k; j++) {
    const refl = qr.reflectors[j];
    let dot = 0;
    for (let i = j; i < qr.n; i++) dot += refl[i - j] * v[i];
    const s = 2 * dot;
    for (let i = j; i < qr.n; i++) v[i] -= s * refl[i - j];
  }
  return v;
}

export function backSubstitute(qr: QR, b: number[]): number[] {
  const k = qr.k;
  const x = new Array(k).fill(0);
  for (let i = k - 1; i >= 0; i--) {
    let s = b[i];
    for (let j = i + 1; j < k; j++) s -= qr.r[i][j] * x[j];
    x[i] = s / qr.r[i][i];
  }
  return x;
}

export function qrSolve(qr: QR, y: number[]): number[] {
  const qty = qrQty(qr, y);
  return backSubstitute(qr, qty.slice(0, qr.k));
}

/** R^{-1} via back substitution against identity columns. */
function rInv(qr: QR): Matrix {
  const k = qr.k;
  const inv: Matrix = Array.from({ length: k }, () => new Array(k).fill(0));
  for (let j = 0; j < k; j++) {
    const e = new Array(k).fill(0);
    e[j] = 1;
    const col = backSubstitute(qr, e);
    for (let i = 0; i < k; i++) inv[i][j] = col[i];
  }
  return inv;
}

/** (X'X)^{-1} = R^{-1} R^{-T}. */
export function xtxInv(qr: QR): Matrix {
  const ri = rInv(qr);
  const k = qr.k;
  const out: Matrix = Array.from({ length: k }, () => new Array(k).fill(0));
  for (let i = 0; i < k; i++) {
    for (let j = 0; j < k; j++) {
      let s = 0;
      for (let l = 0; l < k; l++) s += ri[i][l] * ri[j][l];
      out[i][j] = s;
    }
  }
  return out;
}

export function matVec(m: Matrix, v: number[]): number[] {
  return m.map((row) => row.reduce((s, x, j) => s + x * v[j], 0));
}

export function matMulAB(a: Matrix, b: Matrix): Matrix {
  const n = a.length;
  const p = b[0].length;
  const kk = b.length;
  const out: Matrix = Array.from({ length: n }, () => new Array(p).fill(0));
  for (let i = 0; i < n; i++) {
    for (let t = 0; t < kk; t++) {
      const av = a[i][t];
      if (av === 0) continue;
      for (let j = 0; j < p; j++) out[i][j] += av * b[t][j];
    }
  }
  return out;
}

/** Leverage diagonal h_ii = x_i' A x_i. */
export function leverage(x: Matrix, a: Matrix): number[] {
  const k = a.length;
  return x.map((row) => {
    let h = 0;
    for (let i = 0; i < k; i++) {
      if (row[i] === 0) continue;
      for (let j = 0; j < k; j++) h += row[i] * a[i][j] * row[j];
    }
    return h;
  });
}
