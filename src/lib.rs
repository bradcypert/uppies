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

pub fn trim_version(version: &str) -> &str {
    version.trim()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trim_version() {
        assert_eq!(trim_version("1.2.3
"), "1.2.3");
        assert_eq!(trim_version("  abc123  "), "abc123");
    }
}
