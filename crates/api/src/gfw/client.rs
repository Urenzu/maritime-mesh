use anyhow::Result;
use reqwest::{Client, Response, StatusCode};
use tracing::warn;

const GFW_BASE: &str = "https://gateway.api.globalfishingwatch.org";

#[derive(Clone)]
pub struct GfwClient {
    pub http: Client,
    pub api_key: String,
}

impl GfwClient {
    pub fn new(api_key: String) -> Result<Self> {
        let http = Client::builder()
            .timeout(std::time::Duration::from_secs(15))
            .build()?;
        Ok(Self { http, api_key })
    }


    pub(super) async fn get(&self, path: &str) -> Result<Option<serde_json::Value>> {
        let url = format!("{}{}", GFW_BASE, path);
        let resp: Response = self
            .http
            .get(&url)
            .bearer_auth(&self.api_key)
            .send()
            .await?;

        match resp.status() {
            StatusCode::OK => Ok(Some(resp.json().await?)),
            StatusCode::FORBIDDEN | StatusCode::UNAUTHORIZED => {
                warn!("GFW access denied for {path} — key may lack this permission");
                Ok(None)
            }
            StatusCode::NOT_FOUND => Ok(None),
            s => {
                warn!("GFW unexpected status {s} for {path}");
                Ok(None)
            }
        }
    }
}
