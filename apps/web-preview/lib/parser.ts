// Numeris web-preview engine — command language.
// Faithful port of crates/numeris-command (lexer, recursive-descent
// parser, canonical rendering). GUI form and console compile to the same
// AST and execute through the same engine.

/* ---------- Lexer ---------- */

export type Tok =
  | { t: "ident"; v: string }
  | { t: "number"; v: number }
  | { t: "str"; v: string }
  | { t: "comma" }
  | { t: "lparen" }
  | { t: "rparen" }
  | { t: "eq" }
  | { t: "ne" }
  | { t: "gt" }
  | { t: "ge" }
  | { t: "lt" }
  | { t: "le" }
  | { t: "plus" }
  | { t: "minus" }
  | { t: "star" }
  | { t: "slash" }
  | { t: "caret" }
  | { t: "eof" };

interface Token {
  tok: Tok;
  pos: number;
}

export function lex(source: string): Token[] {
  const tokens: Token[] = [];
  const chars = [...source];
  let i = 0;
  const n = chars.length;
  while (i < n) {
    const c = chars[i];
    if (/\s/.test(c)) {
      i++;
      continue;
    }
    if (c === "/" && chars[i + 1] === "/") {
      while (i < n && chars[i] !== "\n") i++;
      continue;
    }
    if (c === "*") {
      const byteOffset = source.slice(0, i).length;
      const lineStart = source.lastIndexOf("\n", byteOffset - 1) + 1;
      const prefix = source.slice(lineStart, byteOffset);
      if (/^\s*$/.test(prefix)) {
        while (i < n && chars[i] !== "\n") i++;
        continue;
      }
    }
    const pos = i;
    if (c === ",") { tokens.push({ tok: { t: "comma" }, pos }); i++; continue; }
    if (c === "(") { tokens.push({ tok: { t: "lparen" }, pos }); i++; continue; }
    if (c === ")") { tokens.push({ tok: { t: "rparen" }, pos }); i++; continue; }
    if (c === "+") { tokens.push({ tok: { t: "plus" }, pos }); i++; continue; }
    if (c === "-") { tokens.push({ tok: { t: "minus" }, pos }); i++; continue; }
    if (c === "*") { tokens.push({ tok: { t: "star" }, pos }); i++; continue; }
    if (c === "/") { tokens.push({ tok: { t: "slash" }, pos }); i++; continue; }
    if (c === "^") { tokens.push({ tok: { t: "caret" }, pos }); i++; continue; }
    if (c === "=") {
      if (chars[i + 1] === "=") i += 2;
      else i++;
      tokens.push({ tok: { t: "eq" }, pos });
      continue;
    }
    if (c === "!" && chars[i + 1] === "=") { tokens.push({ tok: { t: "ne" }, pos }); i += 2; continue; }
    if (c === ">") {
      if (chars[i + 1] === "=") { tokens.push({ tok: { t: "ge" }, pos }); i += 2; }
      else { tokens.push({ tok: { t: "gt" }, pos }); i++; }
      continue;
    }
    if (c === "<") {
      if (chars[i + 1] === "=") { tokens.push({ tok: { t: "le" }, pos }); i += 2; }
      else { tokens.push({ tok: { t: "lt" }, pos }); i++; }
      continue;
    }
    if (c === '"') {
      let s = "";
      i++;
      while (i < n && chars[i] !== '"') {
        s += chars[i];
        i++;
      }
      if (i >= n) {
        throw new SyntaxError("A quoted string was not closed.");
      }
      i++;
      tokens.push({ tok: { t: "str", v: s }, pos });
      continue;
    }
    if (/[0-9]/.test(c)) {
      const start = i;
      while (i < n && (/[0-9.]/.test(chars[i]))) i++;
      if (i < n && (chars[i] === "e" || chars[i] === "E")) {
        let j = i + 1;
        if (j < n && (chars[j] === "+" || chars[j] === "-")) j++;
        if (j < n && /[0-9]/.test(chars[j])) {
          i = j;
          while (i < n && /[0-9]/.test(chars[i])) i++;
        }
      }
      if (i < n && /[a-zA-Z]/.test(chars[i])) {
        while (i < n && /[a-zA-Z0-9_]/.test(chars[i])) i++;
        tokens.push({ tok: { t: "ident", v: chars.slice(start, i).join("") }, pos: start });
        continue;
      }
      const text = chars.slice(start, i).join("");
      const value = Number(text);
      if (Number.isNaN(value)) {
        throw new SyntaxError(`Could not read the number '${text}'.`);
      }
      tokens.push({ tok: { t: "number", v: value }, pos: start });
      continue;
    }
    if (/[a-zA-Z_]/.test(c)) {
      const start = i;
      while (i < n && /[a-zA-Z0-9_]/.test(chars[i])) i++;
      tokens.push({ tok: { t: "ident", v: chars.slice(start, i).join("") }, pos: start });
      continue;
    }
    throw new SyntaxError(`Unexpected character '${c}' at position ${pos}.`);
  }
  tokens.push({ tok: { t: "eof" }, pos: n });
  return tokens;
}

/* ---------- AST ---------- */

export type VcovOption =
  | "classical"
  | "robust"
  | "hc2"
  | "hc3"
  | { cluster: string }
  | { hac: number };

export type TtestForm =
  | { oneSample: number }
  | { byGroup: string; welch: boolean }
  | { twoVars: string; paired: boolean };

export type Command =
  | { kind: "summarize"; variables: string[]; detail: boolean }
  | { kind: "describe"; variables: string[] }
  | { kind: "correlate"; variables: string[]; spearman: boolean }
  | { kind: "ttest"; variable: string; form: TtestForm }
  | { kind: "anova"; outcome: string; group: string }
  | { kind: "regress"; outcome: string; predictors: string[]; vcov: VcovOption }
  | { kind: "xtreg"; outcome: string; predictors: string[]; entity: string; model: "fe" | "be" | "pooled"; vcov: VcovOption }
  | { kind: "ivregress"; outcome: string; endogenous: string[]; instruments: string[]; exogenous: string[]; vcov: VcovOption }
  | { kind: "did"; outcome: string; controls: string[]; treat: string; time: string; vcov: VcovOption }
  | { kind: "logit" | "probit" | "poisson"; outcome: string; predictors: string[]; robust: boolean }
  | { kind: "generate"; name: string; expr: Expr }
  | { kind: "replace"; name: string; expr: Expr; cond: Cond | null }
  | { kind: "drop"; variables: string[] | null; cond: Cond | null }
  | { kind: "keep"; variables: string[] | null; cond: Cond | null }
  | { kind: "rename"; from: string; to: string }
  | { kind: "label"; variable: string; text: string }
  | { kind: "sort"; variable: string; descending: boolean }
  | { kind: "estimateList" }
  | { kind: "estimateCompare"; ids: string[] }
  | { kind: "note"; text: string }
  | { kind: "notes" }
  | { kind: "help"; command: string | null };

export type Expr =
  | { type: "num"; v: number }
  | { type: "var"; v: string }
  | { type: "bin"; op: "+" | "-" | "*" | "/" | "^"; l: Expr; r: Expr }
  | { type: "neg"; v: Expr }
  | { type: "call"; fn: FnKind; arg: Expr };

export type FnKind = "ln" | "log" | "exp" | "sqrt" | "abs" | "round" | "rank" | "standardize";

export interface Cond {
  variable: string;
  op: "==" | "!=" | ">" | ">=" | "<" | "<=";
  value: number;
}

/* ---------- Parser ---------- */

class Parser {
  private i = 0;
  constructor(private tokens: Token[]) {}

  private peek(): Tok {
    return this.tokens[this.i].tok;
  }

  private advance(): Tok {
    const t = this.tokens[this.i].tok;
    if (this.i + 1 < this.tokens.length) this.i++;
    return t;
  }

  private isIdent(name: string): boolean {
    const t = this.peek();
    return t.t === "ident" && t.v.toLowerCase() === name;
  }

  private eat(kind: Tok["t"]): void {
    if (this.peek().t !== kind) {
      throw new SyntaxError(`Unexpected token '${JSON.stringify(this.peek())}' in command.`);
    }
    this.advance();
  }

  private eatIdent(): string {
    if (this.peek().t !== "ident") throw new SyntaxError("Expected a variable name.");
    return (this.advance() as { t: "ident"; v: string }).v;
  }

  private eatNumber(): number {
    if (this.peek().t !== "number") throw new SyntaxError("Expected a number.");
    return (this.advance() as { t: "number"; v: number }).v;
  }

  private eatString(): string {
    if (this.peek().t !== "str") throw new SyntaxError("Expected a quoted string.");
    return (this.advance() as { t: "str"; v: string }).v;
  }

  private atBoundary(): boolean {
    const t = this.peek().t;
    return t === "comma" || t === "eof" || t === "rparen";
  }

  private varlist(): string[] {
    const vars: string[] = [];
    while (this.peek().t === "ident") vars.push(this.eatIdent());
    return vars;
  }

  private vcovOptions(): VcovOption {
    let vcov: VcovOption = "classical";
    if (this.peek().t === "comma") {
      this.advance();
      while (true) {
        if (this.peek().t !== "ident") throw new SyntaxError("Expected an option after ','.");
        const opt = this.eatIdent().toLowerCase();
        if (opt === "robust") vcov = "robust";
        else if (opt === "hc2") vcov = "hc2";
        else if (opt === "hc3") vcov = "hc3";
        else if (opt === "cluster") {
          this.eat("lparen");
          const v = this.eatIdent();
          this.eat("rparen");
          vcov = { cluster: v };
        } else if (opt === "vce") {
          this.eat("lparen");
          this.eatIdent();
          const lags = this.eatNumber();
          this.eat("rparen");
          vcov = { hac: lags };
        } else {
          throw new SyntaxError(`Unknown option '${opt}'. Regression commands accept: robust, hc2, hc3, cluster(v), vce(hac L).`);
        }
        if (this.peek().t === "comma") {
          this.advance();
          continue;
        }
        if (this.peek().t === "ident") continue;
        break;
      }
    }
    return vcov;
  }

  private condition(): Cond {
    const variable = this.eatIdent();
    const opTok = this.peek().t;
    if (!["eq", "ne", "gt", "ge", "lt", "le"].includes(opTok)) {
      throw new SyntaxError("Conditions compare a variable to a number with ==, !=, >, >=, < or <=.");
    }
    this.advance();
    const op = opTok === "eq" ? "==" : opTok === "ne" ? "!=" : opTok === "gt" ? ">" : opTok === "ge" ? ">=" : opTok === "lt" ? "<" : "<=";
    const value = this.eatNumber();
    return { variable, op, value };
  }

  parse(): Command {
    const head = this.eatIdent().toLowerCase();
    switch (head) {
      case "summarize":
      case "summarise":
      case "su": {
        const variables = this.varlist();
        let detail = false;
        if (this.peek().t === "comma") {
          this.advance();
          const opt = this.eatIdent();
          if (opt.toLowerCase() !== "detail") throw new SyntaxError(`Unknown option '${opt}'.`);
          detail = true;
        }
        return { kind: "summarize", variables, detail };
      }
      case "describe":
      case "des":
        return { kind: "describe", variables: this.varlist() };
      case "correlate":
      case "corr": {
        const variables = this.varlist();
        let spearman = false;
        if (this.peek().t === "comma") {
          this.advance();
          const opt = this.eatIdent();
          if (opt.toLowerCase() !== "spearman") throw new SyntaxError(`Unknown option '${opt}'.`);
          spearman = true;
        }
        return { kind: "correlate", variables, spearman };
      }
      case "ttest": {
        const variable = this.eatIdent();
        const t = this.peek().t;
        if (t === "eq") {
          this.advance();
          if (this.peek().t === "number") {
            return { kind: "ttest", variable, form: { oneSample: this.eatNumber() } };
          }
          const other = this.eatIdent();
          let paired = false;
          if (this.peek().t === "comma") {
            this.advance();
            const opt = this.eatIdent();
            if (opt.toLowerCase() !== "paired") throw new SyntaxError(`Unknown t-test option '${opt}'.`);
            paired = true;
          }
          return { kind: "ttest", variable, form: { twoVars: other, paired } };
        }
        if (t === "comma") {
          this.advance();
          const by = this.eatIdent();
          if (by.toLowerCase() !== "by") throw new SyntaxError("The option after ',' in a t-test must be 'by'.");
          this.eat("lparen");
          const group = this.eatIdent();
          this.eat("rparen");
          const welch = this.isIdent("welch");
          if (welch) this.advance();
          return { kind: "ttest", variable, form: { byGroup: group, welch } };
        }
        throw new SyntaxError("A t-test needs a comparison: 'ttest x == value', 'ttest x, by(group)' or 'ttest a == b'.");
      }
      case "anova": {
        const outcome = this.eatIdent();
        const group = this.eatIdent();
        return { kind: "anova", outcome, group };
      }
      case "regress":
      case "reg": {
        const outcome = this.eatIdent();
        const predictors: string[] = [];
        while (!this.atBoundary()) predictors.push(this.eatIdent());
        const vcov = this.vcovOptions();
        return { kind: "regress", outcome, predictors, vcov };
      }
      case "xtreg": {
        const outcome = this.eatIdent();
        const predictors: string[] = [];
        while (!this.atBoundary()) predictors.push(this.eatIdent());
        let entity: string | null = null;
        let model: "fe" | "be" | "pooled" = "pooled";
        let vcov: VcovOption = "classical";
        if (this.peek().t === "comma") {
          this.advance();
          while (true) {
            const opt = this.eatIdent().toLowerCase();
            if (opt === "fe" || opt === "fixed") model = "fe";
            else if (opt === "be" || opt === "between") model = "be";
            else if (opt === "pooled") model = "pooled";
            else if (opt === "entity" || opt === "id") {
              this.eat("lparen");
              entity = this.eatIdent();
              this.eat("rparen");
            } else if (opt === "robust") vcov = "robust";
            else if (opt === "hc2") vcov = "hc2";
            else if (opt === "hc3") vcov = "hc3";
            else if (opt === "cluster") {
              this.eat("lparen");
              const v = this.eatIdent();
              this.eat("rparen");
              vcov = { cluster: v };
            } else throw new SyntaxError(`Unknown xtreg option '${opt}'.`);
            if (this.peek().t === "comma") {
              this.advance();
              continue;
            }
            if (this.peek().t === "ident") continue;
            break;
          }
        }
        if (!entity) throw new SyntaxError("xtreg requires the entity() option.");
        return { kind: "xtreg", outcome, predictors, entity, model, vcov };
      }
      case "ivregress": {
        const estimator = this.eatIdent();
        if (estimator.toLowerCase() !== "2sls") {
          throw new SyntaxError("ivregress supports the 2sls estimator.");
        }
        const outcome = this.eatIdent();
        const endogenous: string[] = [];
        const instruments: string[] = [];
        const exogenous: string[] = [];
        if (this.peek().t === "lparen") {
          this.advance();
          while (this.peek().t === "ident") endogenous.push(this.eatIdent());
          this.eat("eq");
          while (this.peek().t === "ident") instruments.push(this.eatIdent());
          this.eat("rparen");
        } else {
          throw new SyntaxError("ivregress requires instrumented variables in parentheses.");
        }
        while (!this.atBoundary()) exogenous.push(this.eatIdent());
        const vcov = this.vcovOptions();
        return { kind: "ivregress", outcome, endogenous, instruments, exogenous, vcov };
      }
      case "did": {
        const outcome = this.eatIdent();
        const controls: string[] = [];
        while (!this.atBoundary()) controls.push(this.eatIdent());
        let treat: string | null = null;
        let time: string | null = null;
        let vcov: VcovOption = "classical";
        if (this.peek().t === "comma") {
          this.advance();
          while (true) {
            const opt = this.eatIdent().toLowerCase();
            if (opt === "treat") {
              this.eat("lparen");
              treat = this.eatIdent();
              this.eat("rparen");
            } else if (opt === "time" || opt === "post") {
              this.eat("lparen");
              time = this.eatIdent();
              this.eat("rparen");
            } else if (opt === "robust") vcov = "robust";
            else if (opt === "cluster") {
              this.eat("lparen");
              const v = this.eatIdent();
              this.eat("rparen");
              vcov = { cluster: v };
            } else throw new SyntaxError(`Unknown did option '${opt}'.`);
            if (this.peek().t === "comma") {
              this.advance();
              continue;
            }
            if (this.peek().t === "ident") continue;
            break;
          }
        }
        if (!treat) throw new SyntaxError("did requires the treat() option.");
        if (!time) throw new SyntaxError("did requires the time() option.");
        return { kind: "did", outcome, controls, treat, time, vcov };
      }
      case "logit":
      case "logistic":
      case "probit":
      case "poisson": {
        const outcome = this.eatIdent();
        const predictors: string[] = [];
        while (!this.atBoundary()) predictors.push(this.eatIdent());
        let robust = false;
        if (this.peek().t === "comma") {
          this.advance();
          const opt = this.eatIdent();
          if (opt.toLowerCase() !== "robust") throw new SyntaxError(`Unknown option '${opt}'.`);
          robust = true;
        }
        const fam = head === "logit" || head === "logistic" ? "logit" : head === "probit" ? "probit" : "poisson";
        return { kind: fam, outcome, predictors, robust };
      }
      case "generate":
      case "gen": {
        const name = this.eatIdent();
        this.eat("eq");
        const expr = this.expression();
        return { kind: "generate", name, expr };
      }
      case "replace": {
        const name = this.eatIdent();
        this.eat("eq");
        const expr = this.expression();
        let cond: Cond | null = null;
        if (this.isIdent("if")) {
          this.advance();
          cond = this.condition();
        }
        return { kind: "replace", name, expr, cond };
      }
      case "drop": {
        if (this.isIdent("if")) {
          this.advance();
          return { kind: "drop", variables: null, cond: this.condition() };
        }
        return { kind: "drop", variables: this.varlist(), cond: null };
      }
      case "keep": {
        if (this.isIdent("if")) {
          this.advance();
          return { kind: "keep", variables: null, cond: this.condition() };
        }
        return { kind: "keep", variables: this.varlist(), cond: null };
      }
      case "rename": {
        const from = this.eatIdent();
        const to = this.eatIdent();
        return { kind: "rename", from, to };
      }
      case "label": {
        const next = this.eatIdent();
        if (next.toLowerCase() !== "variable") throw new SyntaxError("'label' must be followed by 'variable'.");
        const variable = this.eatIdent();
        const text = this.eatString();
        return { kind: "label", variable, text };
      }
      case "sort": {
        const variable = this.eatIdent();
        let descending = false;
        if (this.peek().t === "comma") {
          this.advance();
          const opt = this.eatIdent();
          if (opt.toLowerCase() !== "desc" && opt.toLowerCase() !== "descending") throw new SyntaxError(`Unknown option '${opt}'.`);
          descending = true;
        }
        return { kind: "sort", variable, descending };
      }
      case "estimate": {
        const sub = this.eatIdent().toLowerCase();
        if (sub === "list") return { kind: "estimateList" };
        if (sub === "compare") {
          const ids: string[] = [];
          while (this.peek().t === "ident") ids.push(this.eatIdent());
          return { kind: "estimateCompare", ids };
        }
        throw new SyntaxError("estimate supports 'list' and 'compare'.");
      }
      case "note":
        return { kind: "note", text: this.eatString() };
      case "notes":
        return { kind: "notes" };
      case "help":
        return { kind: "help", command: this.peek().t === "ident" ? this.eatIdent() : null };
      default:
        throw new SyntaxError(`Unknown command '${head}'. Run 'help' to see the command reference.`);
    }
  }

  /* Expressions */
  private expression(): Expr {
    let left = this.term();
    while (this.peek().t === "plus" || this.peek().t === "minus") {
      const op = this.advance().t === "plus" ? "+" : "-";
      left = { type: "bin", op, l: left, r: this.term() };
    }
    return left;
  }

  private term(): Expr {
    let left = this.factor();
    while (this.peek().t === "star" || this.peek().t === "slash") {
      const op = this.advance().t === "star" ? "*" : "/";
      left = { type: "bin", op, l: left, r: this.factor() };
    }
    return left;
  }

  private factor(): Expr {
    const base = this.unary();
    if (this.peek().t === "caret") {
      this.advance();
      return { type: "bin", op: "^", l: base, r: this.factor() };
    }
    return base;
  }

  private unary(): Expr {
    if (this.peek().t === "minus") {
      this.advance();
      return { type: "neg", v: this.unary() };
    }
    return this.primary();
  }

  private primary(): Expr {
    const t = this.peek();
    if (t.t === "number") {
      this.advance();
      return { type: "num", v: t.v };
    }
    if (t.t === "lparen") {
      this.advance();
      const inner = this.expression();
      this.eat("rparen");
      return inner;
    }
    if (t.t === "ident") {
      this.advance();
      const fns: Record<string, FnKind> = { ln: "ln", log: "log", exp: "exp", sqrt: "sqrt", abs: "abs", round: "round", rank: "rank", standardize: "standardize" };
      const fn = fns[t.v.toLowerCase()];
      if (fn) {
        this.eat("lparen");
        const arg = this.expression();
        this.eat("rparen");
        return { type: "call", fn, arg };
      }
      return { type: "var", v: t.v };
    }
    throw new SyntaxError(`Unexpected token in expression.`);
  }
}

export function parse(source: string): Command {
  const tokens = lex(source);
  const p = new Parser(tokens);
  const cmd = p.parse();
  return cmd;
}

/* ---------- Canonical rendering ---------- */

function vcovSuffix(v: VcovOption): string {
  if (v === "classical") return "";
  if (v === "robust") return ", robust";
  if (v === "hc2") return ", hc2";
  if (v === "hc3") return ", hc3";
  if (typeof v === "object" && "cluster" in v) return `, cluster(${v.cluster})`;
  if (typeof v === "object" && "hac" in v) return `, vce(hac ${v.hac})`;
  return "";
}

function fmtNum(v: number): string {
  return Number.isInteger(v) && Math.abs(v) < 1e15 ? String(v) : String(v);
}

export function render(cmd: Command): string {
  switch (cmd.kind) {
    case "summarize":
      return `summarize${cmd.variables.length ? " " + cmd.variables.join(" ") : ""}${cmd.detail ? ", detail" : ""}`;
    case "describe":
      return `describe${cmd.variables.length ? " " + cmd.variables.join(" ") : ""}`;
    case "correlate":
      return `correlate${cmd.variables.length ? " " + cmd.variables.join(" ") : ""}${cmd.spearman ? ", spearman" : ""}`;
    case "ttest": {
      const f = cmd.form;
      if ("oneSample" in f) return `ttest ${cmd.variable} == ${fmtNum(f.oneSample)}`;
      if ("byGroup" in f) return `ttest ${cmd.variable}, by(${f.byGroup})${f.welch ? " welch" : ""}`;
      return `ttest ${cmd.variable} == ${f.twoVars}${f.paired ? ", paired" : ""}`;
    }
    case "anova":
      return `anova ${cmd.outcome} ${cmd.group}`;
    case "regress":
      return `regress ${[cmd.outcome, ...cmd.predictors].join(" ")}${vcovSuffix(cmd.vcov)}`;
    case "xtreg":
      return `xtreg ${[cmd.outcome, ...cmd.predictors].join(" ")}, ${cmd.model}, entity(${cmd.entity})${vcovSuffix(cmd.vcov)}`;
    case "ivregress":
      return `ivregress 2sls ${cmd.outcome} (${cmd.endogenous.join(" ")} = ${cmd.instruments.join(" ")})${cmd.exogenous.length ? " " + cmd.exogenous.join(" ") : ""}${vcovSuffix(cmd.vcov)}`;
    case "did":
      return `did ${[cmd.outcome, ...cmd.controls].join(" ")}, treat(${cmd.treat}), time(${cmd.time})${vcovSuffix(cmd.vcov)}`;
    case "logit":
    case "probit":
    case "poisson":
      return `${cmd.kind} ${[cmd.outcome, ...cmd.predictors].join(" ")}${cmd.robust ? ", robust" : ""}`;
    case "generate":
      return `generate ${cmd.name} = ${renderExpr(cmd.expr)}`;
    case "replace":
      return `replace ${cmd.name} = ${renderExpr(cmd.expr)}${cmd.cond ? ` if ${renderCond(cmd.cond)}` : ""}`;
    case "drop":
      return cmd.variables ? `drop ${cmd.variables.join(" ")}` : `drop if ${renderCond(cmd.cond!)}`;
    case "keep":
      return cmd.variables ? `keep ${cmd.variables.join(" ")}` : `keep if ${renderCond(cmd.cond!)}`;
    case "rename":
      return `rename ${cmd.from} ${cmd.to}`;
    case "label":
      return `label variable ${cmd.variable} "${cmd.text}"`;
    case "sort":
      return `sort ${cmd.variable}${cmd.descending ? " desc" : ""}`;
    case "estimateList":
      return "estimate list";
    case "estimateCompare":
      return `estimate compare${cmd.ids.length ? " " + cmd.ids.join(" ") : ""}`;
    case "note":
      return `note "${cmd.text}"`;
    case "notes":
      return "notes";
    case "help":
      return cmd.command ? `help ${cmd.command}` : "help";
  }
}

function renderCond(c: Cond): string {
  return `${c.variable} ${c.op} ${fmtNum(c.value)}`;
}

function renderExpr(e: Expr): string {
  switch (e.type) {
    case "num":
      return fmtNum(e.v);
    case "var":
      return e.v;
    case "bin":
      return `${renderExpr(e.l)} ${e.op} ${renderExpr(e.r)}`;
    case "neg":
      return `-${renderExpr(e.v)}`;
    case "call":
      return `${e.fn}(${renderExpr(e.arg)})`;
  }
}
