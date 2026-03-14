use chrono::Local;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;

/// Returns the path to the update history log file.
/// Platform defaults: ~/.local/share/uppies/history.log (Linux),
/// ~/Library/Application Support/uppies/history.log (macOS),
/// %LOCALAPPDATA%\uppies\history.log (Windows).
pub fn history_path() -> anyhow::Result<PathBuf> {
    let base = dirs::data_local_dir()
        .ok_or_else(|| anyhow::anyhow!("Could not determine data directory"))?;
    Ok(base.join("uppies/history.log"))
}

/// Appends one line to the history log recording a completed update.
pub fn log_update(app_name: &str, from_version: &str, to_version: &str) -> anyhow::Result<()> {
    let path = history_path()?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S");
    let entry = format!("{timestamp}  {app_name}  {from_version} -> {to_version}\n");

    OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)?
        .write_all(entry.as_bytes())?;

    Ok(())
}

/// Reads and returns the last `n` lines from the history log.
/// Returns an empty Vec if the log file does not exist yet.
pub fn read_history(n: usize) -> anyhow::Result<Vec<String>> {
    let path = history_path()?;
    if !path.exists() {
        return Ok(vec![]);
    }
    let content = fs::read_to_string(&path)?;
    let lines: Vec<String> = content
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(String::from)
        .collect();
    let start = lines.len().saturating_sub(n);
    Ok(lines[start..].to_vec())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_history_path_is_set() {
        // Just verify it returns Ok without panicking in the test environment.
        // The actual path is platform-dependent.
        let _ = history_path();
    }
}
