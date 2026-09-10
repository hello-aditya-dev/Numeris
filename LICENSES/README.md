# LICENSES — Numeris license audit trail

This directory is the license audit trail required by the Numeris engineering
specification (§5.1). Together with `THIRD-PARTY-NOTICES.txt` (human-readable
attribution) and `DEPENDENCY-MANIFEST.json` (machine-readable inventory), it
records every license that applies to the software distributed with Numeris.

## Contents

| File | Purpose |
|---|---|
| `MIT.txt` | Full text of the MIT License, as applied by MIT-licensed third-party components |
| `Apache-2.0.txt` | Full text of the Apache License 2.0, as applied by Apache-2.0-licensed third-party components |

## How the trail works

- Every production dependency is checked at the exact version used, and its
  license and notice obligations are recorded in `THIRD-PARTY-NOTICES.txt`.
- The machine-readable inventory (`DEPENDENCY-MANIFEST.json`) lists every
  component shipped in a Numeris build together with its license, source and
  usage.
- Components dual-licensed `MIT OR Apache-2.0` (serde, serde_json, tauri,
  tauri-build, @tauri-apps/api, @tauri-apps/cli) satisfy their license terms by
  compliance with either license; Numeris includes the notice text of both in
  the distribution to keep the audit trail unambiguous.
- The copyright holders of MIT- and Apache-2.0-licensed components are listed
  individually in `THIRD-PARTY-NOTICES.txt`; this file and `MIT.txt` carry the
  aggregated attribution.
- Numeris itself (crates `numeris-core`, `numeris-stats`, `numeris-command`,
  `numeris-project` and the desktop application) is distributed under the
  Numeris Software License — see the `LICENSE` file at the repository root.

## Policy

- "Open source" is not treated as equivalent to "commercially unrestricted."
  Every production dependency must be checked at the exact version used, with
  its license and notice obligations recorded (spec §5.1).
- No copyleft (GPL/AGPL/LGPL/SSPL) dependency may enter the build graph. All
  current dependencies are permissively licensed (MIT, Apache-2.0, or
  MIT OR Apache-2.0).
- Where licensing is unclear at the exact version in use, the dependency must
  not be bundled until reviewed.
- When a dependency is added, removed or upgraded, all three artifacts
  (`THIRD-PARTY-NOTICES.txt`, `DEPENDENCY-MANIFEST.json`, this directory) must
  be updated in the same change.
