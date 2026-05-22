# Google Cloud Setup

OTP Messenger uses Google Drive as the transport layer for ciphertext blobs.
The app needs OAuth2 client credentials embedded at build time so it can ask
each user for permission to access their Drive. This is a one-time setup
performed by whoever produces the binaries.

## What you'll end up with

Two strings — a **client ID** and a **client secret** — that get exported as
environment variables before `cargo build` / `tauri build`:

```
export OTP_GOOGLE_CLIENT_ID='123456789-abc.apps.googleusercontent.com'
export OTP_GOOGLE_CLIENT_SECRET='GOCSPX-...'
```

If these are missing at build time the app still runs, but every Drive-related
command returns `OAuth client not configured`. The USB + manual-paste flow
keeps working without them.

## Steps

1. **Create a Google Cloud project.**  
   <https://console.cloud.google.com/projectcreate>  
   Name it something memorable (e.g. `otp-messenger`). Pick "No organization"
   for a personal project.

2. **Enable the Google Drive API.**  
   APIs & Services → Enabled APIs & Services → ENABLE APIS AND SERVICES →
   search "Google Drive API" → Enable.

3. **Configure the OAuth consent screen.**  
   APIs & Services → OAuth consent screen.
   - User type: **External** (unless you're on Google Workspace and want to
     restrict to your domain).
   - App name: `OTP Messenger` (or your preference).
   - User support email: your address.
   - Developer contact: your address.
   - **Scopes**: add `.../auth/drive.file` — the *narrow* scope that limits
     the app to files it created or that the user explicitly granted via
     Google Picker. Do **not** add `drive` or `drive.readonly`.
   - **Test users**: add the Google accounts that will run the app while the
     project stays in "Testing" mode. Including
     `daniel@aguilartech.com.au` and any peers you plan to message. You can
     skip the verification submission as long as you stay under 100 test
     users.

4. **Create OAuth client credentials.**  
   APIs & Services → Credentials → CREATE CREDENTIALS → OAuth client ID.
   - Application type: **Desktop app**.
   - Name: anything, e.g. `OTP Messenger desktop`.
   - Click Create. Google shows the client ID and client secret. Copy both
     somewhere safe (you can also re-download the JSON from the credentials
     list at any time).

5. **Export them before building.**  
   ```bash
   export OTP_GOOGLE_CLIENT_ID='paste-client-id'
   export OTP_GOOGLE_CLIENT_SECRET='paste-client-secret'
   cargo build --manifest-path src-tauri/Cargo.toml
   # or:
   npm run tauri dev
   ```

   The values bake into the binary via `option_env!`. They are not secret
   in the cryptographic sense — Google explicitly considers desktop OAuth
   client secrets "non-confidential" because they end up shipped to every
   user — but anyone with the values can impersonate the app's identity to
   Google's consent screen. Treat them as moderately sensitive.

## How the flow works at runtime

1. User clicks **Connect Google Drive** in the Settings view.
2. App binds a localhost port (random) as a one-shot HTTP listener.
3. App opens the user's default browser to Google's consent screen, with
   `redirect_uri=http://127.0.0.1:<port>/`.
4. User signs in and approves `drive.file` access.
5. Google redirects the browser to the loopback URL with `?code=...`.
6. App's listener captures the code, closes the listener, exchanges the
   code (with PKCE) for access + refresh tokens.
7. Refresh token is stored in the OS keychain (`com.aguilartech.otp` /
   `google_drive_refresh_token`):
   - macOS: Keychain.
   - Linux: Secret Service (gnome-keyring / kwallet) — install
     `libsecret-1-dev` and run a keyring daemon.
   - Windows: Credential Manager.
8. Subsequent runs read the refresh token at startup; access tokens are
   minted on demand and cached for their lifetime (≈1 hour).

## Known limitation: drive.file + cross-account folders

`drive.file` grants the app access to files it created via the API or that
the user explicitly handed over via Google Picker. A folder created by
account A and shared with account B is **invisible** to account B's app
until B claims it via Picker.

Until Picker integration ships (Phase 4b), the practical paths are:

- Use the **same Google account** on both peers (e.g. one personal account
  for testing).
- Or use the **manual paste** flow (Send button still emits the base64
  frame; peers shuttle it via any out-of-band channel).
