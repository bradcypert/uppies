use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

#[derive(Debug)]
pub struct ScriptResult {
    pub stdout: String,
    pub exit_code: i32,
}

/// Runs a shell script, optionally killing it after `timeout_secs` seconds.
/// Returns `Err` if the process could not be spawned.
pub fn run_script(script: &str, timeout_secs: Option<u64>) -> std::io::Result<ScriptResult> {
    let child = Command::new("sh")
        .arg("-c")
        .arg(script)
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()?;

    let Some(secs) = timeout_secs else {
        // No timeout — simple blocking wait.
        let output = child.wait_with_output()?;
        return Ok(ScriptResult {
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            exit_code: output.status.code().unwrap_or(1),
        });
    };

    // With timeout: move the child into a worker thread and wait on a channel
    // with a deadline. If the deadline passes, kill the process.
    let child_id = child.id();
    let (tx, rx) = mpsc::channel::<std::io::Result<std::process::Output>>();

    thread::spawn(move || {
        let _ = tx.send(child.wait_with_output());
    });

    match rx.recv_timeout(Duration::from_secs(secs)) {
        Ok(Ok(output)) => Ok(ScriptResult {
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            exit_code: output.status.code().unwrap_or(1),
        }),
        Ok(Err(e)) => Err(e),
        Err(_timeout) => {
            kill_process(child_id);
            Err(std::io::Error::new(
                std::io::ErrorKind::TimedOut,
                format!("script timed out after {} seconds", secs),
            ))
        }
    }
}

/// Best-effort process kill. On Unix uses SIGKILL via the `kill` command so we
/// don't need the `libc` crate. On Windows uses `taskkill`.
fn kill_process(pid: u32) {
    #[cfg(unix)]
    {
        let _ = Command::new("kill").args(["-9", &pid.to_string()]).output();
    }
    #[cfg(windows)]
    {
        let _ = Command::new("taskkill")
            .args(["/F", "/PID", &pid.to_string()])
            .output();
    }
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

    #[test]
    fn test_run_script_success() {
        let res = run_script("echo hello", None).unwrap();
        assert_eq!(res.exit_code, 0);
        assert_eq!(res.stdout.trim(), "hello");
    }

    #[test]
    fn test_run_script_exit_code() {
        let res = run_script("exit 1", None).unwrap();
        assert_eq!(res.exit_code, 1);
    }

    #[test]
    fn test_run_script_timeout() {
        let err = run_script("sleep 10", Some(1)).unwrap_err();
        assert_eq!(err.kind(), std::io::ErrorKind::TimedOut);
    }

    #[test]
    fn test_run_script_completes_within_timeout() {
        let res = run_script("echo done", Some(5)).unwrap();
        assert_eq!(res.exit_code, 0);
        assert_eq!(res.stdout.trim(), "done");
    }
}
