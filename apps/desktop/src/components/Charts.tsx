// Publication-quality SVG charts. Same visual language as the rest of the
// app: restrained grids, clear axis labels, no chart chrome.
import React, { useMemo } from "react";

export function NxChartShell({ width, height, children }: { width: number; height: number; children: React.ReactNode }) {
  return (
    <svg className="nx-chart" width={width} height={height} viewBox={`0 0 ${width} ${height}`} role="img">
      {children}
    </svg>
  );
}

export function ScatterChart({ x, y, xLabel, yLabel, width = 460, height = 320 }: { x: number[]; y: number[]; xLabel: string; yLabel: string; width?: number; height?: number }) {
  const pad = { l: 46, r: 12, t: 12, b: 30 };
  const clean = useMemo(() => {
    const pts = x.map((xi, i) => [xi, y[i]] as [number, number]).filter(([a, b]) => Number.isFinite(a) && Number.isFinite(b));
    return pts;
  }, [x, y]);
  if (clean.length === 0) return null;
  const xs = clean.map((p) => p[0]);
  const ys = clean.map((p) => p[1]);
  const xMin = Math.min(...xs);
  const xMax = Math.max(...xs);
  const yMin = Math.min(...ys);
  const yMax = Math.max(...ys);
  const sx = (v: number) => pad.l + ((v - xMin) / (xMax - xMin || 1)) * (width - pad.l - pad.r);
  const sy = (v: number) => height - pad.b - ((v - yMin) / (yMax - yMin || 1)) * (height - pad.t - pad.b);
  return (
    <NxChartShell width={width} height={height}>
      {ticks(yMin, yMax).map((t) => (
        <g key={`gy${t}`}>
          <line className="gridline" x1={pad.l} x2={width - pad.r} y1={sy(t)} y2={sy(t)} />
          <text x={pad.l - 6} y={sy(t) + 3} textAnchor="end">{fmt(t)}</text>
        </g>
      ))}
      {ticks(xMin, xMax).map((t) => (
        <g key={`gx${t}`}>
          <line className="gridline" y1={pad.t} y2={height - pad.b} x1={sx(t)} x2={sx(t)} />
          <text x={sx(t)} y={height - 10} textAnchor="middle">{fmt(t)}</text>
        </g>
      ))}
      <line className="axis" x1={pad.l} x2={pad.l} y1={pad.t} y2={height - pad.b} />
      <line className="axis" x1={pad.l} x2={width - pad.r} y1={height - pad.b} y2={height - pad.b} />
      <text x={(pad.l + width - pad.r) / 2} y={height - 1} textAnchor="middle">{xLabel}</text>
      <text transform={`rotate(-90 10 ${(height - pad.b + pad.t) / 2})`} x={10} y={(height - pad.b + pad.t) / 2} textAnchor="middle">{yLabel}</text>
      {clean.map((p, i) => (
        <circle key={i} className="series" cx={sx(p[0])} cy={sy(p[1])} r={2.2} opacity={0.65} />
      ))}
    </NxChartShell>
  );
}

export function HistogramChart({ values, xLabel, bins = 24, width = 460, height = 280 }: { values: number[]; xLabel: string; bins?: number; width?: number; height?: number }) {
  const pad = { l: 40, r: 12, t: 12, b: 30 };
  const vs = values.filter((v) => Number.isFinite(v));
  if (vs.length === 0) return null;
  const min = Math.min(...vs);
  const max = Math.max(...vs);
  const w = (max - min) / bins || 1;
  const counts = new Array(bins).fill(0);
  for (const v of vs) {
    let b = Math.floor((v - min) / w);
    if (b >= bins) b = bins - 1;
    counts[b]++;
  }
  const maxC = Math.max(...counts);
  const sx = (v: number) => pad.l + ((v - min) / (max - min || 1)) * (width - pad.l - pad.r);
  const sy = (c: number) => height - pad.b - (c / maxC) * (height - pad.t - pad.b);
  return (
    <NxChartShell width={width} height={height}>
      {ticks(0, maxC).map((t) => (
        <g key={`gh${t}`}>
          <line className="gridline" x1={pad.l} x2={width - pad.r} y1={sy(t)} y2={sy(t)} />
          <text x={pad.l - 6} y={sy(t) + 3} textAnchor="end">{t}</text>
        </g>
      ))}
      {counts.map((c, i) => {
        const x0 = sx(min + i * w);
        const x1 = sx(min + (i + 1) * w);
        return <rect key={i} className="series" x={x0} y={sy(c)} width={Math.max(x1 - x0 - 0.5, 0.5)} height={height - pad.b - sy(c)} opacity={0.75} />;
      })}
      {ticks(min, max).map((t) => (
        <text key={`gx${t}`} x={sx(t)} y={height - 10} textAnchor="middle">{fmt(t)}</text>
      ))}
      <line className="axis" x1={pad.l} x2={width - pad.r} y1={height - pad.b} y2={height - pad.b} />
      <text x={(pad.l + width - pad.r) / 2} y={height - 1} textAnchor="middle">{xLabel}</text>
    </NxChartShell>
  );
}

/** Coefficient plot with 95% confidence intervals. */
export function CoefPlot({ terms, coef, ciLo, ciHi, width = 460 }: { terms: string[]; coef: number[]; ciLo: number[]; ciHi: number[]; width?: number }) {
  const rowH = 26;
  const height = terms.length * rowH + 46;
  const pad = { l: 110, r: 24, t: 14, b: 30 };
  const all = [...coef, ...ciLo, ...ciHi].filter(Number.isFinite);
  if (all.length === 0) return null;
  let lo = Math.min(...all);
  let hi = Math.max(...all);
  const m = (hi - lo) * 0.08 || 1;
  lo -= m;
  hi += m;
  const sx = (v: number) => pad.l + ((v - lo) / (hi - lo)) * (width - pad.l - pad.r);
  const cy = (i: number) => pad.t + i * rowH + rowH / 2;
  return (
    <NxChartShell width={width} height={height}>
      {ticks(lo, hi).map((t) => (
        <g key={t}>
          <line className="gridline" x1={sx(t)} x2={sx(t)} y1={pad.t} y2={height - pad.b} />
          <text x={sx(t)} y={height - 10} textAnchor="middle">{fmt(t)}</text>
        </g>
      ))}
      <line className="axis" x1={sx(0)} x2={sx(0)} y1={pad.t} y2={height - pad.b} strokeDasharray="2 2" />
      {terms.map((t, i) => (
        <g key={t}>
          <line x1={sx(ciLo[i])} x2={sx(ciHi[i])} y1={cy(i)} y2={cy(i)} className="series-line" />
          <line x1={sx(ciLo[i])} x2={sx(ciLo[i])} y1={cy(i) - 3.5} y2={cy(i) + 3.5} className="series-line" />
          <line x1={sx(ciHi[i])} x2={sx(ciHi[i])} y1={cy(i) - 3.5} y2={cy(i) + 3.5} className="series-line" />
          <circle cx={sx(coef[i])} cy={cy(i)} r={3.2} className="series" />
          <text x={pad.l - 8} y={cy(i) + 3} textAnchor="end">{t}</text>
        </g>
      ))}
    </NxChartShell>
  );
}

function ticks(min: number, max: number): number[] {
  if (!Number.isFinite(min) || !Number.isFinite(max) || min === max) return [min];
  const span = max - min;
  const step = Math.pow(10, Math.floor(Math.log10(span / 4)));
  const nice = [1, 2, 5, 10].map((m) => m * step).find((s) => span / s <= 5) ?? step;
  const out: number[] = [];
  for (let t = Math.ceil(min / nice) * nice; t <= max + 1e-12; t += nice) out.push(Math.abs(t) < 1e-12 ? 0 : t);
  return out;
}

function fmt(v: number): string {
  if (Math.abs(v) >= 1000) return v.toLocaleString("en-US", { maximumFractionDigits: 0 });
  if (Math.abs(v) >= 10) return v.toFixed(0);
  if (Math.abs(v) >= 1) return v.toFixed(1);
  return v.toFixed(2);
}
