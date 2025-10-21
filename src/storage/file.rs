use anyhow::{Context, Result};
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::PathBuf;

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

use crate::storage::types::CredentialStorage;

const FILE_NAME: &str = ".verilib_credentials";

pub struct FileStorage {
    file_path: PathBuf,
}

impl FileStorage {
    pub fn new() -> Result<Self> {
        let home_dir = dirs::home_dir()
            .context("Failed to get home directory")?;
        let file_path = home_dir.join(FILE_NAME);
        Ok(Self { file_path })
    }

    fn ensure_secure_file(&self) -> Result<()> {
        if !self.file_path.exists() {
            File::create(&self.file_path)
                .context("Failed to create credentials file")?;
        }

        #[cfg(unix)]
        {
            let metadata = fs::metadata(&self.file_path)
                .context("Failed to read file metadata")?;
            let mut permissions = metadata.permissions();
            permissions.set_mode(0o600);
            fs::set_permissions(&self.file_path, permissions)
                .context("Failed to set file permissions")?;
        }

        Ok(())
    }
}

impl CredentialStorage for FileStorage {
    fn set_password(&self, password: &str) -> Result<()> {
        self.ensure_secure_file()?;

        let mut file = OpenOptions::new()
            .write(true)
            .truncate(true)
            .open(&self.file_path)
            .context("Failed to open credentials file for writing")?;

        file.write_all(password.as_bytes())
            .context("Failed to write password to file")?;

        Ok(())
    }

    fn get_password(&self) -> Result<String> {
        if !self.file_path.exists() {
            anyhow::bail!("No credentials file found");
        }

        let mut file = File::open(&self.file_path)
            .context("Failed to open credentials file")?;

        let mut password = String::new();
        file.read_to_string(&mut password)
            .context("Failed to read password from file")?;

        if password.is_empty() {
            anyhow::bail!("Credentials file is empty");
        }

        Ok(password)
    }

    fn delete_password(&self) -> Result<()> {
        if self.file_path.exists() {
            fs::remove_file(&self.file_path)
                .context("Failed to delete credentials file")?;
        }
        Ok(())
    }
}
