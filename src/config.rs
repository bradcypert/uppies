use crate::version::CompareMode;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct App {
    pub name: String,
    pub description: Option<String>,
    pub local: ScriptConfig,
    pub remote: ScriptConfig,
    pub update: ScriptConfig,
    #[serde(rename = "compare", default)]
    pub compare_mode: CompareMode,
    /// Tags for grouping/filtering apps
    #[serde(default)]
    pub tags: Vec<String>,
    /// Skip this app during check/update
    #[serde(default)]
    pub pinned: bool,
    /// Kill scripts after this many seconds (applies to local, remote, and update scripts)
    pub timeout_secs: Option<u64>,
    /// Names of apps that must be updated before this one
    #[serde(default)]
    pub depends_on: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    #[serde(rename = "app", default)]
    pub apps: Vec<App>,
}

impl Config {
    pub fn load_from_file(path: &Path) -> anyhow::Result<Self> {
        if !path.exists() {
            return Ok(Config { apps: vec![] });
        }
        let content = fs::read_to_string(path)?;
        if content.trim().is_empty() {
            return Ok(Config { apps: vec![] });
        }
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

            // Validate depends_on references
            for dep in &app.depends_on {
                if !self.apps.iter().any(|a| &a.name == dep) {
                    return Err(anyhow::anyhow!(
                        "App '{}' depends on '{}' which is not registered",
                        app.name,
                        dep
                    ));
                }
            }
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
        assert!(config.apps[0].tags.is_empty());
        assert!(!config.apps[0].pinned);
        assert!(config.apps[0].timeout_secs.is_none());
        assert!(config.apps[0].depends_on.is_empty());
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

    #[test]
    fn test_parse_new_fields() {
        let toml_str = r#"
[[app]]
name = "myapp"
tags = ["dev", "shell"]
pinned = true
timeout_secs = 30
depends_on = ["other-app"]

[app.local]
inline = "myapp --version"

[app.remote]
inline = "curl -s https://example.com/version"

[app.update]
inline = "brew upgrade myapp"
"#;
        let config: Config = toml::from_str(toml_str).unwrap();
        assert_eq!(config.apps[0].tags, vec!["dev", "shell"]);
        assert!(config.apps[0].pinned);
        assert_eq!(config.apps[0].timeout_secs, Some(30));
        assert_eq!(config.apps[0].depends_on, vec!["other-app"]);
    }

    #[test]
    fn test_empty_config() {
        let config: Config = toml::from_str("").unwrap();
        assert!(config.apps.is_empty());
    }
}
