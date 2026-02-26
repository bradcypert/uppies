use semver::Version;
use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CompareMode {
    #[default]
    String,
    Semver,
}

/// Returns `true` if an update is needed, `false` if up to date.
/// Returns `Err` if semver versions could not be parsed.
pub fn needs_update(
    compare_mode: CompareMode,
    local_ver: &str,
    remote_ver: &str,
) -> anyhow::Result<bool> {
    match compare_mode {
        CompareMode::String => Ok(local_ver != remote_ver),
        CompareMode::Semver => {
            let local_sem = Version::parse(local_ver);
            let remote_sem = Version::parse(remote_ver);
            match (local_sem, remote_sem) {
                (Ok(l), Ok(r)) => Ok(l < r),
                _ => anyhow::bail!(
                    "failed to parse semver (local: {}, remote: {})",
                    local_ver,
                    remote_ver
                ),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_string_mode_up_to_date() {
        assert!(!needs_update(CompareMode::String, "1.0.0", "1.0.0").unwrap());
    }

    #[test]
    fn test_string_mode_update_available() {
        assert!(needs_update(CompareMode::String, "abc123", "abc124").unwrap());
    }

    #[test]
    fn test_semver_up_to_date() {
        assert!(!needs_update(CompareMode::Semver, "1.0.0", "1.0.0").unwrap());
    }

    #[test]
    fn test_semver_update_available() {
        assert!(needs_update(CompareMode::Semver, "1.0.0", "1.1.0").unwrap());
    }

    #[test]
    fn test_semver_local_newer() {
        assert!(!needs_update(CompareMode::Semver, "2.0.0", "1.9.9").unwrap());
    }

    #[test]
    fn test_semver_invalid_versions_err() {
        assert!(needs_update(CompareMode::Semver, "not-a-version", "1.0.0").is_err());
    }
}
