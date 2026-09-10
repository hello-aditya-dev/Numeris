# Numeris — Licensing & Activation Design

**Scope:** product licensing, activation, entitlements and device
management (engineering specification §24)
**Status:** design implemented as a **local stub** in `1.0.0-dev`
(offline development mode); server activation pending — see
`KNOWN_LIMITATIONS.md`

---

## 1. Principles

1. **No hostile DRM.** The activation system exists to reduce casual
   account/device sharing, not to make piracy mathematically impossible —
   desktop software executing on the customer's machine cannot be made
   perfectly unpatchable. The design goal is:
   > **Make legitimate ownership so inexpensive and frictionless that
   > piracy is unattractive.**
2. **Offline after activation.** Core statistics and research workflows
   continue offline once the product is activated. The license server may
   be contacted periodically for entitlement verification, but continuous
   internet access is never mandatory. A hard "restricted mode" after an
   arbitrary number of offline days is **not** part of the product
   philosophy and must not be introduced without a genuine licensing or
   security need.
3. **No subscription.** The $29 purchase is one-time for the 1.x series.
   There is no recurring requirement to retain access, no usage meter, and
   update wording never says "your subscription expired".
4. **Privacy.** The licensing service sees only what licensing requires —
   never research data (§30).

## 2. Commercial model

### First 1,000 verified users — free

"Download count" means **verified unique users**, not raw installer
downloads. Flow: email → verification → entitlement → download/activation.
The count is tracked server-side against verified email addresses.

### After user 1,000

- The **Free edition** remains available.
- The **Full edition** is a **$29 one-time purchase for the 1.x series**,
  unlocking the full research platform (advanced econometrics, causal
  inference, expanded graphics, publication reporting, the full
  reproducibility workflow). The exact feature boundary is frozen and
  documented publicly before launch.
- Possible future major-version upgrade pricing (2.0: $19–$29; 3.0: $19–$29)
  is a policy placeholder, not a contractual commitment.

### Payment rails

- **India:** Razorpay
- **International:** PayPal
- Voluntary support/donations use the same rails (supplementary, never
  required).

## 3. Device model

```text
1 verified email + 1 active device = 1 active license
```

- **Device transfer is a first-class flow.** A user can deactivate a device
  (Settings → License → Deactivate Device) and activate a new one.
  Deactivation releases the device slot server-side.
- Legitimate transfer must never require support contact.
- Re-activation after hardware changes (a new machine, a reinstalled OS)
  follows the same deactivate/reactivate path; a grace path handles the
  case where the old install is unreachable (server-side release of a
  stale device slot after verification).

## 4. Activation flow

```text
Email
  → verification        (code sent to the address; proves ownership)
  → entitlement          (free-tier / first-1000 / Full purchase record)
  → device registration (this device is bound to the license)
  → signed lease         (server issues an Ed25519-signed license lease)
  → encrypted local storage (lease stored encrypted; OS keychain where
                             available — Keychain on macOS, Credential
                             Manager on Windows, Secret Service on Linux)
  → offline verification (the app verifies the lease against the embedded
                         public key; no server round-trip needed to run)
```

### Lease contents

A compact JSON document signed with the license server's Ed25519 private
key:

- license id and edition (Free / Full / first-1000)
- verified email (or a salted hash of it)
- device identifier (stable, per-install, non-reversible)
- entitlement validity window and lease renewal timestamp
- product version scope (1.x)

### Cryptographic design

- **The application ships ONLY the Ed25519 public verification key.** The
  private signing key exists exclusively on the license server (and in a
  secure backup), is never committed to the repository, and is never
  embedded in any client build (spec §37: no hard-coded private keys).
- Verification is local: signature check against the embedded public key
  plus lease expiry/consistency checks. A forged or expired lease fails
  closed; an unreachable server never bricks an activated install (the
  offline principle above).

### Device identifiers

Generated per installation, stored alongside the lease; never derived from
personal data beyond the install itself, and never used for tracking.

## 5. What the license server sees — and never sees

The licensing service receives only:

- email / account identifier
- license id
- device activation identifier
- entitlement information
- timestamps required for licensing

It must never receive: datasets, variable names, commands, model
specifications, results, reports, project files, or any research content.
The client sends licensing payloads only; the statistical engine has no
network capability at all.

## 6. Offline behavior

- After activation, all statistical computation, projects and exports run
  fully offline.
- Periodic entitlement verification (e.g. at app start, or once every N
  days when a network happens to be available) is **best-effort**: failure
  to reach the server is logged quietly and changes nothing for a valid
  local lease.
- No arbitrary lockout. If a lease is genuinely revoked (refund fraud,
  chargeback), the server-side state is picked up on the next successful
  contact — never by disabling an offline working session mid-analysis.

## 7. Updater interaction

- Update checks are optional (Settings → Privacy: on/off).
- Update artifacts are signed with the Tauri Ed25519 updater key; the app
  verifies signatures before installing (spec §26).
- The private update key lives only in CI secrets
  (`.github/workflows/release.yml` documents `TAURI_SIGNING_PRIVATE_KEY` /
  `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`) and is backed up securely — losing
  it prevents publishing verifiable updates to installed clients.
- Update wording is calm and factual: "Numeris 1.2 is available."

## 8. UI requirements (design system §74)

License information is secondary to research: Settings/About shows the
product version, "Licensed to" email, the device name, and a Deactivate
Device button. No persistent subscription banners (there is no
subscription). Activation uses a sheet/modal; offline state is quiet
("Offline — core data analysis remains available").

## 9. Current implementation status (1.0.0-dev)

To keep the development preview usable before the license server exists:

- Activation runs against a **local stub** in offline development mode:
  the email/verification/entitlement steps complete locally, and a locally
  generated lease satisfies the same verification code path the real
  server lease will use.
- No payment rails are wired; no device registry exists server-side yet.
- The cryptographic envelope (Ed25519 public-key-only verification,
  encrypted local storage) is implemented per this document so the server
  integration is a deployment task, not a redesign.
- Before public release: stand up the license service, generate the
  production Ed25519 keypair, embed the production public key, configure
  Razorpay/PayPal, and verify device deactivation/transfer end-to-end
  (release checklist item).

## 10. Anti-piracy reality check

Desktop software that executes on the customer's machine cannot be made
perfectly unpatchable. Numeris invests in making the legitimate path
frictionless (cheap one-time price, instant activation, genuine offline
operation, device transfer without support tickets) rather than in hostile
DRM that would punish paying researchers — the people the product exists
for.
