use serde::{Deserialize, Serialize};
use std::fs;
use std::os::unix::fs::MetadataExt;
use crate::version::CompareMode;

#[derive(Debug, Serialize, Deserialize)]
pub struct AppScriptConfig {
    pub script: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct App {
    pub name: String,
    pub description: Option<String>,
    pub local: AppScriptConfig,
    pub remote: AppScriptConfig,
    pub update: AppScriptConfig,
    #[serde(rename = "compare", default)]
    pub compare_mode: CompareMode,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    #[serde(rename = "app")]
    pub apps: Vec<App>,
}

impl Config {
    pub fn load_from_file(path: &str) -> anyhow::Result<Self> {
        let content = fs::read_to_string(path)?;
        let config: Config = toml::from_str(&content)?;
        Ok(config)
    }

    pub fn validate(&self) -> anyhow::Result<()> {
        for app in &self.apps {
            if app.name.is_empty() {
                return Err(anyhow::anyhow!("App name must not be empty"));
            }

            validate_script(&app.local.script)?;
            validate_script(&app.remote.script)?;
            validate_script(&app.update.script)?;
        }
        Ok(())
    }
}

fn validate_script(path: &str) -> anyhow::Result<()> {
    let metadata = fs::metadata(path).map_err(|e| anyhow::anyhow!("Failed to stat script {}: {}", path, e))?;

    if !metadata.is_file() {
        return Err(anyhow::anyhow!("Script path {} is not a file", path));
    }

    let mode = metadata.mode();
    let is_executable = (mode & 0o111) != 0;
    if !is_executable {
        return Err(anyhow::anyhow!("Script {} is not executable (chmod +x)", path));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_toml() {
        let toml_str = r#"
[[app]]
name = "dust"
description = "du replacement"

[app.local]
script = "/tmp/local.sh"

[app.remote]
script = "/tmp/remote.sh"

[app.update]
script = "/tmp/update.sh"
"#;
        let config: Config = toml::from_str(toml_str).unwrap();
        assert_eq!(config.apps.len(), 1);
        assert_eq!(config.apps[0].name, "dust");
        assert_eq!(config.apps[0].compare_mode, CompareMode::String);
    }
}
