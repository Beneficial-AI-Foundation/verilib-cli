use serde::{Deserialize, Serialize};

#[derive(Debug)]
pub struct Language {
    pub id: u32,
    pub name: &'static str,
    pub extensions: &'static [&'static str],
}

pub const LANGUAGES: &[Language] = &[
    Language {
        id: 1,
        name: "Dafny",
        extensions: &[".dfy"],
    },
    Language {
        id: 2,
        name: "Lean",
        extensions: &[".lean"],
    },
    Language {
        id: 3,
        name: "Rocq",
        extensions: &[".v"],
    },
    Language {
        id: 4,
        name: "Isabelle",
        extensions: &[".thy"],
    },
    Language {
        id: 5,
        name: "Metamath",
        extensions: &[".mm"],
    },
    Language {
        id: 6,
        name: "Rust",
        extensions: &[".rs"],
    },
    Language {
        id: 7,
        name: "RefinedC",
        extensions: &[".c"],
    },
    Language {
        id: 8,
        name: "Python",
        extensions: &[".py"],
    },
    Language {
        id: 9,
        name: "Kani",
        extensions: &[".rs"],
    },
    Language {
        id: 10,
        name: "Verus",
        extensions: &[".rs"],
    },
];

pub const TYPES: &[(u32, &str)] = &[
    (1, "Algorithms"),
    (5, "Blockchain"),
    (6, "Privacy"),
    (7, "Security"),
    (8, "Math"),
];

#[derive(Debug, Serialize)]
pub struct DeployNode {
    pub identifier: String,
    pub content: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub dependencies: Vec<String>,
    pub code_name: String,
    pub children: Vec<DeployNode>,
    pub file_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status_id: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub snippets: Option<serde_json::Value>,
    pub specified: bool,
    #[serde(default)]
    pub disabled: bool,
}

#[derive(Debug, Deserialize)]
pub struct VerifierVersionsResponse {
    pub data: Vec<VerifierVersion>,
}

#[derive(Debug, Deserialize)]
pub struct VerifierVersion {
    pub id: u32,
    pub version: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Config {
    pub repo: RepoConfig,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct RepoConfig {
    pub id: String,
    pub url: String,
    #[serde(default)]
    pub is_admin: bool,
}

#[derive(Debug, Deserialize)]
pub struct DeployResponse {
    #[allow(dead_code)]
    pub status: String,
    pub data: DeployData,
}

#[derive(Debug, Deserialize)]
pub struct DeployData {
    pub id: u64,
}
