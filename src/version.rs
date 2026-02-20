use serde::{Deserialize, Serialize};
use semver::Version;

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
