use serde::{Deserialize, Deserializer, Serialize};
use std::collections::HashMap;

#[derive(Debug, Deserialize, Serialize)]
pub struct TreeNode {
    pub id: u64,
    pub parent_id: Option<u64>,
    pub identifier: String,
    pub index: u32,
    pub statement_type: String,
    pub status_id: u32,
    pub specified: bool,
    pub path: String,
    pub snippets: Vec<Snippet>,
    #[serde(default)]
    pub children: Vec<TreeNode>,
    #[serde(default)]
    pub dependencies: Vec<String>,
    #[serde(default)]
    pub code_name: String,
    #[serde(default)]
    pub disabled: bool,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Snippet {
    pub type_id: u32,
    pub text: String,
    pub sortorder: u32,
}

#[derive(Debug, Deserialize)]
pub struct DownloadResponse {
    pub data: DownloadData,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct LayoutNode {
    pub identifier: String,
    #[serde(default)]
    pub id: Option<String>,
    pub fx: f64,
    pub fy: f64,
    pub path: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Layout {
    pub nodes: Vec<LayoutNode>,
    #[serde(default)]
    pub zoom: Option<serde_json::Value>,
    #[serde(default)]
    pub repositioned: Option<bool>,
}

fn deserialize_layouts<'de, D>(deserializer: D) -> Result<HashMap<String, Layout>, D::Error>
where
    D: Deserializer<'de>,
{
    use serde::de::Error;
    use serde_json::Value;
    
    let value = Value::deserialize(deserializer)?;
    
    match value {
        Value::Array(arr) if arr.is_empty() => Ok(HashMap::new()),
        Value::Object(_) => serde_json::from_value(value).map_err(Error::custom),
        _ => Err(Error::custom("expected an object or empty array for layouts")),
    }
}

#[derive(Debug, Deserialize)]
pub struct DownloadData {
    pub repo: RepoInfo,
    pub tree: Vec<TreeNode>,
    #[serde(deserialize_with = "deserialize_layouts")]
    pub layouts: HashMap<String, Layout>,
    #[serde(default, rename = "isAdmin")]
    pub is_admin: bool,
}

#[derive(Debug, Deserialize)]
pub struct RepoInfo {
    pub id: String,
}

#[derive(Debug, Deserialize)]
pub struct AtomizationStatusResponse {
    pub status_id: String,
}
