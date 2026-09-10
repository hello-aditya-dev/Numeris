// Nx* design-system primitives. One anatomy per component, consumed by
// every screen. No screen may invent its own controls.
import React from "react";

export function NxButton({
  variant = "secondary",
  size,
  children,
  ...rest
}: React.ButtonHTMLAttributes<HTMLButtonElement> & {
  variant?: "primary" | "secondary" | "tertiary" | "destructive";
  size?: "sm";
}) {
  const cls =
    variant === "primary"
      ? "nx-btn nx-btn-primary"
      : variant === "tertiary"
        ? "nx-btn nx-btn-tertiary"
        : variant === "destructive"
          ? "nx-btn nx-btn-destructive"
          : "nx-btn";
  return (
    <button className={size === "sm" ? `${cls} nx-btn-sm` : cls} {...rest}>
      {children}
    </button>
  );
}

export function NxIconBtn({ title, children, ...rest }: React.ButtonHTMLAttributes<HTMLButtonElement>) {
  return (
    <button className="nx-btn nx-iconbtn" title={title} aria-label={title} {...rest}>
      {children}
    </button>
  );
}

export function NxField({
  label,
  help,
  children,
}: {
  label: string;
  help?: string;
  children: React.ReactNode;
}) {
  return (
    <div className="nx-field">
      <label>{label}</label>
      {children}
      {help ? <div className="nx-help">{help}</div> : null}
    </div>
  );
}

export function NxInput(props: React.InputHTMLAttributes<HTMLInputElement>) {
  return <input className="nx-input" {...props} />;
}

export function NxSearchField({
  value,
  onChange,
  placeholder,
  shortcut,
}: {
  value: string;
  onChange: (v: string) => void;
  placeholder: string;
  shortcut?: string;
}) {
  return (
    <div className="nx-search">
      <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
        <circle cx="11" cy="11" r="7" />
        <path d="m20 20-3.5-3.5" />
      </svg>
      <input
        value={value}
        placeholder={placeholder}
        onChange={(e) => onChange(e.target.value)}
        aria-label={placeholder}
      />
      {shortcut ? <kbd>{shortcut}</kbd> : null}
    </div>
  );
}

export function NxSelect({
  value,
  onChange,
  options,
}: {
  value: string;
  onChange: (v: string) => void;
  options: { value: string; label: string }[];
}) {
  return (
    <select className="nx-select" value={value} onChange={(e) => onChange(e.target.value)}>
      {options.map((o) => (
        <option key={o.value} value={o.value}>
          {o.label}
        </option>
      ))}
    </select>
  );
}

export function NxSegmented({
  value,
  onChange,
  options,
}: {
  value: string;
  onChange: (v: string) => void;
  options: { value: string; label: string }[];
}) {
  return (
    <div className="nx-segmented" role="tablist">
      {options.map((o) => (
        <button key={o.value} data-active={value === o.value} onClick={() => onChange(o.value)} type="button">
          {o.label}
        </button>
      ))}
    </div>
  );
}

export function NxTabs({
  value,
  onChange,
  options,
}: {
  value: string;
  onChange: (v: string) => void;
  options: { value: string; label: string }[];
}) {
  return (
    <div className="nx-tabs" role="tablist">
      {options.map((o) => (
        <button key={o.value} data-active={value === o.value} onClick={() => onChange(o.value)} type="button">
          {o.label}
        </button>
      ))}
    </div>
  );
}

export function NxPill({ tone, children }: { tone?: "success" | "warning" | "error" | "accent"; children: React.ReactNode }) {
  return (
    <span className="nx-pill" data-tone={tone ?? undefined}>
      {children}
    </span>
  );
}

export function NxEmpty({
  title,
  hint,
  action,
}: {
  title: string;
  hint: string;
  action?: React.ReactNode;
}) {
  return (
    <div className="nx-empty">
      <div className="nx-empty-title">{title}</div>
      <div className="nx-empty-hint">{hint}</div>
      {action}
    </div>
  );
}

export function NxSectionLabel({ children }: { children: React.ReactNode }) {
  return <div className="nx-section-label">{children}</div>;
}

/** The What/Why/To-do error report pattern. */
export function NxErrorReport({ error }: { error: { what: string; why: string; action: string } }) {
  return (
    <div className="nx-error-report" role="alert">
      <h4>{error.what}</h4>
      <dl>
        <dt>Why this happened</dt>
        <dd>{error.why}</dd>
        <dt>What to do</dt>
        <dd>{error.action}</dd>
      </dl>
    </div>
  );
}

/** Statistic/value cell: tabular numerals, configured precision. */
export function formatNumber(v: number | null | undefined, digits = 3): string {
  if (v === null || v === undefined || Number.isNaN(v)) return "—";
  if (!Number.isFinite(v)) return v > 0 ? "∞" : "−∞";
  if (v === 0) return "0";
  const abs = Math.abs(v);
  if (abs >= 1e7 || abs < 1e-4) return v.toExponential(2);
  return v.toFixed(digits).replace(/\.?0+$/, "");
}

export function formatP(p: number | null | undefined): string {
  if (p === null || p === undefined || Number.isNaN(p)) return "—";
  if (p < 0.001) return "<0.001";
  return p.toFixed(3);
}

export function significanceStars(p: number | null | undefined): string {
  if (p === null || p === undefined || Number.isNaN(p)) return "";
  if (p < 0.001) return "***";
  if (p < 0.01) return "**";
  if (p < 0.05) return "*";
  return "";
}

export function formatInt(v: number): string {
  return v.toLocaleString("en-US");
}
