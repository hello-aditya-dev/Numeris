// Virtualized data grid for the Data Workbench. Windowed rendering keeps
// large datasets interactive; preview is not the same as loading everything.
import { useMemo, useState } from "react";
import type { VariableInfo, ColumnData } from "../bridge";
import { formatNumber } from "./Nx";

const ROW_HEIGHT = 22;
const OVERSCAN = 20;

export function NxDataGrid({
  variables,
  columns,
  nRows,
  onSelectVariable,
  selectedVariable,
}: {
  variables: VariableInfo[];
  columns: ColumnData[];
  nRows: number;
  onSelectVariable?: (name: string) => void;
  selectedVariable?: string | null;
}) {
  const [scrollTop, setScrollTop] = useState(0);
  const [viewportH, setViewportH] = useState(600);
  const [sort, setSort] = useState<{ col: number; dir: 1 | -1 } | null>(null);

  const order = useMemo(() => {
    const all = Array.from({ length: nRows }, (_, i) => i);
    if (sort) {
      const col = columns[sort.col];
      const key = (i: number): number | null => {
        if (!col) return null;
        if ("Numeric" in col) return col.Numeric[i];
        return null;
      };
      all.sort((a, b) => {
        const ka = key(a);
        const kb = key(b);
        if (ka === null && kb === null) return 0;
        if (ka === null) return 1;
        if (kb === null) return -1;
        return (ka - kb) * sort.dir;
      });
    }
    return all;
  }, [sort, columns, nRows]);

  if (nRows === 0 || variables.length === 0) {
    return (
      <div style={{ padding: 48, color: "var(--color-text-tertiary)", textAlign: "center" }}>
        No rows to display.
      </div>
    );
  }

  const first = Math.max(0, Math.floor(scrollTop / ROW_HEIGHT) - OVERSCAN);
  const last = Math.min(nRows, Math.ceil((scrollTop + viewportH) / ROW_HEIGHT) + OVERSCAN);
  const visible = order.slice(first, last);

  return (
    <div
      className="nx-grid"
      style={{ height: "100%" }}
      onScroll={(e) => {
        setScrollTop(e.currentTarget.scrollTop);
        setViewportH(e.currentTarget.clientHeight);
      }}
    >
      <table>
        <thead>
          <tr>
            <th style={{ minWidth: 48 }}>#</th>
            {variables.map((v, j) => (
              <th
                key={v.name}
                onClick={() => {
                  if (onSelectVariable) onSelectVariable(v.name);
                  setSort((s) => (s && s.col === j ? { col: j, dir: s.dir === 1 ? -1 : 1 } : { col: j, dir: 1 }));
                }}
                style={selectedVariable === v.name ? { color: "var(--color-accent-text)" } : undefined}
                title={v.label || v.name}
              >
                {v.name}
                {sort?.col === j ? (sort.dir === 1 ? " ↑" : " ↓") : ""}
              </th>
            ))}
          </tr>
        </thead>
        <tbody>
          {visible.map((i) => (
            <tr key={i}>
              <td>{i + 1}</td>
              {variables.map((_v, j) => {
                const col = columns[j];
                if (!col) return <td key={j} />;
                if ("Numeric" in col) {
                  const val = col.Numeric[i];
                  return (
                    <td key={j} data-missing={val === null}>
                      {val === null ? "" : formatNumber(val, 4)}
                    </td>
                  );
                }
                return <td key={j}>{col.Text[i] ?? ""}</td>;
              })}
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}
