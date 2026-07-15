//! ShirabeDriver — CDP-based browser test driver replacing Selenium WebDriver.
//!
//! Uses shirabe's HTTP debug API to drive headless Chromium via CDP,
//! eliminating the thirtyfour/Selenium dependency and its aws-lc-rs chain.

use anyhow::{Context, Result};
use serde::Deserialize;
use std::time::Duration;

pub struct ShirabeDriver {
    client: reqwest::Client,
    base_url: String,
    _server: Option<tokio::task::JoinHandle<()>>,
    shutdown_tx: Option<tokio::sync::oneshot::Sender<()>>,
}

impl ShirabeDriver {
    pub async fn connect(website_url: &str, debug_port: u16) -> Result<Self> {
        let cfg = shirabe::DebugServerConfig {
            base_url: website_url.to_string(),
            dev_port: 0,
            dist_dir: String::new(),
            package_name: "tairitsu-e2e".to_string(),
            proxy: None,
        };
        let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel::<()>();
        let base_url = format!("http://127.0.0.1:{}", debug_port);
        let server_base = base_url.clone();
        let server = tokio::spawn(async move {
            tokio::select! {
                _ = shutdown_rx => {}
                r = shirabe::start_debug_server(cfg, debug_port) => {
                    if let Err(e) = r { tracing::error!("Shirabe error: {e}"); }
                }
            }
        });
        let client = reqwest::Client::new();
        for _ in 0..30 {
            tokio::time::sleep(Duration::from_millis(500)).await;
            if let Ok(r) = client.get(format!("{}/health", server_base)).send().await {
                if r.status().is_success() {
                    return Ok(Self { client, base_url: server_base, _server: Some(server), shutdown_tx: Some(shutdown_tx) });
                }
            }
        }
        let _ = shutdown_tx.send(());
        anyhow::bail!("Shirabe debug server did not become healthy");
    }

    pub async fn goto(&self, url: &str) -> Result<()> {
        self.client.post(format!("{}/navigate", self.base_url)).json(&serde_json::json!({"url":url,"wait_for":"load"})).send().await?.error_for_status()?;
        Ok(())
    }

    pub async fn find(&self, selector: &str) -> Result<ShirabeElement> {
        Ok(ShirabeElement { client: self.client.clone(), base_url: self.base_url.clone(), selector: selector.to_string() })
    }

    pub async fn find_all(&self, selector: &str) -> Result<Vec<ShirabeElement>> {
        #[derive(Deserialize)] struct R { count: Option<u64>, }
        let r: R = self.client.get(format!("{}/dom", self.base_url)).query(&[("selector",selector),("all","true")]).send().await?.json().await?;
        let n = r.count.unwrap_or(0) as usize;
        Ok((0..n).map(|i| ShirabeElement { client: self.client.clone(), base_url: self.base_url.clone(), selector: format!("{selector}:nth-child({})", i+1) }).collect())
    }

    pub async fn execute_script(&self, script: &str) -> Result<serde_json::Value> {
        #[derive(Deserialize)] struct R { result: serde_json::Value }
        let r: R = self.client.post(format!("{}/evaluate", self.base_url)).json(&serde_json::json!({"expression":script,"await_promise":false})).send().await?.json().await?;
        Ok(r.result)
    }

    pub async fn execute(&self, script: &str, _args: Vec<serde_json::Value>) -> Result<serde_json::Value> {
        self.execute_script(script).await
    }

    pub async fn screenshot(&self) -> Result<Vec<u8>> {
        #[derive(Deserialize)] struct R { data: String }
        let r: R = self.client.post(format!("{}/screenshot", self.base_url)).json(&serde_json::json!({"full_page":true})).send().await?.json().await?;
        base64::Engine::decode(&base64::engine::general_purpose::STANDARD, &r.data).context("decode screenshot")
    }

    pub async fn current_url(&self) -> Result<String> {
        let r: serde_json::Value = self.execute_script("window.location.href").await?;
        Ok(r.as_str().unwrap_or("").to_string())
    }

    pub async fn back(&self) -> Result<()> {
        self.client.post(format!("{}/back", self.base_url)).send().await?.error_for_status()?;
        Ok(())
    }

    pub async fn forward(&self) -> Result<()> {
        self.client.post(format!("{}/forward", self.base_url)).send().await?.error_for_status()?;
        Ok(())
    }

    pub fn base_url(&self) -> &str { &self.base_url }

    pub async fn quit(mut self) -> Result<()> {
        if let Some(tx) = self.shutdown_tx.take() { let _ = tx.send(()); }
        tokio::time::sleep(Duration::from_millis(200)).await;
        Ok(())
    }
}

pub struct ShirabeElement {
    client: reqwest::Client,
    base_url: String,
    selector: String,
}

impl ShirabeElement {
    pub async fn click(&self) -> Result<()> {
        self.client.post(format!("{}/click", self.base_url)).json(&serde_json::json!({"selector":self.selector})).send().await?.error_for_status()?;
        Ok(())
    }

    pub async fn send_keys(&self, text: &str) -> Result<()> {
        self.client.post(format!("{}/type", self.base_url)).json(&serde_json::json!({"selector":self.selector,"text":text,"clear_first":false,"submit":false})).send().await?.error_for_status()?;
        Ok(())
    }

    pub async fn clear(&self) -> Result<()> {
        self.client.post(format!("{}/type", self.base_url)).json(&serde_json::json!({"selector":self.selector,"text":"","clear_first":true,"submit":false})).send().await?.error_for_status()?;
        Ok(())
    }

    pub async fn press_key(&self, key: &str) -> Result<()> {
        self.client.post(format!("{}/press", self.base_url)).json(&serde_json::json!({"key":key})).send().await?.error_for_status()?;
        Ok(())
    }

    pub async fn attr(&self, name: &str) -> Result<String> {
        #[derive(Deserialize)] struct R { attributes: Option<std::collections::HashMap<String,String>> }
        match self.client.get(format!("{}/dom", self.base_url)).query(&[("selector",self.selector.as_str()),("attribute",name)]).send().await {
            Ok(r) if r.status().is_success() => {
                let body: R = r.json().await?;
                Ok(body.attributes.and_then(|mut m| m.remove(name)).unwrap_or_default())
            }
            _ => Ok(String::new())
        }
    }

    pub async fn text(&self) -> Result<String> {
        #[derive(Deserialize)] struct R { text: Option<String> }
        match self.client.get(format!("{}/dom", self.base_url)).query(&[("selector",self.selector.as_str())]).send().await {
            Ok(r) if r.status().is_success() => { let b: R = r.json().await?; Ok(b.text.unwrap_or_default()) }
            _ => Ok(String::new())
        }
    }

    pub async fn inner_html(&self) -> Result<String> {
        #[derive(Deserialize)] struct R { html: Option<String> }
        match self.client.get(format!("{}/dom", self.base_url)).query(&[("selector",self.selector.as_str())]).send().await {
            Ok(r) if r.status().is_success() => { let b: R = r.json().await?; Ok(b.html.unwrap_or_default()) }
            _ => Ok(String::new())
        }
    }

    pub async fn is_displayed(&self) -> Result<bool> {
        #[derive(Deserialize)] struct R { visible: Option<bool> }
        match self.client.get(format!("{}/dom", self.base_url)).query(&[("selector",self.selector.as_str())]).send().await {
            Ok(r) if r.status().is_success() => { let b: R = r.json().await?; Ok(b.visible.unwrap_or(false)) }
            _ => Ok(false)
        }
    }

    /// Find a child element within this element using a CSS selector.
    pub async fn find(&self, child_selector: &str) -> Result<ShirabeElement> {
        let combined = if child_selector.starts_with('.') || child_selector.starts_with('#') || child_selector.starts_with('[') {
            format!("{} {}", self.selector, child_selector)
        } else {
            format!("{} {}", self.selector, child_selector)
        };
        Ok(ShirabeElement { client: self.client.clone(), base_url: self.base_url.clone(), selector: combined })
    }
}
