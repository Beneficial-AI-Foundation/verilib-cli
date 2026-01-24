//! Certificate management for verilib structure.
//!
//! Handles creation and lookup of specification certificates.

use anyhow::Result;
use chrono::{DateTime, Utc};
use percent_encoding::{percent_decode_str, utf8_percent_encode, NON_ALPHANUMERIC};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::{Path, PathBuf};

/// Certificate data stored in cert files.
#[derive(Debug, Serialize, Deserialize)]
pub struct Cert {
    pub timestamp: DateTime<Utc>,
}

/// Encode an identifier for use as a filename.
///
/// Uses URL percent-encoding to replace special characters like '/', ':', '#', etc.
pub fn encode_name(name: &str) -> String {
    utf8_percent_encode(name, NON_ALPHANUMERIC).to_string()
}

/// Decode a filename back to an identifier.
pub fn decode_name(encoded: &str) -> String {
    percent_decode_str(encoded)
        .decode_utf8_lossy()
        .to_string()
}

/// Get the set of identifiers that already have certs.
pub fn get_existing_certs(certs_dir: &Path) -> Result<HashSet<String>> {
    let mut existing = HashSet::new();

    if !certs_dir.exists() {
        return Ok(existing);
    }

    for entry in std::fs::read_dir(certs_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().map_or(false, |ext| ext == "json") {
            if let Some(stem) = path.file_stem() {
                let encoded_name = stem.to_string_lossy();
                let name = decode_name(&encoded_name);
                existing.insert(name);
            }
        }
    }

    Ok(existing)
}

/// Create a cert file for a function.
pub fn create_cert(certs_dir: &Path, name: &str) -> Result<PathBuf> {
    std::fs::create_dir_all(certs_dir)?;

    let encoded_name = encode_name(name);
    let cert_path = certs_dir.join(format!("{}.json", encoded_name));

    let cert = Cert {
        timestamp: Utc::now(),
    };

    let content = serde_json::to_string_pretty(&cert)?;
    std::fs::write(&cert_path, content)?;

    Ok(cert_path)
}
