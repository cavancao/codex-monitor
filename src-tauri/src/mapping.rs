use serde::{Deserialize, Serialize};
use std::{fs, path::{Path, PathBuf}};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FieldRule { pub field: String, pub provider: String, pub relative_path: PathBuf, pub selector: String, pub scale: Option<f64> }

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct FieldMapping { pub base_directory: Option<PathBuf>, pub rules: Vec<FieldRule> }

impl FieldMapping {
    pub fn load(path: &Path) -> Result<Self, String> {
        let content = fs::read_to_string(path).map_err(|e| format!("读取映射失败: {e}"))?;
        let mapping: Self = serde_json::from_str(&content).map_err(|e| format!("映射格式错误: {e}"))?;
        if mapping.rules.iter().any(|r| r.relative_path.is_absolute() || r.relative_path.components().any(|c| matches!(c, std::path::Component::ParentDir))) {
            return Err("relativePath 只能是安全的相对路径".into());
        }
        Ok(mapping)
    }
}

pub fn read_json_value(base: &Path, rule: &FieldRule) -> Result<serde_json::Value, String> {
    let file = base.join(&rule.relative_path);
    let text = fs::read_to_string(file).map_err(|e| format!("只读打开失败: {e}"))?;
    let mut value: serde_json::Value = serde_json::from_str(&text).map_err(|e| e.to_string())?;
    for part in rule.selector.trim_start_matches("$.").split('.') { value = value.get(part).cloned().ok_or_else(|| format!("字段路径不存在: {}", rule.selector))?; }
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_absolute_mapping_paths() {
        let temp = tempfile::tempdir().unwrap(); let file = temp.path().join("map.json");
        fs::write(&file, r#"{"rules":[{"field":"model","provider":"file","relativePath":"C:\\\\secret.json","selector":"$.model","scale":null}]}"#).unwrap();
        assert!(FieldMapping::load(&file).is_err());
    }
}
