// Numeris web-preview engine — deterministic synthetic dataset and CSV import.
// The demo data is generated with a fixed-seed PRNG so every session sees
// identical data (deterministic philosophy), and clearly labeled synthetic.

/** mulberry32 — small, fast, deterministic PRNG. */
function mulberry32(seed: number): () => number {
  let a = seed >>> 0;
  return function () {
    a |= 0;
    a = (a + 0x6d2b79f5) | 0;
    let t = Math.imul(a ^ (a >>> 15), 1 | a);
    t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t;
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}

function randn(rnd: () => number): number {
  // Box–Muller with a deterministic uniform pair.
  const u1 = Math.max(rnd(), 1e-12);
  const u2 = rnd();
  return Math.sqrt(-2 * Math.log(u1)) * Math.cos(2 * Math.PI * u2);
}

export interface VariableInfo {
  name: string;
  label: string;
  storage: "Numeric" | "Text";
  semantic: "Continuous" | "Categorical" | "Identifier" | "Temporal" | "Unknown";
}

export interface Dataset {
  variables: VariableInfo[];
  /** Numeric columns; missing = null. */
  numeric: Record<string, (number | null)[]>;
  /** Text columns; missing = null. */
  text: Record<string, (string | null)[]>;
  rows: number;
  name: string;
}

const LABELS: Record<string, string> = {
  id: "Worker identifier",
  year: "Calendar year",
  firm: "Firm identifier",
  wage: "Hourly wage (USD)",
  education: "Years of education",
  experience: "Years of work experience",
  female: "Female indicator",
  urban: "Urban residence indicator",
  distance: "Distance to nearest college (10 km units)",
  employed: "Employed full-time indicator",
  visits: "Health visits last year (count)",
  treated: "Firm enrolled in training program",
  post: "Post-program year (2020+)",
  productivity: "Firm productivity index",
};

/**
 * Synthetic demonstration panel: 40 firms × 5 years (2018–2022) × 3 workers.
 * Deterministically generated with seed 20250910.
 */
export function buildDemoDataset(): Dataset {
  const rnd = mulberry32(20250910);
  const variables: VariableInfo[] = [
    "id", "year", "firm", "wage", "education", "experience", "female",
    "urban", "distance", "employed", "visits", "treated", "post",
  ].map((name) => ({
    name,
    label: LABELS[name] ?? "",
    storage: "Numeric" as const,
    semantic: name === "id" || name === "firm" ? ("Identifier" as const)
      : ["female", "urban", "employed", "treated", "post"].includes(name) ? ("Categorical" as const)
      : name === "year" ? ("Temporal" as const)
      : ("Continuous" as const),
  }));
  const numeric: Record<string, (number | null)[]> = {};
  for (const v of variables) numeric[v.name] = [];

  let id = 0;
  // Firm-level characteristics.
  const firmProductivity: number[] = [];
  const firmTreated: number[] = [];
  for (let f = 0; f < 40; f++) {
    firmProductivity.push(0.8 + rnd() * 0.4);
    firmTreated.push(f % 3 === 0 ? 1 : 0); // ~1/3 treated firms
  }

  for (let f = 0; f < 40; f++) {
    for (let year = 2018; year <= 2022; year++) {
      for (let w = 0; w < 3; w++) {
        id++;
        const workerId = f * 15 + w + 1;
        // Distance to college (IV for education): varies by worker.
        const distance = 0.5 + rnd() * 9.5;
        const ability = randn(rnd);
        const education = Math.round(
          Math.min(20, Math.max(8, 12 + 0.9 * (10 - distance) * 0.28 + 1.6 * ability + randn(rnd) * 0.8)),
        );
        const baseExperience = Math.max(1, Math.round(2 + rnd() * 30));
        const experience = baseExperience + (year - 2018); // grows each year
        const female = rnd() < 0.46 ? 1 : 0;
        const urban = rnd() < 0.62 ? 1 : 0;
        const post = year >= 2020 ? 1 : 0;
        const treated = firmTreated[f] * post; // DID: treatment active from 2020
        // Wage DGP: returns to education ~1.6, experience ~0.4 concave,
        // female gap −3.4, urban premium +1.6, firm productivity effect,
        // DID training effect +2.2, lognormal-ish noise.
        const noise = Math.exp(0.28 * randn(rnd)) - 1;
        const wage =
          4 +
          1.6 * education +
          0.42 * experience -
          0.006 * experience * experience -
          3.4 * female +
          1.6 * urban +
          5 * (firmProductivity[f] - 1) +
          2.2 * treated +
          6 * noise;
        const employed = 1 / (1 + Math.exp(-(-3.2 + 0.24 * education + 0.05 * experience - 0.4 * female + 0.8 * randn(rnd)))) > rnd() ? 1 : 0;
        const lambda = Math.exp(0.7 + 0.06 * (60 - Math.min(experience, 40)) / 3 + 0.08 * (wage > 30 ? 1 : 0));
        // Poisson count via inverse-CDF on the deterministic uniforms.
        let visits = 0;
        let cum = Math.exp(-lambda);
        let pv = rnd();
        while (pv > cum && visits < 30) {
          visits++;
          cum += (Math.exp(-lambda) * Math.pow(lambda, visits)) / factorial(visits);
        }
        numeric.id.push(workerId);
        numeric.year.push(year);
        numeric.firm.push(f + 1);
        numeric.wage.push(Math.round(wage * 100) / 100);
        numeric.education.push(education);
        numeric.experience.push(experience);
        numeric.female.push(female);
        numeric.urban.push(urban);
        numeric.distance.push(Math.round(distance * 10) / 10);
        numeric.employed.push(employed);
        numeric.visits.push(visits);
        numeric.treated.push(firmTreated[f]);
        numeric.post.push(post);
      }
    }
  }
  // Introduce a small, clearly-marked amount of missingness in wage.
  const wage = numeric.wage;
  for (let i = 0; i < wage.length; i++) {
    if (i % 137 === 13) wage[i] = null;
  }
  return { variables, numeric, text: {}, rows: wage.length, name: "wage_survey_synthetic.numeris-demo" };
}

function factorial(n: number): number {
  let f = 1;
  for (let i = 2; i <= n; i++) f *= i;
  return f;
}

/** RFC 4180 CSV parser (mirror of the Rust importer). */
export function parseCsv(text: string): Dataset {
  const delimiter = detectDelimiter(text);
  const rows = parseCsvRows(text, delimiter);
  if (rows.length === 0) throw new Error("The file contains no data rows.");
  const header = rows[0];
  const data = rows.slice(1);
  const names = header.map((h, i) => normalizeName(h, i));
  const isNumeric = new Array(header.length).fill(true) as boolean[];
  const values: (number | null)[][] = names.map(() => new Array(data.length).fill(null));
  const texts: (string | null)[][] = names.map(() => new Array(data.length).fill(null));
  for (let r = 0; r < data.length; r++) {
    for (let c = 0; c < names.length; c++) {
      const cell = data[r][c] ?? "";
      const t = cell.trim();
      if (isMissingToken(t)) continue;
      if (isNumeric[c]) {
        const v = Number(t);
        if (Number.isFinite(v) && !Number.isNaN(Number(t))) values[c][r] = v;
        else {
          isNumeric[c] = false;
          texts[c][r] = t;
        }
      } else texts[c][r] = t;
    }
  }
  const numeric: Record<string, (number | null)[]> = {};
  const textCols: Record<string, (string | null)[]> = {};
  const variables: VariableInfo[] = [];
  names.forEach((name, c) => {
    if (isNumeric[c]) {
      numeric[name] = values[c];
      const uniq = new Set(values[c].filter((v) => v !== null));
      variables.push({
        name,
        label: "",
        storage: "Numeric",
        semantic: uniq.size <= 10 ? "Categorical" : "Continuous",
      });
    } else {
      textCols[name] = texts[c];
      const uniq = new Set(texts[c].filter((v) => v !== null));
      variables.push({ name, label: "", storage: "Text", semantic: uniq.size <= 10 ? "Categorical" : "Identifier" });
    }
  });
  return { variables, numeric, text: textCols, rows: data.length, name: "imported.csv" };
}

function detectDelimiter(text: string): string {
  const line = text.split(/\r?\n/)[0] ?? "";
  const counts: Record<string, number> = { ",": (line.match(/,/g) ?? []).length, ";": (line.match(/;/g) ?? []).length, "\t": (line.match(/\t/g) ?? []).length };
  if (counts["\t"] > counts[","] && counts["\t"] > counts[";"]) return "\t";
  if (counts[";"] > counts[","]) return ";";
  return ",";
}

function parseCsvRows(text: string, delimiter: string): string[][] {
  const rows: string[][] = [];
  let row: string[] = [];
  let field = "";
  let inQuotes = false;
  let fieldStarted = false;
  const src = text.replace(/^\uFEFF/, "");
  for (let i = 0; i < src.length; i++) {
    const c = src[i];
    if (inQuotes) {
      if (c === '"') {
        if (src[i + 1] === '"') {
          field += '"';
          i++;
        } else inQuotes = false;
      } else field += c;
    } else if (c === '"' && field === "" && !fieldStarted) {
      inQuotes = true;
      fieldStarted = true;
    } else if (c === delimiter) {
      row.push(field);
      field = "";
      fieldStarted = false;
    } else if (c === "\n" || c === "\r") {
      if (c === "\r" && src[i + 1] === "\n") i++;
      row.push(field);
      field = "";
      fieldStarted = false;
      if (!(row.length === 1 && row[0] === "")) rows.push(row);
      row = [];
    } else {
      field += c;
      fieldStarted = true;
    }
  }
  if (field !== "" || fieldStarted) row.push(field);
  // An empty row or a single empty cell is a blank line artifact: skip it.
  if (row.length > 0 && !(row.length === 1 && row[0] === "")) rows.push(row);
  return rows;
}

function normalizeName(raw: string, i: number): string {
  let name = raw
    .trim()
    .toLowerCase()
    .replace(/[^a-z0-9_]+/g, "_")
    .replace(/_{2,}/g, "_");
  if (!name) name = `v${i + 1}`;
  if (/^[0-9]/.test(name)) name = `v${name}`;
  return name;
}

function isMissingToken(t: string): boolean {
  return ["", "na", "n/a", "nan", ".", "null", "none"].includes(t.toLowerCase());
}
