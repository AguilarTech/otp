use std::time::Duration;

use oauth2::basic::BasicClient;
use oauth2::{
    AuthUrl, AuthorizationCode, ClientId, ClientSecret, CsrfToken, PkceCodeChallenge, RedirectUrl,
    Scope, TokenResponse, TokenUrl,
};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpListener;

use super::error::{Result, TransportError};

pub const AUTH_URL: &str = "https://accounts.google.com/o/oauth2/v2/auth";
pub const TOKEN_URL: &str = "https://oauth2.googleapis.com/token";
pub const DRIVE_FILE_SCOPE: &str = "https://www.googleapis.com/auth/drive.file";

pub struct OAuthTokens {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_in_secs: u64,
}

/// Run the loopback OAuth2 flow. Spins up a one-shot localhost listener,
/// opens the consent URL in the system browser, waits for the redirect
/// containing the authorization code, exchanges it for tokens (with PKCE),
/// and returns access + refresh tokens.
pub async fn run_loopback_flow(
    client_id: &str,
    client_secret: Option<&str>,
) -> Result<OAuthTokens> {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let port = listener
        .local_addr()
        .map_err(TransportError::Io)?
        .port();
    let redirect_uri = format!("http://127.0.0.1:{}/", port);

    let mut client_builder = BasicClient::new(ClientId::new(client_id.to_string()))
        .set_auth_uri(AuthUrl::new(AUTH_URL.to_string())?)
        .set_token_uri(TokenUrl::new(TOKEN_URL.to_string())?)
        .set_redirect_uri(RedirectUrl::new(redirect_uri.clone())?);
    if let Some(secret) = client_secret {
        client_builder = client_builder.set_client_secret(ClientSecret::new(secret.to_string()));
    }
    let client = client_builder;

    let (pkce_challenge, pkce_verifier) = PkceCodeChallenge::new_random_sha256();

    let (auth_url, csrf_token) = client
        .authorize_url(CsrfToken::new_random)
        .add_scope(Scope::new(DRIVE_FILE_SCOPE.to_string()))
        .add_extra_param("access_type", "offline")
        .add_extra_param("prompt", "consent")
        .set_pkce_challenge(pkce_challenge)
        .url();

    webbrowser::open(auth_url.as_str())
        .map_err(|e| TransportError::OAuth(format!("could not open browser: {}", e)))?;

    let (code, state) = tokio::time::timeout(
        Duration::from_secs(300),
        wait_for_redirect(listener),
    )
    .await
    .map_err(|_| TransportError::OAuth("timed out waiting for browser redirect".into()))??;

    if state != *csrf_token.secret() {
        return Err(TransportError::OAuth(
            "CSRF token mismatch — possible authorization endpoint tampering".into(),
        ));
    }

    let http = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(TransportError::from)?;

    let token_result = client
        .exchange_code(AuthorizationCode::new(code))
        .set_pkce_verifier(pkce_verifier)
        .request_async(&http)
        .await
        .map_err(|e| TransportError::OAuth(format!("token exchange failed: {}", e)))?;

    let access_token = token_result.access_token().secret().clone();
    let refresh_token = token_result
        .refresh_token()
        .ok_or_else(|| {
            TransportError::OAuth(
                "Google did not return a refresh token; re-authorize from a fresh consent screen"
                    .into(),
            )
        })?
        .secret()
        .clone();
    let expires_in_secs = token_result
        .expires_in()
        .map(|d| d.as_secs())
        .unwrap_or(3600);

    Ok(OAuthTokens {
        access_token,
        refresh_token,
        expires_in_secs,
    })
}

async fn wait_for_redirect(listener: TcpListener) -> Result<(String, String)> {
    let (mut stream, _) = listener.accept().await?;
    let (read_half, mut write_half) = stream.split();
    let mut reader = BufReader::new(read_half);
    let mut request_line = String::new();
    reader.read_line(&mut request_line).await?;

    // Drain remaining headers so the browser sees a complete response.
    let mut tmp = String::new();
    loop {
        tmp.clear();
        let n = reader.read_line(&mut tmp).await?;
        if n == 0 || tmp == "\r\n" || tmp == "\n" {
            break;
        }
    }

    let (code, state) = parse_callback_query(&request_line)?;

    let body = b"<!doctype html><html><body style=\"font-family:sans-serif;padding:40px;background:#1e1e1e;color:#eee\"><h2>OTP Messenger</h2><p>Authorization complete. You can close this tab and return to the app.</p></body></html>";
    let response = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    );
    write_half.write_all(response.as_bytes()).await?;
    write_half.write_all(body).await?;
    write_half.flush().await?;

    Ok((code, state))
}

fn parse_callback_query(request_line: &str) -> Result<(String, String)> {
    // request_line: "GET /?code=...&state=... HTTP/1.1\r\n"
    let mut parts = request_line.split_whitespace();
    let _method = parts.next();
    let path = parts
        .next()
        .ok_or_else(|| TransportError::OAuth("malformed redirect request".into()))?;
    let url = url::Url::parse(&format!("http://127.0.0.1{}", path))?;

    if let Some(error) = url.query_pairs().find(|(k, _)| k == "error").map(|(_, v)| v) {
        return Err(TransportError::OAuth(format!(
            "Google returned error: {}",
            error
        )));
    }

    let code = url
        .query_pairs()
        .find(|(k, _)| k == "code")
        .map(|(_, v)| v.into_owned())
        .ok_or_else(|| TransportError::OAuth("no code in redirect".into()))?;
    let state = url
        .query_pairs()
        .find(|(k, _)| k == "state")
        .map(|(_, v)| v.into_owned())
        .ok_or_else(|| TransportError::OAuth("no state in redirect".into()))?;
    Ok((code, state))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_callback_query() {
        let line = "GET /?code=ABC&state=XYZ HTTP/1.1\r\n";
        let (code, state) = parse_callback_query(line).unwrap();
        assert_eq!(code, "ABC");
        assert_eq!(state, "XYZ");
    }

    #[test]
    fn surfaces_oauth_error_param() {
        let line = "GET /?error=access_denied&state=XYZ HTTP/1.1\r\n";
        let err = parse_callback_query(line).unwrap_err();
        assert!(matches!(err, TransportError::OAuth(_)));
    }
}
