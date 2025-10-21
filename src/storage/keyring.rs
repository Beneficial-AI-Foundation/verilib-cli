use anyhow::{Context, Result};
use crate::storage::types::CredentialStorage;

const SERVICE_NAME: &str = "verilib";

#[cfg(not(target_os = "linux"))]
pub struct KeyringStorage {
    entry: keyring::Entry,
}

#[cfg(not(target_os = "linux"))]
impl KeyringStorage {
    pub fn new() -> Result<Self> {
        let user = whoami::username();
        let entry = keyring::Entry::new(SERVICE_NAME, &user)
            .context("Failed to create keyring entry")?;
        Ok(Self { entry })
    }
}

#[cfg(not(target_os = "linux"))]
impl CredentialStorage for KeyringStorage {
    fn set_password(&self, password: &str) -> Result<()> {
        self.entry
            .set_password(password)
            .context("Failed to set password in keyring")
    }

    fn get_password(&self) -> Result<String> {
        self.entry
            .get_password()
            .context("Failed to get password from keyring")
    }

    fn delete_password(&self) -> Result<()> {
        self.entry
            .delete_credential()
            .context("Failed to delete password from keyring")
    }
}
