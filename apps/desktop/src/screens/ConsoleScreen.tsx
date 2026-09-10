// Console: command line with history, errors and result links.
import { useEffect, useRef, useState } from "react";
import { runCommand } from "../bridge";
import { useApp } from "../state/store";

export function ConsoleScreen() {
  const entries = useApp((s) => s.consoleEntries);
  const history = useApp((s) => s.consoleHistory);
  const pushConsole = useApp((s) => s.pushConsole);
  const handleOutput = useApp((s) => s.handleOutput);
  const setScreen = useApp((s) => s.setScreen);
  const [input, setInput] = useState("");
  const [histIdx, setHistIdx] = useState<number | null>(null);
  const scrollRef = useRef<HTMLDivElement>(null);
  const inputRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    scrollRef.current?.scrollTo({ top: scrollRef.current.scrollHeight });
  }, [entries]);

  const execute = async (source: string) => {
    const trimmed = source.trim();
    if (!trimmed) return;
    pushConsole({ kind: "command", text: trimmed });
    setInput("");
    setHistIdx(null);
    try {
      const out = await runCommand(trimmed);
      pushConsole({ kind: "output", text: describe(out) });
      handleOutput(out);
      if ("Estimate" in out) setScreen("results");
    } catch (e) {
      const err = e as { what: string; why: string; action: string };
      pushConsole({ kind: "error", text: `${err.what}\n  why: ${err.why}\n  to do: ${err.action}` });
    }
  };

  return (
    <div style={{ display: "flex", flexDirection: "column", height: "100%" }}>
      <div className="nx-console" ref={scrollRef} style={{ flex: 1 }} onClick={() => inputRef.current?.focus()}>
        {entries.map((e, i) => (
          <div key={i} style={{ marginBottom: 4 }}>
            {e.kind === "command" ? (
              <div className="nx-console-line">
                <span className="nx-console-prompt">›</span>
                <span>{e.text}</span>
              </div>
            ) : (
              <div
                className="nx-console-out"
                style={e.kind === "error" ? { color: "var(--color-error)", whiteSpace: "pre-wrap" } : undefined}
              >
                {e.text}
              </div>
            )}
          </div>
        ))}
      </div>
      <div className="nx-console" style={{ borderTop: "var(--border-subtle)", flexShrink: 0 }}>
        <div className="nx-console-line">
          <span className="nx-console-prompt">›</span>
          <input
            ref={inputRef}
            className="nx-console-input"
            value={input}
            placeholder="Type a command and press Enter — 'help' for the reference"
            onChange={(e) => setInput(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === "Enter") {
                execute(input);
              } else if (e.key === "ArrowUp") {
                e.preventDefault();
                if (history.length === 0) return;
                const idx = histIdx === null ? history.length - 1 : Math.max(0, histIdx - 1);
                setHistIdx(idx);
                setInput(history[idx]);
              } else if (e.key === "ArrowDown") {
                e.preventDefault();
                if (histIdx === null) return;
                const idx = histIdx + 1;
                if (idx >= history.length) {
                  setHistIdx(null);
                  setInput("");
                } else {
                  setHistIdx(idx);
                  setInput(history[idx]);
                }
              }
            }}
            autoFocus
          />
        </div>
      </div>
    </div>
  );
}

function describe(out: Record<string, unknown>): string {
  if ("Estimate" in out) {
    const est = out.Estimate as { id: string; label: string };
    return `${est.id} · ${est.label} — output opened in Results.`;
  }
  if ("Summary" in out) {
    const stats = out.Summary as { name: string; n: number; mean: number | null }[];
    return `Summary of ${stats.length} variable(s): ${stats.map((s) => `${s.name} (N=${s.n}, mean=${s.mean?.toFixed(2) ?? "—"})`).join(", ")}`;
  }
  if ("TTest" in out) {
    const r = out.TTest as { variable: string; t: number; p_two_sided: number };
    return `t-test on ${r.variable}: t = ${r.t.toFixed(3)}, p = ${r.p_two_sided.toFixed(4)}`;
  }
  if ("Corr" in out) {
    const r = out.Corr as { variables: string[]; n: number; method: string };
    return `${r.method} correlation matrix for ${r.variables.length} variables (${r.n} observations).`;
  }
  if ("Anova" in out) {
    const r = out.Anova as { variable: string; by: string; f: number; p: number };
    return `ANOVA ${r.variable} by ${r.by}: F = ${r.f.toFixed(3)}, p = ${r.p.toFixed(4)}`;
  }
  if ("Compare" in out) return "Model comparison table shown in Results.";
  if ("EstimateList" in out) {
    const list = out.EstimateList as { id: string; label: string }[];
    return list.length === 0 ? "No stored estimates." : list.map((e) => `${e.id} ${e.label}`).join(" · ");
  }
  if ("Message" in out) return String(out.Message);
  if ("Help" in out) return "Command reference opened in Results.";
  if ("Notes" in out) {
    const notes = out.Notes as string[];
    return notes.length === 0 ? "No research notes." : `${notes.length} note(s) recorded.`;
  }
  if ("DataLoaded" in out) return (out.DataLoaded as { message: string }).message;
  if ("DatasetChanged" in out) return (out.DatasetChanged as { message: string }).message;
  if ("Describe" in out) return "Variable list shown in Results.";
  if ("Frequency" in out) return "Frequency table shown in Results.";
  return "Completed.";
}
