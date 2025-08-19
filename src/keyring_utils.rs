use anyhow::{Context, Result};
use keyring::Entry;

const SERVICE_NAME: &str = "verilib";

pub fn get_keyring_entry() -> Result<Entry> {
    let user = whoami::username();
    Entry::new(SERVICE_NAME, &user).context("Failed to create keyring entry")
}

pub fn get_platform_keyring_info() -> &'static str {
    #[cfg(target_os = "macos")]
    return "macOS Keychain (apple-native)";
    
    #[cfg(target_os = "windows")]
    return "Windows Credential Manager (windows-native)";
    
    #[cfg(target_os = "linux")]
    return "Linux Secret Service (libsecret/gnome-keyring)";
    
    #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
    return "Generic keyring backend";
}

pub fn print_platform_help() {
    #[cfg(target_os = "macos")]
    {
        eprintln!("   • Make sure you allow access to the keychain when prompted");
        eprintln!("   • You may need to unlock your keychain");
    }
    
    #[cfg(target_os = "windows")]
    {
        eprintln!("   • Make sure Windows Credential Manager is available");
        eprintln!("   • You may need administrator privileges");
    }
    
    #[cfg(target_os = "linux")]
    {
        eprintln!("   • Make sure libsecret or gnome-keyring is installed");
        eprintln!("   • On Ubuntu/Debian: sudo apt install libsecret-1-0");
        eprintln!("   • On Fedora/RHEL: sudo dnf install libsecret");
        eprintln!("   • Make sure you're in a desktop session with keyring unlocked");
    }
}
