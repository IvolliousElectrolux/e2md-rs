use anyhow::{bail, Context, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::time::Duration;

use crate::utils::{
    file::{ensure_dir, extract_zip},
    log::log_info,
    path::{job_tmp_dir, to_ascii_safe},
};

const POLL_INTERVAL_SECS: u64 = 10;
const MAX_POLL_ATTEMPTS: u32 = 180; // 30 minutes maximum

pub struct MinerUModule;

impl MinerUModule {
    /// Full pipeline: register → upload → poll → download → extract.
    ///
    /// Returns the path to `merged_raw.md` inside the job's tmp dir.
    pub async fn convert(
        client: &Client,
        base_url: &str,
        token: &str,
        guid: &str,
        source_path: &Path,
    ) -> Result<std::path::PathBuf> {
        let original_name = source_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("source.pdf");
        let data_id = to_ascii_safe(original_name);

        log_info(format!("[MinerU] 开始转换: {} ({})", original_name, guid));

        // 1. Register batch
        let (batch_id, upload_url) =
            Self::register_batch(client, base_url, token, original_name, &data_id).await?;
        log_info(format!("[MinerU] 批次注册成功: {}", batch_id));

        // 2. Upload file
        let file_bytes = std::fs::read(source_path).context("Reading source file")?;
        Self::upload_file(client, &upload_url, &file_bytes).await?;
        log_info(format!("[MinerU] 文件上传成功: {}", original_name));

        // 3. Poll until done
        let zip_url = Self::poll_until_done(client, base_url, token, &batch_id).await?;
        log_info(format!("[MinerU] 转换完成, 下载结果..."));

        // 4. Download and extract
        let zip_bytes = Self::download(client, &zip_url).await?;
        let tmp_dir = job_tmp_dir(guid);
        ensure_dir(&tmp_dir)?;
        extract_zip(&zip_bytes, &tmp_dir).context("Extracting MinerU zip")?;

        // 5. Find the markdown file (MinerU names it `full.md` or `<dataId>.md`)
        let md_path = find_markdown_file(&tmp_dir)?;
        let raw_md_dest = tmp_dir.join("merged_raw.md");
        if md_path != raw_md_dest {
            std::fs::rename(&md_path, &raw_md_dest)
                .context("Renaming markdown to merged_raw.md")?;
        }

        log_info(format!("[MinerU] 完成: {} -> merged_raw.md", guid));
        Ok(raw_md_dest)
    }

    async fn register_batch(
        client: &Client,
        base_url: &str,
        token: &str,
        original_name: &str,
        data_id: &str,
    ) -> Result<(String, String)> {
        #[derive(Serialize)]
        struct FileEntry<'a> {
            name: &'a str,
            data_id: &'a str,
        }
        #[derive(Serialize)]
        struct RegisterBody<'a> {
            model_version: &'a str,
            files: Vec<FileEntry<'a>>,
        }
        #[derive(Deserialize)]
        struct RegisterResponse {
            code: i64,
            msg: String,
            data: Option<RegisterData>,
        }
        #[derive(Deserialize)]
        struct RegisterData {
            batch_id: String,
            file_urls: Vec<String>,
        }

        let url = format!("{}/api/v4/file-urls/batch", base_url);
        let body = RegisterBody {
            model_version: "vlm",
            files: vec![FileEntry { name: original_name, data_id }],
        };

        let resp = client
            .post(&url)
            .header("Authorization", format!("Bearer {}", token))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .context("Register batch request failed")?;

        let text = resp.text().await?;
        let parsed: RegisterResponse =
            serde_json::from_str(&text).context("Parsing register response")?;

        if parsed.code != 0 {
            if parsed.msg.contains("A0202") {
                bail!("MinerU Token 格式错误, 请检查是否带有 Bearer 前缀");
            }
            if parsed.msg.contains("A0211") {
                bail!("MinerU Token 已过期 (有效期 14 天), 请重新申请");
            }
            bail!("MinerU 注册失败: {}", parsed.msg);
        }

        let data = parsed.data.ok_or_else(|| anyhow::anyhow!("Missing data field"))?;
        let upload_url = data
            .file_urls
            .into_iter()
            .next()
            .ok_or_else(|| anyhow::anyhow!("Empty file_urls array"))?;

        Ok((data.batch_id, upload_url))
    }

    async fn upload_file(client: &Client, url: &str, bytes: &[u8]) -> Result<()> {
        // ⚠️  PUT with NO extra headers — OSS signature validation will fail otherwise
        let resp = client
            .put(url)
            .body(bytes.to_vec())
            .send()
            .await
            .context("Upload PUT request failed")?;

        let status = resp.status();
        if status.as_u16() == 403 {
            bail!("MinerU 上传 403: 请确认 PUT 请求未携带 Authorization 或 Content-Type header");
        }
        if !status.is_success() {
            bail!("MinerU 上传失败, HTTP {}", status);
        }
        Ok(())
    }

    async fn poll_until_done(
        client: &Client,
        base_url: &str,
        token: &str,
        batch_id: &str,
    ) -> Result<String> {
        #[derive(Deserialize)]
        struct PollResponse {
            code: i64,
            msg: String,
            data: Option<PollData>,
        }
        #[derive(Deserialize)]
        struct PollData {
            extract_result: Vec<ExtractResult>,
        }
        #[derive(Deserialize)]
        struct ExtractResult {
            state: String,
            err_msg: Option<String>,
            extract_progress: Option<Progress>,
            full_zip_url: Option<String>,
        }
        #[derive(Deserialize)]
        struct Progress {
            extracted_pages: u32,
            total_pages: u32,
        }

        let url = format!("{}/api/v4/extract-results/batch/{}", base_url, batch_id);

        for attempt in 0..MAX_POLL_ATTEMPTS {
            tokio::time::sleep(Duration::from_secs(POLL_INTERVAL_SECS)).await;

            let resp = client
                .get(&url)
                .header("Authorization", format!("Bearer {}", token))
                .send()
                .await
                .context("Poll request failed")?;

            let text = resp.text().await?;
            let parsed: PollResponse =
                serde_json::from_str(&text).context("Parsing poll response")?;

            if parsed.code != 0 {
                bail!("MinerU 轮询失败: {}", parsed.msg);
            }

            let result = parsed
                .data
                .and_then(|d| d.extract_result.into_iter().next())
                .ok_or_else(|| anyhow::anyhow!("Empty extract_result"))?;

            if let Some(prog) = &result.extract_progress {
                log_info(format!(
                    "[MinerU] 进度: {}/{} 页 (批次 {})",
                    prog.extracted_pages, prog.total_pages, batch_id
                ));
            }

            match result.state.as_str() {
                "done" => {
                    if let Some(zip_url) = result.full_zip_url {
                        if !zip_url.is_empty() {
                            return Ok(zip_url);
                        }
                    }
                    bail!("MinerU 状态 done 但 full_zip_url 为空");
                }
                "failed" => {
                    let err = result.err_msg.unwrap_or_default();
                    bail!("MinerU 转换失败: {}", err);
                }
                state => {
                    log_info(format!("[MinerU] 状态: {} (尝试 {}/{})", state, attempt + 1, MAX_POLL_ATTEMPTS));
                }
            }
        }

        bail!("MinerU 轮询超时 ({}s)", MAX_POLL_ATTEMPTS as u64 * POLL_INTERVAL_SECS);
    }

    async fn download(client: &Client, url: &str) -> Result<Vec<u8>> {
        let resp = client
            .get(url)
            .send()
            .await
            .context("Download request failed")?;
        if !resp.status().is_success() {
            bail!("MinerU 结果下载失败, HTTP {}", resp.status());
        }
        Ok(resp.bytes().await?.to_vec())
    }
}

fn find_markdown_file(dir: &Path) -> Result<std::path::PathBuf> {
    // MinerU typically produces `full.md` at root or in a subdirectory
    for candidate in &["full.md", "output.md", "result.md"] {
        let p = dir.join(candidate);
        if p.exists() {
            return Ok(p);
        }
    }
    // Fallback: first .md file found
    for entry in walkdir(dir)? {
        if entry.extension().and_then(|e| e.to_str()) == Some("md") {
            return Ok(entry);
        }
    }
    bail!("找不到 MinerU 输出的 Markdown 文件 in {}", dir.display());
}

fn walkdir(dir: &Path) -> Result<Vec<std::path::PathBuf>> {
    let mut files = Vec::new();
    if dir.is_dir() {
        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                files.extend(walkdir(&path)?);
            } else {
                files.push(path);
            }
        }
    }
    Ok(files)
}
