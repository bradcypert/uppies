use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum CompareMode {
    String,
    Semver,
}

impl Default for CompareMode {
    fn default() -> Self {
        Self::String
    }
}
