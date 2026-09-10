// App shell: title bar, navigation, main region, status bar, palette,
// and the global keyboard model.
import { useEffect, useMemo } from "react";
import "./styles/tokens.css";
import { useApp, type ScreenId } from "./state/store";
import { NxCommandPalette, type PaletteAction } from "./components/CommandPalette";
import { AnalysisScreen } from "./screens/AnalysisScreen";
import { ConsoleScreen } from "./screens/ConsoleScreen";
import { DataWorkbenchScreen } from "./screens/DataWorkbenchScreen";
import {
  AboutScreen,
  ComparisonScreen,
  GraphicsScreen,
  HomeScreen,
  LicenseScreen,
  NotesScreen,
  RegistryScreen,
  ResultsScreen,
  SettingsScreen,
  TablesScreen,
} from "./screens/Screens";

const NAV: { section: string; items: { id: ScreenId; label: string; hint: string }[] }[] = [
  {
    section: "Workspace",
    items: [
      { id: "home", label: "Project home", hint: "Overview" },
      { id: "data", label: "Data", hint: "Workbench" },
    ],
  },
  {
    section: "Analysis",
    items: [
      { id: "analysis", label: "Model workspace", hint: "Forms" },
      { id: "console", label: "Console", hint: "Ctrl+`" },
    ],
  },
  {
    section: "Output",
    items: [
      { id: "results", label: "Results", hint: "" },
      { id: "comparison", label: "Model comparison", hint: "" },
      { id: "graphics", label: "Graphics", hint: "" },
      { id: "tables", label: "Tables", hint: "" },
    ],
  },
  {
    section: "Research",
    items: [
      { id: "notes", label: "Notes", hint: "" },
      { id: "registry", label: "Registry & replay", hint: "" },
    ],
  },
  {
    section: "Application",
    items: [
      { id: "settings", label: "Settings", hint: "" },
      { id: "license", label: "License", hint: "" },
      { id: "about", label: "About", hint: "" },
    ],
  },
];

export default function App() {
  const screen = useApp((s) => s.screen);
  const theme = useApp((s) => s.theme);
  const navCollapsed = useApp((s) => s.navCollapsed);
  const paletteOpen = useApp((s) => s.paletteOpen);
  const setPalette = useApp((s) => s.setPalette);
  const setScreen = useApp((s) => s.setScreen);
  const toggleTheme = useApp((s) => s.toggleTheme);
  const toggleNav = useApp((s) => s.toggleNav);
  const dataLoaded = useApp((s) => s.dataLoaded);
  const nRows = useApp((s) => s.nRows);
  const variables = useApp((s) => s.variables);
  const estimates = useApp((s) => s.estimates);
  const projectTitle = useApp((s) => s.projectTitle);

  useEffect(() => {
    document.documentElement.className = theme === "dark" ? "nx-theme-dark" : "nx-theme-light";
  }, [theme]);

  const actions: PaletteAction[] = useMemo(() => {
    const navActions = NAV.flatMap((s) => s.items).map((item) => ({
      id: `nav-${item.id}`,
      group: "Navigate",
      label: item.label,
      run: () => setScreen(item.id),
    }));
    const modelActions = [
      { id: "run-ols", group: "Analyze", label: "Run linear regression", hint: "", run: () => setScreen("analysis") },
      { id: "run-logit", group: "Analyze", label: "Run logistic regression", hint: "", run: () => setScreen("analysis") },
      { id: "compare", group: "Analyze", label: "Compare models", run: () => setScreen("comparison") },
      { id: "summarize", group: "Data", label: "Summarize variables", run: () => setScreen("data") },
      { id: "theme", group: "View", label: "Toggle light/dark theme", run: toggleTheme },
    ];
    return [...navActions, ...modelActions];
  }, [setScreen, toggleTheme]);

  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      const mod = e.metaKey || e.ctrlKey;
      if (mod && e.key.toLowerCase() === "k") {
        e.preventDefault();
        setPalette(!paletteOpen);
      } else if (mod && e.key === "Enter") {
        e.preventDefault();
        setScreen("results");
      } else if (e.key === "Escape") {
        setPalette(false);
      }
    };
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, [paletteOpen, setPalette, setScreen]);

  const current = (() => {
    switch (screen) {
      case "home": return <HomeScreen />;
      case "data": return <DataWorkbenchScreen />;
      case "analysis": return <AnalysisScreen />;
      case "console": return <ConsoleScreen />;
      case "results": return <ResultsScreen />;
      case "comparison": return <ComparisonScreen />;
      case "graphics": return <GraphicsScreen />;
      case "tables": return <TablesScreen />;
      case "notes": return <NotesScreen />;
      case "registry": return <RegistryScreen />;
      case "settings": return <SettingsScreen />;
      case "license": return <LicenseScreen />;
      case "about": return <AboutScreen />;
    }
  })();

  return (
    <div className="nx-app" data-nav-collapsed={navCollapsed}>
      <header className="nx-titlebar">
        <button
          className="nx-btn nx-iconbtn"
          title="Toggle navigation"
          onClick={toggleNav}
        >
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
            <path d="M3 6h18M3 12h18M3 18h18" />
          </svg>
        </button>
        <span className="nx-logo">
          <span className="nx-logo-mark">N</span>
          Numeris
        </span>
        <span style={{ color: "var(--color-text-tertiary)", fontSize: 12 }}>
          {projectTitle ?? "Untitled workspace"}
        </span>
        <div className="nx-titlebar-actions" style={{ marginLeft: "auto" }}>
          <button className="nx-btn nx-iconbtn" title="Toggle theme" onClick={toggleTheme}>
            {theme === "dark" ? (
              <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                <circle cx="12" cy="12" r="4" />
                <path d="M12 2v2m0 16v2M4.9 4.9l1.4 1.4m11.4 11.4 1.4 1.4M2 12h2m16 0h2M4.9 19.1l1.4-1.4m11.4-11.4 1.4-1.4" />
              </svg>
            ) : (
              <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                <path d="M21 12.8A9 9 0 1 1 11.2 3a7 7 0 0 0 9.8 9.8Z" />
              </svg>
            )}
          </button>
          <button className="nx-btn nx-btn-sm" onClick={() => setPalette(true)} title="Command palette (Ctrl/Cmd+K)">
            Search…
          </button>
        </div>
      </header>

      <nav className="nx-sidebar" aria-label="Primary">
        {NAV.map((section) => (
          <div key={section.section} className="nx-nav-section">
            <div className="nx-nav-label">{section.section}</div>
            {section.items.map((item) => (
              <button key={item.id} className="nx-nav-item" data-active={screen === item.id} onClick={() => setScreen(item.id)}>
                {item.label}
              </button>
            ))}
          </div>
        ))}
      </nav>

      <main className="nx-main">{current}</main>

      <footer className="nx-statusbar">
        <span>
          {dataLoaded ? `${nRows.toLocaleString()} observations · ${variables.length} variables` : "No dataset loaded"}
        </span>
        <span>{estimates.length > 0 ? `${estimates.length} estimates` : ""}</span>
        <div className="nx-status-right">
          <span>Ready</span>
          <span>Offline</span>
          <span>Licensed</span>
        </div>
      </footer>

      <NxCommandPalette open={paletteOpen} onClose={() => setPalette(false)} actions={actions} />
    </div>
  );
}
