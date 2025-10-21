use anyhow::{Context, Result};
use rpassword::prompt_password;

use crate::storage::{get_credential_storage, print_platform_help};

pub async fn handle_auth() -> Result<()> {
    println!("Please enter your Verilib API key:");
    
    let key = prompt_password("API Key: ")
        .context("Failed to read API key from input")?;

    if key.trim().is_empty() {
        anyhow::bail!("API key cannot be empty");
    }

    println!("Attempting to store API key...");

    let entry = get_credential_storage()?;
    
    match entry.set_password(&key.trim()) {
        Ok(()) => {
            println!("API key successfully stored.");
            println!("Your API key is securely stored in the system keyring.");
            
            match entry.get_password() {
                Ok(stored_key) => {
                    if stored_key == key.trim() {
                        println!("Storage verified successfully.");
                    } else {
                        println!("Warning: Storage verification failed - keys don't match");
                    }
                }
                Err(e) => {
                    println!("Warning: Could not verify storage: {}", e);
                }
            }
        }
        Err(err) => {
            eprintln!("Failed to store API key: {}", err);
            eprintln!("Platform-specific help:");
            print_platform_help();
            anyhow::bail!("Failed to store API key");
        }
    }
    
    Ok(())
}
