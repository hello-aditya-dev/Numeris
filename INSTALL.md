# Installing Numeris

Numeris runs on Windows, macOS and Linux. Installers are published on the
[GitHub Releases](https://github.com/hello-aditya-dev/Numeris/releases) page.
Download the artifact for your platform and follow the steps below.

> Numeris is local-first: after installation, all statistical computation,
> projects and data stay on your machine. Internet access is only used for
> license activation and optional update checks.

## Windows (10/11, x64)

1. Download the `.msi` installer (or the NSIS `.exe` setup) for the release.
2. Run the installer. If the system WebView2 runtime is missing, the
   installer retrieves it automatically (Windows 11 ships it by default).
3. Launch **Numeris** from the Start menu.

**SmartScreen note:** current development builds are unsigned, so Windows
SmartScreen may show "Windows protected your PC" on first run. Choose
*More info → Run anyway*. Signed installers are planned before the public
release; note that SmartScreen reputation builds over time even for signed
applications.

## macOS (Apple silicon and Intel)

1. Download the DMG matching your Mac:
   - **arm64** — Apple silicon (M1/M2/M3/M4)
   - **x64** — Intel Macs
2. Open the DMG and drag **Numeris** into *Applications*.
3. Launch Numeris from *Applications*.

**Gatekeeper note:** current development builds are not code-signed or
notarized, so macOS will block the first launch with
"*Numeris* can't be opened because it is from an unidentified developer."
To open it: right-click (or Control-click) the app → **Open** → **Open** in
the confirmation dialog — this is remembered for subsequent launches.
Alternatively, from a terminal:

```bash
xattr -d com.apple.quarantine /Applications/Numeris.app
```

Developer ID signing and notarization are required before the public release
and will remove this friction.

## Linux

**AppImage** (works on most distributions):

```bash
chmod +x Numeris_<version>_amd64.AppImage
./Numeris_<version>_amd64.AppImage
```

**deb** (Debian, Ubuntu, Mint):

```bash
sudo apt install ./numeris_<version>_amd64.deb
```

The desktop shell uses WebKitGTK; on minimal installations ensure the
runtime libraries are present (`libwebkit2gtk-4.1-0`, `libayatana-appindicator3-1`,
`librsvg2-2` on Debian/Ubuntu).

## Build from source (quick path)

Prerequisites: [Rust](https://rustup.rs) (stable), [Bun](https://bun.sh) ≥ 1.1
(or Node ≥ 20), plus the Tauri 2 platform dependencies for your OS.

```bash
git clone https://github.com/hello-aditya-dev/Numeris.git
cd Numeris

# 1. run the engine test suite (includes NIST StRD validation)
cargo test --all

# 2. install frontend dependencies
cd apps/desktop
bun install            # or: npm install

# 3. run the desktop app
bun run tauri dev     # or: npm run tauri dev

# 4. build a production bundle
bun run tauri build
```

Installers land in `apps/desktop/src-tauri/target/release/bundle/`
(MSI/NSIS on Windows, DMG on macOS, AppImage/deb on Linux).

See `docs/DEVELOPER_GUIDE.md` for the full development setup.

## First-run activation

On first launch, Numeris asks you to activate. The flow is:

```text
Email
  → verification (a code is sent to your address)
  → entitlement  (free for the first 1,000 verified users; $29 one-time
                  Full license for the 1.x series afterwards)
  → device registration
  → activation
```

- Enter your email, request a verification code, and enter the code.
- Your entitlement is verified and this device is registered to your
  license (one verified email + one active device = one active license;
  devices can be deactivated and transferred).
- After activation, **Numeris works fully offline** — core statistics,
  projects, and all research workflows run without internet access.
  License verification may be contacted periodically but never blocks
  analysis.

**Status in the current development preview (`1.0.0-dev`):** the licensing
service runs as a local stub in offline development mode — activation
completes without contacting a server, and server-backed activation,
payment processing and device management arrive before public release (see
`KNOWN_LIMITATIONS.md` and `security/LICENSING_DESIGN.md`).

## Updating

Numeris checks for updates only if you leave update checks enabled in
Settings → Privacy (off/on). Updates are cryptographically signed; the app
verifies every signature before installing. Update notifications read
"Numeris 1.2 is available" — never subscription-pressure wording (there is
no subscription).

## Uninstalling

- **Windows:** Settings → Apps → Numeris → Uninstall (MSI/NSIS entry).
- **macOS:** drag Numeris from *Applications* to the Trash.
- **Linux:** remove the AppImage file, or `sudo apt remove numeris` for the
  deb package.

Projects and datasets are ordinary files on disk (`.numeris` directories)
and are never deleted by uninstalling — remove them manually if you wish.

## Troubleshooting

- **The app won't launch (Linux):** install the WebKitGTK runtime packages
  listed above.
- **Blank window:** ensure your GPU drivers are current; the desktop shell
  uses your system webview.
- **Bug reports:** open an issue at
  [github.com/hello-aditya-dev/Numeris/issues](https://github.com/hello-aditya-dev/Numeris/issues)
  including the Numeris version, OS, architecture, the command or
  specification that failed, and the exact error message. Research data is
  **never** attached automatically.
