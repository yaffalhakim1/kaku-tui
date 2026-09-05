// ponytail: no config file yet. CLI flags only. Add config when first user complains.
// ponytail: hand-rolled base64 for the one header. Add the `base64` crate if we
// need it elsewhere.

pub mod types;

use anyhow::{Context, Result};
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION};
use reqwest::{Client, RequestBuilder, Url};

pub use types::{Health, PromptResponse, Session};

#[derive(Debug, Clone)]
pub struct OpencodeClient {
    http: Client,
    base: Url,
}

impl OpencodeClient {
    pub fn new(base: Url, password: Option<&str>) -> Result<Self> {
        let mut headers = HeaderMap::new();
        if let Some(pw) = password {
            // ponytail: HTTP Basic per opencode contract. Username is always
            // "opencode" unless OPENCODE_SERVER_USERNAME is set server-side.
            let creds = format!("opencode:{pw}");
            let encoded = base64_encode(&creds);
            let val = HeaderValue::from_str(&format!("Basic {encoded}"))?;
            headers.insert(AUTHORIZATION, val);
        }
        let http = Client::builder()
            .default_headers(headers)
            .build()
            .context("build reqwest client")?;
        Ok(Self { http, base })
    }

    pub fn base_url(&self) -> &Url {
        &self.base
    }

    /// GET /global/health → { healthy, version }
    pub async fn health(&self) -> Result<Health> {
        let url = self.base.join("/global/health")?;
        let h = self
            .http
            .get(url)
            .send()
            .await
            .context("GET /global/health")?
            .error_for_status()
            .context("health status")?
            .json::<Health>()
            .await
            .context("decode health")?;
        Ok(h)
    }

    /// POST /session { title } → Session
    pub async fn create_session(&self, title: Option<&str>) -> Result<Session> {
        let url = self.base.join("/session")?;
        let body = serde_json::json!({ "title": title });
        let s = self
            .http
            .post(url)
            .json(&body)
            .send()
            .await
            .context("POST /session")?
            .error_for_status()
            .context("session status")?
            .json::<Session>()
            .await
            .context("decode session")?;
        Ok(s)
    }

    /// POST /session/:id/prompt_async — fire-and-forget, returns 204.
    /// The actual response streams back via SSE on /event.
    ///
    /// Body shape (per opencode server contract):
    /// {
    ///   "parts": [{ "type": "text", "text": "..." }],
    ///   // model is optional — server picks its default if omitted.
    /// }
    ///
    /// ponytail: no `model` field per the plan. Server decides.
    pub async fn send_prompt(&self, session_id: &str, text: &str) -> Result<()> {
        let url = self.base.join(&format!("/session/{session_id}/prompt_async"))?;
        let body = serde_json::json!({
            "parts": [{ "type": "text", "text": text }],
        });
        let resp = self
            .http
            .post(url)
            .json(&body)
            .send()
            .await
            .context("POST prompt_async")?
            .error_for_status()
            .context("prompt_async status")?;
        // Expected: 204 No Content. We don't need to parse anything.
        let _ = resp.bytes().await;
        Ok(())
    }

    /// POST /session/:id/abort — cancel an in-flight prompt.
    pub async fn abort(&self, session_id: &str) -> Result<()> {
        let url = self.base.join(&format!("/session/{session_id}/abort"))?;
        self.http
            .post(url)
            .send()
            .await
            .context("POST abort")?
            .error_for_status()
            .context("abort status")?;
        Ok(())
    }

    /// Exposes the underlying reqwest Client for the SSE reader task that needs
    /// a streaming body. Kept narrow on purpose — don't add more escape hatches.
    pub fn http_get(&self, url: Url) -> RequestBuilder {
        self.http.get(url)
    }

    /// POST /session/:id/message — kept for debugging / single-shot scripts.
    /// v0 doesn't use it (prompt_async + SSE is the streaming path).
    ///
    /// ponytail: dead in the main loop. Marked allow(dead_code) at call site.
    #[allow(dead_code)]
    pub async fn send_prompt_sync(&self, session_id: &str, text: &str) -> Result<PromptResponse> {
        let url = self.base.join(&format!("/session/{session_id}/message"))?;
        let body = serde_json::json!({
            "parts": [{ "type": "text", "text": text }],
        });
        let resp = self
            .http
            .post(url)
            .json(&body)
            .send()
            .await
            .context("POST message")?
            .error_for_status()
            .context("message status")?
            .json::<PromptResponse>()
            .await
            .context("decode message response")?;
        Ok(resp)
    }
}

// ponytail: avoid base64 crate for one header. Hand-roll, ~10 lines.
// If we add base64-encoding for other things later, swap to the crate.
fn base64_encode(input: &str) -> String {
    const TABLE: &[u8; 64] =
        b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let bytes = input.as_bytes();
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let b0 = chunk[0];
        let b1 = chunk.get(1).copied().unwrap_or(0);
        let b2 = chunk.get(2).copied().unwrap_or(0);
        let n = ((b0 as u32) << 16) | ((b1 as u32) << 8) | (b2 as u32);
        out.push(TABLE[((n >> 18) & 0x3f) as usize] as char);
        out.push(TABLE[((n >> 12) & 0x3f) as usize] as char);
        out.push(if chunk.len() > 1 { TABLE[((n >> 6) & 0x3f) as usize] as char } else { '=' });
        out.push(if chunk.len() > 2 { TABLE[(n & 0x3f) as usize] as char } else { '=' });
    }
    out
}
