use crate::storage::file::FileStorage;
use crate::storage::types::{CredentialStorage, StorageType};
use anyhow::Result;

#[cfg(not(target_os = "linux"))]
use crate::storage::keyring::KeyringStorage;

pub struct CredentialStorageFactory;

impl CredentialStorageFactory {
    pub fn create() -> Result<Box<dyn CredentialStorage>> {
        Self::create_with_type(StorageType::from_env())
    }

    pub fn create_with_type(storage_type: StorageType) -> Result<Box<dyn CredentialStorage>> {
        if storage_type.should_use_file_storage() {
            Ok(Box::new(FileStorage::new()?))
        } else {
            #[cfg(not(target_os = "linux"))]
            {
                Ok(Box::new(KeyringStorage::new()?))
            }

            #[cfg(target_os = "linux")]
            {
                anyhow::bail!(
                    "Keyring storage is not available on Linux. Use file storage instead \
                     or set VERILIB_STORAGE=file"
                )
            }
        }
    }
}
