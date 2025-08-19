use anyhow::Result;

use crate::keyring_utils::{get_keyring_entry, get_platform_keyring_info};

pub async fn handle_status() -> Result<()> {
    let platform_info = get_platform_keyring_info();
    
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
    let entry = get_keyring_entry()?;
    
    match entry.get_password() {
        Ok(password) => Ok(password),
        Err(keyring::Error::NoEntry) => {
            anyhow::bail!("No API key found in keyring")
        }
        Err(keyring::Error::PlatformFailure(err)) => {
            anyhow::bail!("Platform keyring error: {}", err)
        }
        Err(err) => {
            anyhow::bail!("Keyring error: {}", err)
        }
    }
}
