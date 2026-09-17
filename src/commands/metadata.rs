use anyhow::Result;
use serde_json::{json, Value};

/// Matches the server's mb_strlen(..., 'UTF-8') limits (Unicode scalar values).
pub fn field_error(field: &str, value: &str, max: usize, required: bool) -> Option<Value> {
    let length = value.chars().count();
    let code = if required && value.trim().is_empty() {
        "required"
    } else if length > max {
        "max_length"
    } else {
        return None;
    };
    Some(json!({"field": field, "code": code, "actual_length": length, "max_length": max}))
}

pub fn validate(summary: &str, description: Option<&str>) -> Result<()> {
    let fields: Vec<_> = [
        field_error("summary", summary, 128, true),
        field_error("description", description.unwrap_or(""), 512, false),
    ]
    .into_iter()
    .flatten()
    .collect();
    if !fields.is_empty() {
        anyhow::bail!("{}", json!({"code":"VALIDATION_ERROR", "fields":fields}));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn limits_match_unicode_characters_not_bytes() {
        assert!(validate(&"č".repeat(128), Some(&"🦀".repeat(512))).is_ok());
        assert!(validate(&"a".repeat(129), None).is_err());
        assert!(validate("ok", Some(&"🦀".repeat(513))).is_err());
        assert!(validate(" \n", None).is_err());
        assert!(validate("ok", Some("")).is_ok());
        let error = field_error("description", &"é".repeat(513), 512, false).unwrap();
        assert_eq!(error["actual_length"], 513);
        assert_eq!(error["max_length"], 512);
    }
}
