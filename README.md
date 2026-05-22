# OTP Messenger

A cross-platform desktop messenger for 1-to-1 encrypted communication using
**one-time pads**, built with Tauri 2 and Vue 3.

## Security model

- **Confidentiality**: XOR with a pre-shared random pad. Information-theoretic
  security as long as pad material is not reused.
- **Authentication**: per-message Poly1305 one-time MAC. The 32-byte key for
  each message is drawn from the same outgoing pad immediately before that
  message's keystream. MAC covers `header || ciphertext` (encrypt-then-MAC),
  so an attacker cannot rebind a frame to a different offset, sequence
  number, or length.
- **Pad accounting**: a single atomic-write chokepoint advances the
  consumption cursor on disk before the consumed bytes are best-effort
  zeroized in place. A crash can waste pad bytes but cannot cause reuse.
- **Replay protection**: strict monotonic sequence numbers per stream;
  out-of-order delivery is accepted but consumes the skipped range, and a
  later message arriving after consumption is rejected.
- **Plaintext lifetime**: lives only in memory until the user dismisses
  each message. No persistent history.

Known limits the spec acknowledges:
- Erasure is best-effort on modern filesystems; copy-on-write systems
  (APFS, btrfs, ZFS) may retain older versions of pad bytes in unmapped
  blocks until snapshot expiry.
- Drive sees ciphertext blob sizes, timing, and account associations.
- Webview plaintext can't be cryptographically zeroized — it sits in JS
  engine memory until garbage collection reclaims it after dismiss.

## How it works

1. **Pairings**. Each pair of peers shares two random pads, one per
   direction. The **New pairing** flow generates them locally with `OsRng`,
   streams identical bytes to the local app data dir and a USB folder in
   one pass, and writes a small sidecar (`pairing.toml`) describing the
   pairing id and originator label. The peer plugs the USB into their
   machine and runs **Import pairing**, which copies the pads with roles
   swapped.

2. **Transport**. Ciphertext blobs travel through Google Drive. Each
   pairing gets its own folder. The creator's app creates the folder and
   shares it with the peer's email; the peer's app claims access via
   Google Picker (the only way `drive.file` scope can see a folder owned
   by another account). A background tokio task polls every 30 seconds,
   verifies each blob via the vault, and emits an inbox event to the UI on
   success. Verified blobs are deleted from Drive.

3. **Manual fallback**. If Drive isn't connected (or for cross-account
   pairs before Picker is wired up), `Encrypt` still produces a base64
   frame the user can copy and paste over any out-of-band channel. The
   receive box on the conversation view accepts the same format.

## Running

```bash
npm install
npm run tauri dev
```

For the Drive transport you also need to:

1. Work through [`CLOUD_SETUP.md`](CLOUD_SETUP.md) to produce Google Cloud
   OAuth credentials and a Google Picker API key.
2. Export `OTP_GOOGLE_CLIENT_ID`, `OTP_GOOGLE_CLIENT_SECRET`, and
   `OTP_GOOGLE_API_KEY` before building. Without them the Drive features
   stay disabled but the manual paste flow remains usable.
3. On Linux, install a Secret Service backend (gnome-keyring or kwallet)
   for the OAuth refresh-token storage. The keyring crate auto-detects.

## Tests

```bash
cd src-tauri && cargo test
```

22 tests cover the crypto round-trip, MAC tamper rejection, replay
rejection, out-of-order delivery, pad exhaustion, on-disk zeroization,
state persistence across vault reopens, sidecar version checks, frame
layout, plus the Drive multipart body, OAuth callback parser, and Picker
URL parser.

Network paths (OAuth, Drive API, Picker) are not covered by automated
tests; verify manually after wiring up Cloud credentials.

## Source layout

```
src-tauri/
├── src/
│   ├── main.rs                     Tauri command surface
│   ├── vault/                      OTP crypto + pad state
│   │   ├── crypto.rs               Poly1305 + XOR helpers
│   │   ├── frame.rs                Wire format (header + MAC)
│   │   ├── pad.rs                  Pad I/O + best-effort zeroize
│   │   ├── state.rs                pairings.toml + atomic write
│   │   ├── error.rs                VaultError
│   │   ├── mod.rs                  Vault struct, create/import flows
│   │   └── tests.rs                Unit tests
│   └── transport/                  Google Drive transport
│       ├── oauth.rs                Loopback OAuth2 (PKCE)
│       ├── drive.rs                Drive REST client
│       ├── picker.rs               Loopback-served Google Picker
│       ├── picker.html             Picker SDK page (embedded)
│       ├── poller.rs               30s polling task → inbox events
│       ├── error.rs                TransportError
│       └── mod.rs                  Credentials + refresh-token cache
└── capabilities/default.json       Tauri 2 capability scope

src/
├── App.vue                         View switcher
└── views/
    ├── PairingList.vue
    ├── CreatePairing.vue
    ├── ImportPairing.vue
    ├── Conversation.vue
    └── Settings.vue
```
