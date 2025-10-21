use anyhow::{Context, Result};

use crate::storage::{get_credential_storage, get_platform_info};

pub async fn handle_status() -> Result<()> {
    let platform_info = get_platform_info();
    
    match get_stored_api_key() {
        Ok(key) => {
            let masked_key = format!("{}***", if key.len() > 4 { &key[..4] } else { &key });

            println!("API key is stored: {}", masked_key);
            println!("Stored in keyring service: verilib");
            println!("Platform: {}", platform_info);
        }
        Err(e) => {
            println!("No API key found");
            println!("Run 'verilib-cli auth' to authenticate");
            println!("Platform: {}", platform_info);
            println!("Debug info: {}", e);
        }
    }
    Ok(())
}

pub fn get_stored_api_key() -> Result<String> {
    let entry = get_credential_storage()?;
    
    entry.get_password()
        .context("Failed to retrieve API key from storage")
}
