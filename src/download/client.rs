use anyhow::{Context, Result};
use reqwest::Client;
use std::fs;

use super::types::DownloadResponse;
use super::error::handle_api_error;

pub async fn download_repo(
    repo_id: &str,
    base_url: &str,
    api_key: &str,
    debug: bool,
) -> Result<DownloadResponse> {
    let endpoint = format!("{}/v2/repo/download/{}", base_url, repo_id);
    
    let client = Client::new();
    let response = client
        .get(&endpoint)
        .header("Authorization", format!("ApiKey {}", api_key))
        .header("Accept", "application/json")
        .send()
        .await
        .context("Failed to send request to API")?;
    
    if !response.status().is_success() {
        let error_msg = handle_api_error(response).await?;
        anyhow::bail!(error_msg);
    }
    
    let response_text = response
        .text()
        .await
        .context("Failed to read response body")?;
    
    if debug {
        fs::create_dir_all(".verilib")
            .context("Failed to create .verilib directory for debug output")?;
        fs::write(".verilib/debug_response.json", &response_text)
            .context("Failed to write debug response file")?;
        println!("Debug: API response saved to .verilib/debug_response.json");
    }
    
    let download_data: DownloadResponse = serde_json::from_str(&response_text)
        .context("Failed to parse JSON response")?;
    
    Ok(download_data)
}
