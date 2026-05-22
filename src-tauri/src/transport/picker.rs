use std::time::Duration;

use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpListener;

use super::error::{Result, TransportError};

const PICKER_HTML: &str = include_str!("picker.html");

pub struct PickerInputs<'a> {
    pub access_token: &'a str,
    pub api_key: &'a str,
    pub app_id: &'a str,
}

/// Spin up a one-shot loopback HTTP server, serve the embedded Picker
/// page on the random localhost port, open it in the system browser,
/// and wait for the page to POST the picked folder id back. Returns
/// the picked id, or an error if the user cancels or the timeout
/// elapses.
pub async fn run_picker_flow(inputs: PickerInputs<'_>) -> Result<String> {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let port = listener.local_addr()?.port();

    let fragment = url::form_urlencoded::Serializer::new(String::new())
        .append_pair("access_token", inputs.access_token)
        .append_pair("api_key", inputs.api_key)
        .append_pair("app_id", inputs.app_id)
        .finish();
    let url = format!("http://127.0.0.1:{}/#{}", port, fragment);

    webbrowser::open(&url)
        .map_err(|e| TransportError::OAuth(format!("could not open browser: {}", e)))?;

    tokio::time::timeout(Duration::from_secs(300), serve_until_done(listener))
        .await
        .map_err(|_| TransportError::OAuth("timed out waiting for picker selection".into()))?
}

#[derive(Debug)]
enum PickerOutcome {
    Picked(String),
    Cancelled,
    ContinueServing,
}

async fn serve_until_done(listener: TcpListener) -> Result<String> {
    loop {
        let (mut stream, _) = listener.accept().await?;
        let (read_half, mut write_half) = stream.split();
        let mut reader = BufReader::new(read_half);
        let mut request_line = String::new();
        reader.read_line(&mut request_line).await?;

        let mut tmp = String::new();
        loop {
            tmp.clear();
            let n = reader.read_line(&mut tmp).await?;
            if n == 0 || tmp == "\r\n" || tmp == "\n" {
                break;
            }
        }

        let path = request_line
            .split_whitespace()
            .nth(1)
            .unwrap_or("/")
            .to_string();

        let outcome = if path.starts_with("/result") {
            match extract_folder_id(&path) {
                Ok(id) => PickerOutcome::Picked(id),
                Err(_) => PickerOutcome::ContinueServing,
            }
        } else if path.starts_with("/cancel") {
            PickerOutcome::Cancelled
        } else {
            PickerOutcome::ContinueServing
        };

        match outcome {
            PickerOutcome::Picked(id) => {
                write_text_response(&mut write_half, "OK").await?;
                return Ok(id);
            }
            PickerOutcome::Cancelled => {
                write_text_response(&mut write_half, "Cancelled").await?;
                return Err(TransportError::OAuth("picker cancelled by user".into()));
            }
            PickerOutcome::ContinueServing => {
                if path == "/favicon.ico" {
                    write_404(&mut write_half).await?;
                } else {
                    write_html_response(&mut write_half, PICKER_HTML).await?;
                }
            }
        }
    }
}

fn extract_folder_id(path: &str) -> Result<String> {
    let url = url::Url::parse(&format!("http://127.0.0.1{}", path))?;
    url.query_pairs()
        .find(|(k, _)| k == "folder_id")
        .map(|(_, v)| v.into_owned())
        .ok_or_else(|| TransportError::OAuth("picker result missing folder_id".into()))
}

async fn write_html_response<W>(w: &mut W, body: &str) -> Result<()>
where
    W: tokio::io::AsyncWrite + Unpin,
{
    let head = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    );
    w.write_all(head.as_bytes()).await?;
    w.write_all(body.as_bytes()).await?;
    w.flush().await?;
    Ok(())
}

async fn write_text_response<W>(w: &mut W, body: &str) -> Result<()>
where
    W: tokio::io::AsyncWrite + Unpin,
{
    let head = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: text/plain; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    );
    w.write_all(head.as_bytes()).await?;
    w.write_all(body.as_bytes()).await?;
    w.flush().await?;
    Ok(())
}

async fn write_404<W>(w: &mut W) -> Result<()>
where
    W: tokio::io::AsyncWrite + Unpin,
{
    let head = "HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n";
    w.write_all(head.as_bytes()).await?;
    w.flush().await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_folder_id_from_path() {
        let id = extract_folder_id("/result?folder_id=ABC123").unwrap();
        assert_eq!(id, "ABC123");
    }

    #[test]
    fn extract_folder_id_rejects_missing_param() {
        let err = extract_folder_id("/result?wrong=value").unwrap_err();
        assert!(matches!(err, TransportError::OAuth(_)));
    }

    #[test]
    fn extract_folder_id_url_decodes() {
        let id = extract_folder_id("/result?folder_id=abc%2Bdef").unwrap();
        assert_eq!(id, "abc+def");
    }
}
