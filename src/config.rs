use crate::version::CompareMode;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ScriptConfig {
    File { file: String },
    Inline { inline: String },
}

impl ScriptConfig {
    pub fn as_command(&self) -> &str {
        match self {
            Self::File { file } => file,
            Self::Inline { inline } => inline,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct App {
    pub name: String,
    pub description: Option<String>,
    pub local: ScriptConfig,
    pub remote: ScriptConfig,
    pub update: ScriptConfig,
    #[serde(rename = "compare", default)]
    pub compare_mode: CompareMode,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    #[serde(rename = "app")]
    pub apps: Vec<App>,
}

impl Config {
    pub fn load_from_file(path: &Path) -> anyhow::Result<Self> {
        let content = fs::read_to_string(path)?;
        let config: Config = toml::from_str(&content)?;
        Ok(config)
    }

    pub fn validate(&self) -> anyhow::Result<()> {
        for app in &self.apps {
            if app.name.is_empty() {
                return Err(anyhow::anyhow!("App name must not be empty"));
            }
            validate_script_config(&app.local)?;
            validate_script_config(&app.remote)?;
            validate_script_config(&app.update)?;
        }
        Ok(())
    }
}

fn validate_script_config(config: &ScriptConfig) -> anyhow::Result<()> {
    let ScriptConfig::File { file } = config else {
        return Ok(()); // inline scripts are validated at runtime
    };

    let metadata =
        fs::metadata(file).map_err(|e| anyhow::anyhow!("Failed to stat script {}: {}", file, e))?;

    if !metadata.is_file() {
        return Err(anyhow::anyhow!("Script path {} is not a file", file));
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        if (metadata.mode() & 0o111) == 0 {
            return Err(anyhow::anyhow!(
                "Script {} is not executable (chmod +x)",
                file
            ));
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_file_script() {
        let toml_str = r#"
[[app]]
name = "dust"
description = "du replacement"

[app.local]
file = "/tmp/local.sh"

[app.remote]
file = "/tmp/remote.sh"

[app.update]
file = "/tmp/update.sh"
"#;
        let config: Config = toml::from_str(toml_str).unwrap();
        assert_eq!(config.apps.len(), 1);
        assert_eq!(config.apps[0].name, "dust");
        assert_eq!(config.apps[0].compare_mode, CompareMode::String);
        assert_eq!(config.apps[0].local.as_command(), "/tmp/local.sh");
    }

    #[test]
    fn test_parse_inline_script() {
        let toml_str = r#"
[[app]]
name = "myapp"

[app.local]
inline = "myapp --version"

[app.remote]
inline = "curl -s https://example.com/version"

[app.update]
inline = "brew upgrade myapp"
"#;
        let config: Config = toml::from_str(toml_str).unwrap();
        assert_eq!(config.apps[0].local.as_command(), "myapp --version");
        assert_eq!(
            config.apps[0].remote.as_command(),
            "curl -s https://example.com/version"
        );
    }
}
