# Google Cloud Setup

OTP Messenger uses Google Drive as the transport layer for ciphertext blobs.
The app needs OAuth2 client credentials embedded at build time so it can ask
each user for permission to access their Drive. This is a one-time setup
performed by whoever produces the binaries.

## What you'll end up with

Three strings — a **client ID**, a **client secret**, and an **API key** —
that get exported as environment variables before `cargo build` / `tauri
build`:

```
export OTP_GOOGLE_CLIENT_ID='123456789-abc.apps.googleusercontent.com'
export OTP_GOOGLE_CLIENT_SECRET='GOCSPX-...'
export OTP_GOOGLE_API_KEY='AIza...'
```

- Client ID / secret: required for OAuth (Connect Google Drive in Settings).
- API key: required only for Google Picker (cross-account folder claiming).

If any are missing at build time the app still runs; the affected features
just stay disabled and surface a banner pointing at this file. The USB +
manual-paste flow keeps working with zero credentials.

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

5. **Create a Google API key for Picker.**  
   APIs & Services → Credentials → CREATE CREDENTIALS → API key.
   - Copy the generated key.
   - Click "Edit API key" to restrict it:
     - **API restrictions**: select "Restrict key" → check **Google Picker
       API** (you may need to enable Picker API first via Enabled APIs &
       Services → ENABLE APIS AND SERVICES → "Google Picker API").
     - **Application restrictions**: leave as "None" for a desktop app;
       Picker validates via the OAuth token rather than referer.

6. **Enable the Picker API.**  
   APIs & Services → Library → "Google Picker API" → Enable. (Skip if you
   already enabled it via the restriction dialog above.)

7. **Export them before building.**  
   ```bash
   export OTP_GOOGLE_CLIENT_ID='paste-client-id'
   export OTP_GOOGLE_CLIENT_SECRET='paste-client-secret'
   export OTP_GOOGLE_API_KEY='paste-api-key'
   cargo build --manifest-path src-tauri/Cargo.toml
   # or:
   npm run tauri dev
   ```

   The values bake into the binary via `option_env!`. They are not secret
   in the cryptographic sense — Google explicitly considers desktop OAuth
   client secrets "non-confidential" because they end up shipped to every
   user — but anyone with the values can impersonate the app's identity to
   Google's consent screen. Treat them as moderately sensitive.

   The Picker `appId` is derived at runtime from the first segment of the
   client ID (which is the Cloud project number for desktop OAuth
   clients), so no separate env var is needed.

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

## Cross-account folder claiming via Picker

`drive.file` grants the app access to files it created via the API or that
the user explicitly handed over via Google Picker. A folder created by
account A and shared with account B is **invisible** to account B's app
until B claims it via Picker.

The flow when both peers are on different Google accounts:

1. Creator clicks **Create** in the conversation's Drive section and enters
   the peer's Google email. The app creates a folder, shares it (writer
   role) with the peer, and Drive emails the peer an invite.
2. Peer opens the email or `drive.google.com` → Shared with me. They don't
   need to do anything in Drive itself, just confirm they can see it.
3. Peer goes to the imported pairing's conversation in OTP Messenger,
   clicks **Pick from Google Drive**. A browser tab opens with Google
   Picker pre-authenticated via the OAuth token; peer selects the shared
   folder under "Shared with me" and the tab posts the folder id back to
   the app.
4. App verifies it can now see the folder under `drive.file` and binds it
   to the pairing. Background polling begins.

The Picker page itself is served from a one-shot loopback HTTP listener
(`http://127.0.0.1:<random_port>/`) so Google's origin check sees a
standard http origin rather than Tauri's `tauri://localhost`.
