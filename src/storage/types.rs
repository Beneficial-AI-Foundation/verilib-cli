use anyhow::Result;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum StorageType {
    Auto,
    Keyring,
    File,
}

impl StorageType {
    pub fn from_env() -> Self {
        std::env::var("VERILIB_STORAGE")
            .ok()
            .and_then(|s| match s.to_lowercase().as_str() {
                "keyring" => Some(StorageType::Keyring),
                "file" => Some(StorageType::File),
                "auto" => Some(StorageType::Auto),
                _ => None,
            })
            .unwrap_or(StorageType::Auto)
    }

    pub fn should_use_file_storage(self) -> bool {
        match self {
            StorageType::File => true,
            StorageType::Keyring => false,
            StorageType::Auto => cfg!(target_os = "linux"),
        }
    }
}

pub trait CredentialStorage {
    fn set_password(&self, password: &str) -> Result<()>;
    fn get_password(&self) -> Result<String>;

    #[allow(dead_code)]
    fn delete_password(&self) -> Result<()>;
}
