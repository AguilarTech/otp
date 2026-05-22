use std::sync::Arc;

use serde::Deserialize;
use serde_json::json;

use super::error::{Result, TransportError};
use super::Transport;

const DRIVE_API: &str = "https://www.googleapis.com/drive/v3";
const DRIVE_UPLOAD: &str = "https://www.googleapis.com/upload/drive/v3/files";
const FOLDER_MIME: &str = "application/vnd.google-apps.folder";

#[derive(Debug, Clone, Deserialize)]
pub struct DriveFile {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Deserialize)]
struct ListResponse {
    #[serde(default)]
    files: Vec<DriveFile>,
}

#[derive(Debug, Deserialize)]
struct IdResponse {
    id: String,
}

pub struct DriveClient {
    transport: Arc<Transport>,
    http: reqwest::Client,
}

impl DriveClient {
    pub fn new(transport: Arc<Transport>) -> Self {
        let http = reqwest::Client::builder()
            .build()
            .expect("reqwest client construction");
        Self { transport, http }
    }

    async fn token(&self) -> Result<String> {
        self.transport.access_token().await
    }

    async fn check_status(resp: reqwest::Response) -> Result<reqwest::Response> {
        let status = resp.status();
        if status.is_success() {
            return Ok(resp);
        }
        let code = status.as_u16();
        let body = resp.text().await.unwrap_or_default();
        Err(TransportError::DriveApi { status: code, body })
    }

    /// Create a folder at the root of the OAuth account's Drive.
    pub async fn create_folder(&self, name: &str) -> Result<String> {
        self.create_folder_impl(name, None).await
    }

    /// Create a folder inside an existing parent folder.
    pub async fn create_subfolder(
        &self,
        name: &str,
        parent_folder_id: &str,
    ) -> Result<String> {
        self.create_folder_impl(name, Some(parent_folder_id)).await
    }

    async fn create_folder_impl(
        &self,
        name: &str,
        parent_folder_id: Option<&str>,
    ) -> Result<String> {
        let token = self.token().await?;
        let mut body = json!({
            "name": name,
            "mimeType": FOLDER_MIME,
        });
        if let Some(parent) = parent_folder_id {
            body["parents"] = json!([parent]);
        }
        let resp = self
            .http
            .post(format!("{}/files", DRIVE_API))
            .bearer_auth(&token)
            .json(&body)
            .send()
            .await?;
        let resp = Self::check_status(resp).await?;
        let id: IdResponse = resp.json().await?;
        Ok(id.id)
    }

    /// Look up an existing folder at the Drive root by exact name, returning
    /// the first match. drive.file scope means we only see folders this app
    /// created — perfect for finding our own app-root container without
    /// stepping on anything else in the user's Drive.
    pub async fn find_root_folder_by_name(
        &self,
        name: &str,
    ) -> Result<Option<String>> {
        let token = self.token().await?;
        let q = format!(
            "name = '{}' and mimeType = '{}' and 'root' in parents and trashed = false",
            name.replace('\'', "\\'"),
            FOLDER_MIME,
        );
        let resp = self
            .http
            .get(format!("{}/files", DRIVE_API))
            .bearer_auth(&token)
            .query(&[
                ("q", q.as_str()),
                ("fields", "files(id,name)"),
                ("pageSize", "1"),
            ])
            .send()
            .await?;
        let resp = Self::check_status(resp).await?;
        let parsed: ListResponse = resp.json().await?;
        Ok(parsed.files.into_iter().next().map(|f| f.id))
    }

    /// Find-or-create the named folder at the Drive root, returning its id.
    pub async fn ensure_root_folder(&self, name: &str) -> Result<String> {
        if let Some(id) = self.find_root_folder_by_name(name).await? {
            return Ok(id);
        }
        self.create_folder(name).await
    }

    /// Share a folder with a user by email. Sends Google's standard
    /// notification email so the peer sees the invite.
    pub async fn share_folder(&self, folder_id: &str, email: &str, role: &str) -> Result<()> {
        let token = self.token().await?;
        let body = json!({
            "role": role,
            "type": "user",
            "emailAddress": email,
        });
        let resp = self
            .http
            .post(format!(
                "{}/files/{}/permissions?sendNotificationEmail=true",
                DRIVE_API, folder_id
            ))
            .bearer_auth(&token)
            .json(&body)
            .send()
            .await?;
        Self::check_status(resp).await?;
        Ok(())
    }

    /// HEAD-equivalent: fetch folder metadata. Used to confirm drive.file
    /// scope can see the folder before binding.
    pub async fn get_folder(&self, folder_id: &str) -> Result<DriveFile> {
        let token = self.token().await?;
        let resp = self
            .http
            .get(format!(
                "{}/files/{}?fields=id,name,createdTime,mimeType",
                DRIVE_API, folder_id
            ))
            .bearer_auth(&token)
            .send()
            .await?;
        let resp = Self::check_status(resp).await?;
        Ok(resp.json().await?)
    }

    pub async fn list_folder(&self, folder_id: &str) -> Result<Vec<DriveFile>> {
        let token = self.token().await?;
        let q = format!("'{}' in parents and trashed = false", folder_id);
        let resp = self
            .http
            .get(format!("{}/files", DRIVE_API))
            .bearer_auth(&token)
            .query(&[
                ("q", q.as_str()),
                ("fields", "files(id,name,createdTime)"),
                ("orderBy", "createdTime"),
                ("pageSize", "100"),
            ])
            .send()
            .await?;
        let resp = Self::check_status(resp).await?;
        let parsed: ListResponse = resp.json().await?;
        Ok(parsed.files)
    }

    pub async fn upload(&self, folder_id: &str, name: &str, data: &[u8]) -> Result<String> {
        let token = self.token().await?;
        let metadata = json!({
            "name": name,
            "parents": [folder_id],
        })
        .to_string();
        let body = build_multipart_body(&metadata, data);
        let resp = self
            .http
            .post(format!("{}?uploadType=multipart", DRIVE_UPLOAD))
            .bearer_auth(&token)
            .header(
                reqwest::header::CONTENT_TYPE,
                format!("multipart/related; boundary={}", MULTIPART_BOUNDARY),
            )
            .body(body)
            .send()
            .await?;
        let resp = Self::check_status(resp).await?;
        let id: IdResponse = resp.json().await?;
        Ok(id.id)
    }

    pub async fn download(&self, file_id: &str) -> Result<Vec<u8>> {
        let token = self.token().await?;
        let resp = self
            .http
            .get(format!("{}/files/{}?alt=media", DRIVE_API, file_id))
            .bearer_auth(&token)
            .send()
            .await?;
        let resp = Self::check_status(resp).await?;
        Ok(resp.bytes().await?.to_vec())
    }

    pub async fn delete(&self, file_id: &str) -> Result<()> {
        let token = self.token().await?;
        let resp = self
            .http
            .delete(format!("{}/files/{}", DRIVE_API, file_id))
            .bearer_auth(&token)
            .send()
            .await?;
        Self::check_status(resp).await?;
        Ok(())
    }
}

const MULTIPART_BOUNDARY: &str = "otp-msgr-boundary";

fn build_multipart_body(metadata_json: &str, data: &[u8]) -> Vec<u8> {
    let mut body = Vec::with_capacity(metadata_json.len() + data.len() + 256);
    body.extend_from_slice(b"--");
    body.extend_from_slice(MULTIPART_BOUNDARY.as_bytes());
    body.extend_from_slice(b"\r\nContent-Type: application/json; charset=UTF-8\r\n\r\n");
    body.extend_from_slice(metadata_json.as_bytes());
    body.extend_from_slice(b"\r\n--");
    body.extend_from_slice(MULTIPART_BOUNDARY.as_bytes());
    body.extend_from_slice(b"\r\nContent-Type: application/octet-stream\r\n\r\n");
    body.extend_from_slice(data);
    body.extend_from_slice(b"\r\n--");
    body.extend_from_slice(MULTIPART_BOUNDARY.as_bytes());
    body.extend_from_slice(b"--\r\n");
    body
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn multipart_body_structure() {
        let body = build_multipart_body(r#"{"name":"x"}"#, b"DATA");
        let s = std::str::from_utf8(&body).unwrap();
        assert!(s.contains("--otp-msgr-boundary"));
        assert!(s.contains("Content-Type: application/json"));
        assert!(s.contains(r#"{"name":"x"}"#));
        assert!(s.contains("Content-Type: application/octet-stream"));
        assert!(s.contains("DATA"));
        assert!(s.ends_with("--otp-msgr-boundary--\r\n"));
    }
}
