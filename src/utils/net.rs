#![allow(dead_code)]
use anyhow::Context;
use reqwest::{Client, ClientBuilder};
use std::time::Duration;

/// Build a shared HTTP client with a generous timeout for long AI responses.
pub fn build_client() -> anyhow::Result<Client> {
    Ok(ClientBuilder::new()
        .timeout(Duration::from_secs(300))
        .use_rustls_tls()
        .build()
        .context("Failed to build HTTP client")?)
}

/// Build a dedicated upload client (no default headers, plain PUT support).
pub fn build_upload_client() -> anyhow::Result<Client> {
    Ok(ClientBuilder::new()
        .timeout(Duration::from_secs(120))
        .use_rustls_tls()
        .build()
        .context("Failed to build upload HTTP client")?)
}
