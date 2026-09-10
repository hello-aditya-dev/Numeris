import { fitOls } from "/home/z/my-project/src/lib/numeris/stats";
import { parseCsv } from "/home/z/my-project/src/lib/numeris/dataset";
import { readFileSync } from "fs";

const csv = readFileSync("/home/z/my-project/numeris/validation/golden/longley.csv", "utf8");
const ds = parseCsv(csv);
const names = ["defl", "gnp", "unemp", "armed", "pop", "year"];
const y: number[] = [];
const x: number[][] = [];
for (let i = 0; i < ds.rows; i++) {
  y.push(ds.numeric["total"][i] as number);
  x.push(names.map((n) => ds.numeric[n][i] as number));
}
const reg = fitOls({ y, x, names, vcov: { type: "classical" } });

const cert = {
  b: [-3482258.63459582, 15.0618722713733, -0.0358191792925910, -2.02022980381683, -1.03322686717359, -0.0511041056535807, 1829.15146461355],
  se: [890420.383607373, 84.9149257747669, 0.033491007722432, 0.488399681651699, 0.214274163161675, 0.226073200069370, 455.478499142212],
};
let ok = true;
const keys = ["b0", "b1", "b2", "b3", "b4", "b5", "b6"];
keys.forEach((k, j) => {
  const rel = Math.abs(reg.coef[j] - cert.b[j]) / Math.abs(cert.b[j]);
  if (rel > 5e-7) { ok = false; console.log(`FAIL ${k}: ${reg.coef[j]} vs ${cert.b[j]} rel=${rel.toExponential(2)}`); }
  else console.log(`ok  ${k}: ${reg.coef[j].toPrecision(10)} (rel err ${rel.toExponential(1)})`);
});
keys.forEach((k, j) => {
  const rel = Math.abs(reg.se[j] - cert.se[j]) / Math.abs(cert.se[j]);
  if (rel > 5e-5) { ok = false; console.log(`FAIL ${k} SE: ${reg.se[j]} vs ${cert.se[j]} rel=${rel.toExponential(2)}`); }
});
console.log("R² =", reg.r2, "vs 0.995479004577296");
console.log("RMSE =", reg.rmse, "vs 304.854073561965");
console.log(ok && Math.abs(reg.r2 - 0.995479004577296) < 1e-10 ? "NIST LONGLEY: PASS" : "NIST LONGLEY: FAIL");

// Distribution reference values.
import { tQuantile, normalCdf, chi2Quantile } from "/home/z/my-project/src/lib/numeris/dist";
const checks: [string, boolean][] = [
  ["t(.975,10)=2.228139", Math.abs(tQuantile(0.975, 10) - 2.228138852) < 1e-6],
  ["t(.975,30)=2.042272", Math.abs(tQuantile(0.975, 30) - 2.042272456) < 1e-6],
  ["Phi(1.96)=.975", Math.abs(normalCdf(1.959963984540054) - 0.975) < 1e-12],
  ["chi2(.95,10)=18.307", Math.abs(chi2Quantile(0.95, 10) - 18.307038) < 1e-5],
];
for (const [name, pass] of checks) console.log((pass ? "ok  " : "FAIL") + " " + name);
