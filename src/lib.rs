use std::process::{Command, Stdio};

pub struct ScriptResult {
    pub stdout: String,
    pub exit_code: i32,
}

pub fn run_script(script_path: &str) -> std::io::Result<ScriptResult> {
    let output = Command::new("sh")
        .arg("-c")
        .arg(script_path)
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .output()?;

    Ok(ScriptResult {
        stdout: String::from_utf8_lossy(&output.stdout).to_string(),
        exit_code: output.status.code().unwrap_or(1),
    })
}

/// Trims surrounding whitespace and strips a leading `v` (e.g. `"v1.2.3\n"` → `"1.2.3"`).
pub fn trim_version(version: &str) -> &str {
    version.trim().trim_start_matches('v')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trim_version() {
        assert_eq!(trim_version("1.2.3\n"), "1.2.3");
        assert_eq!(trim_version("  abc123  "), "abc123");
        assert_eq!(trim_version("v1.2.3\n"), "1.2.3");
        assert_eq!(trim_version("  v2.0.0  "), "2.0.0");
    }
}
