// Application state (Zustand). UI state only — results live in the engine.
import { create } from "zustand";
import type { ExecOutput, VariableInfo, ImportReport, DiagnosticItem } from "../bridge";

export type ScreenId =
  | "home"
  | "data"
  | "analysis"
  | "console"
  | "results"
  | "comparison"
  | "graphics"
  | "tables"
  | "notes"
  | "registry"
  | "settings"
  | "license"
  | "about";

export interface ConsoleEntry {
  kind: "command" | "output" | "error";
  text: string;
}

export interface EstimateEntry {
  id: string;
  label: string;
  command: string;
  result: ExecOutput;
  diagnostics: DiagnosticItem[];
}

interface AppState {
  screen: ScreenId;
  theme: "light" | "dark";
  navCollapsed: boolean;
  paletteOpen: boolean;

  dataLoaded: boolean;
  datasetMessage: string;
  importReport: ImportReport | null;
  variables: VariableInfo[];
  nRows: number;
  selectedVariable: string | null;

  consoleEntries: ConsoleEntry[];
  consoleHistory: string[];
  estimates: EstimateEntry[];
  notes: string[];

  projectTitle: string | null;
  projectPath: string | null;

  setScreen: (s: ScreenId) => void;
  toggleTheme: () => void;
  toggleNav: () => void;
  setPalette: (open: boolean) => void;
  pushConsole: (entry: ConsoleEntry) => void;
  handleOutput: (out: ExecOutput) => void;
  addEstimate: (e: EstimateEntry) => void;
  selectVariable: (name: string | null) => void;
  addNote: (text: string) => void;
  setProject: (title: string, path: string) => void;
}

export const useApp = create<AppState>((set, get) => ({
  screen: "home",
  theme: window.matchMedia?.("(prefers-color-scheme: dark)").matches ? "dark" : "light",
  navCollapsed: false,
  paletteOpen: false,

  dataLoaded: false,
  datasetMessage: "",
  importReport: null,
  variables: [],
  nRows: 0,
  selectedVariable: null,

  consoleEntries: [{ kind: "output", text: "Numeris console ready. Type 'help' for the command reference." }],
  consoleHistory: [],
  estimates: [],
  notes: [],

  projectTitle: null,
  projectPath: null,

  setScreen: (s) => set({ screen: s }),
  toggleTheme: () => set((st) => ({ theme: st.theme === "light" ? "dark" : "light" })),
  toggleNav: () => set((st) => ({ navCollapsed: !st.navCollapsed })),
  setPalette: (open) => set({ paletteOpen: open }),
  selectVariable: (name) => set({ selectedVariable: name }),

  pushConsole: (entry) =>
    set((st) => ({
      consoleEntries: [...st.consoleEntries.slice(-500), entry],
      consoleHistory: entry.kind === "command" ? [...st.consoleHistory, entry.text] : st.consoleHistory,
    })),

  addEstimate: (e) => set((st) => ({ estimates: [...st.estimates, e] })),
  addNote: (text) => set((st) => ({ notes: [...st.notes, text] })),

  setProject: (title, path) => set({ projectTitle: title, projectPath: path }),

  handleOutput: (out) => {
    const st = get();
    if ("DataLoaded" in out) {
      set({
        dataLoaded: true,
        datasetMessage: out.DataLoaded.message,
        importReport: out.DataLoaded.report,
        variables: out.DataLoaded.variables,
        nRows: out.DataLoaded.report.rows,
      });
    } else if ("DatasetChanged" in out) {
      // refresh variables via next preview poll
    } else if ("Estimate" in out) {
      const e = out.Estimate;
      st.addEstimate({
        id: e.id,
        label: e.label,
        command: e.label,
        result: out,
        diagnostics: e.diagnostics,
      });
    } else if ("Notes" in out) {
      set({ notes: out.Notes });
    }
  },
}));
