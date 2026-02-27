mod factory;
mod file;
mod types;

#[cfg(not(target_os = "linux"))]
mod keyring;

pub use factory::CredentialStorageFactory;
pub use types::{CredentialStorage, StorageType};

use anyhow::Result;

pub fn get_credential_storage() -> Result<Box<dyn CredentialStorage>> {
    CredentialStorageFactory::create()
}

pub fn get_platform_info() -> String {
    let storage_type = StorageType::from_env();

    let base_info = if storage_type.should_use_file_storage() {
        "Secure file storage (~/.verilib_credentials)"
    } else {
        #[cfg(target_os = "macos")]
        let platform = "macOS Keychain (apple-native)";

        #[cfg(target_os = "windows")]
        let platform = "Windows Credential Manager (windows-native)";

        #[cfg(not(any(target_os = "macos", target_os = "windows")))]
        let platform = "Generic keyring backend";

        platform
    };

    match storage_type {
        StorageType::Auto => base_info.to_string(),
        StorageType::File => format!("{} (forced via VERILIB_STORAGE=file)", base_info),
        StorageType::Keyring => format!("{} (forced via VERILIB_STORAGE=keyring)", base_info),
    }
}

pub fn print_platform_help() {
    let storage_type = StorageType::from_env();

    eprintln!("Storage configuration:");
    eprintln!("   • Current: {}", get_platform_info());
    eprintln!();

    if storage_type.should_use_file_storage() {
        eprintln!("File storage tips:");
        eprintln!("   • Credentials are stored in a secure file: ~/.verilib_credentials");
        eprintln!("   • File permissions are set to 0600 (owner read/write only)");
        eprintln!("   • Make sure your home directory has appropriate permissions");
    } else {
        #[cfg(target_os = "macos")]
        {
            eprintln!("Keychain tips:");
            eprintln!("   • Make sure you allow access to the keychain when prompted");
            eprintln!("   • You may need to unlock your keychain");
        }

        #[cfg(target_os = "windows")]
        {
            eprintln!("Credential Manager tips:");
            eprintln!("   • Make sure Windows Credential Manager is available");
            eprintln!("   • You may need administrator privileges");
        }
    }

    eprintln!();
    eprintln!("Environment variable options:");
    eprintln!("   • VERILIB_STORAGE=auto    (default, platform-specific)");
    eprintln!("   • VERILIB_STORAGE=keyring (force system keyring)");
    eprintln!("   • VERILIB_STORAGE=file    (force file storage, useful for testing)");
}
