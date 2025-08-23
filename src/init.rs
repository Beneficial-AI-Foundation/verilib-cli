use anyhow::{Context, Result};
use reqwest::Client;
use serde_json::Value;
use std::fs;

use crate::constants::{auth_required_msg, DEFAULT_BASE_URL};
use crate::status::get_stored_api_key;

pub async fn handle_init(repo_id: String, base_url: Option<String>) -> Result<()> {
    println!("Initializing project with repository ID: {}", repo_id);
    
    // Get the API key from keyring
    let api_key = get_stored_api_key()
        .context(auth_required_msg())?;
    
    // Determine the base URL
    let url_base = base_url.unwrap_or_else(|| DEFAULT_BASE_URL.to_string());
    let endpoint = format!("{}/v2/repo/tree/{}", url_base, repo_id);
    
    println!("Fetching repository tree from: {}", endpoint);
    
    // Create HTTP client
    let client = Client::new();
    
    // Make the API request
    let response = client
        .get(&endpoint)
        .header("Authorization", format!("ApiKey {}", api_key))
        .header("Accept", "application/json")
        .send()
        .await
        .context("Failed to send request to API")?;
    
    // Check if request was successful
    if !response.status().is_success() {
        anyhow::bail!(
            "API request failed with status: {} - {}",
            response.status(),
            response.text().await.unwrap_or_else(|_| "Unable to read error response".to_string())
        );
    }
    
    // Get the response body
    let response_text = response
        .text()
        .await
        .context("Failed to read response body")?;
    
    // Parse JSON response
    let json_response: Value = serde_json::from_str(&response_text)
        .context("Failed to parse JSON response")?;
    
    // Extract the data property
    let data = json_response
        .get("data")
        .context("Response does not contain 'data' property")?;
    
    // Convert data back to JSON string for storage
    let data_json = serde_json::to_string_pretty(data)
        .context("Failed to serialize data to JSON")?;
    
    // Create .verilib directory if it doesn't exist
    fs::create_dir_all(".verilib")
        .context("Failed to create .verilib directory")?;
    
    // Write to .verilib/tree.json file
    fs::write(".verilib/tree.json", &data_json)
        .context("Failed to write tree.json file")?;
    
    println!("Repository tree data successfully saved to .verilib/tree.json");
    println!("File size: {} bytes", data_json.len());
    
    Ok(())
}
