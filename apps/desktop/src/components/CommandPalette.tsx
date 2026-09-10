// Command palette (Ctrl/Cmd+K) — first-class, keyboard-first.
import { useEffect, useMemo, useRef, useState } from "react";

export interface PaletteAction {
  id: string;
  label: string;
  hint?: string;
  group: string;
  run: () => void;
}

export function NxCommandPalette({
  open,
  onClose,
  actions,
}: {
  open: boolean;
  onClose: () => void;
  actions: PaletteAction[];
}) {
  const [query, setQuery] = useState("");
  const [selected, setSelected] = useState(0);
  const inputRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    if (open) {
      setQuery("");
      setSelected(0);
      requestAnimationFrame(() => inputRef.current?.focus());
    }
  }, [open]);

  const filtered = useMemo(() => {
    const q = query.trim().toLowerCase();
    if (!q) return actions;
    return actions.filter((a) => {
      const hay = `${a.label} ${a.group} ${a.hint ?? ""}`.toLowerCase();
      return q.split(/\s+/).every((word) => hay.includes(word));
    });
  }, [query, actions]);

  useEffect(() => setSelected(0), [query]);

  if (!open) return null;

  return (
    <div
      className="nx-palette-backdrop"
      onMouseDown={(e) => {
        if (e.target === e.currentTarget) onClose();
      }}
    >
      <div className="nx-palette" role="dialog" aria-label="Command palette">
        <input
          ref={inputRef}
          className="nx-palette-input"
          placeholder="Search commands…"
          value={query}
          onChange={(e) => setQuery(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === "ArrowDown") {
              e.preventDefault();
              setSelected((s) => Math.min(s + 1, filtered.length - 1));
            } else if (e.key === "ArrowUp") {
              e.preventDefault();
              setSelected((s) => Math.max(s - 1, 0));
            } else if (e.key === "Enter") {
              const a = filtered[selected];
              if (a) {
                onClose();
                a.run();
              }
            } else if (e.key === "Escape") {
              onClose();
            }
          }}
        />
        <div className="nx-palette-list">
          {filtered.length === 0 ? (
            <div style={{ padding: 16, color: "var(--color-text-tertiary)", fontSize: 13 }}>No matching commands.</div>
          ) : (
            filtered.slice(0, 30).map((a, i) => (
              <div
                key={a.id}
                className="nx-palette-item"
                data-selected={i === selected}
                onMouseEnter={() => setSelected(i)}
                onMouseDown={(e) => {
                  e.preventDefault();
                  onClose();
                  a.run();
                }}
              >
                <span style={{ color: "var(--color-text-tertiary)", fontSize: 11, minWidth: 88 }}>{a.group}</span>
                <span>{a.label}</span>
                {a.hint ? <kbd>{a.hint}</kbd> : null}
              </div>
            ))
          )}
        </div>
      </div>
    </div>
  );
}
