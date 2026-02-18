use serde::Deserialize;
use std::fs;
use std::io;
use std::os::unix::fs::PermissionsExt;
use std::process::Command;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Platform {
    LinuxX86_64,
    LinuxAarch64,
    MacosX86_64,
    MacosAarch64,
    Unknown,
}

impl Platform {
    pub fn current() -> Self {
        let os = std::env::consts::OS;
        let arch = std::env::consts::ARCH;

        match (os, arch) {
            ("linux", "x86_64") => Self::LinuxX86_64,
            ("linux", "aarch64") => Self::LinuxAarch64,
            ("macos", "x86_64") => Self::MacosX86_64,
            ("macos", "aarch64") => Self::MacosAarch64,
            _ => Self::Unknown,
        }
    }

    pub fn asset_name(&self) -> &'static str {
        match self {
            Self::LinuxX86_64 => "uppies-linux-x86_64.tar.gz",
            Self::LinuxAarch64 => "uppies-linux-aarch64.tar.gz",
            Self::MacosX86_64 => "uppies-macos-x86_64.tar.gz",
            Self::MacosAarch64 => "uppies-macos-aarch64.tar.gz",
            Self::Unknown => "unknown",
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct ReleaseInfo {
    #[serde(rename = "tag_name")]
    pub version: String,
    pub assets: Vec<Asset>,
}

#[derive(Debug, Deserialize)]
pub struct Asset {
    pub name: String,
    pub browser_download_url: String,
}

pub fn get_current_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

pub fn fetch_latest_release(repo: &str) -> Result<ReleaseInfo, Box<dyn std::error::Error>> {
    let url = format!("https://api.github.com/repos/{}/releases/latest", repo);

    let output = Command::new("curl")
        .arg("-sL")
        .arg("-H")
        .arg("Accept: application/vnd.github+json")
        .arg(&url)
        .output()?;

    if !output.status.success() {
        return Err("Failed to fetch release info".into());
    }

    let release: ReleaseInfo = serde_json::from_slice(&output.stdout)?;
    Ok(release)
}

pub fn download_and_extract(url: &str, dest_dir: &str) -> io::Result<()> {
    let tmp_path = format!("{}/uppies-download.tar.gz", dest_dir);

    // Download with curl
    let status = Command::new("curl")
        .arg("-sL")
        .arg("-o")
        .arg(&tmp_path)
        .arg(url)
        .status()?;

    if !status.success() {
        return Err(io::Error::new(io::ErrorKind::Other, "Download failed"));
    }

    // Extract with tar
    let status = Command::new("tar")
        .arg("-xzf")
        .arg(&tmp_path)
        .arg("-C")
        .arg(dest_dir)
        .status()?;

    // Clean up
    let _ = fs::remove_file(&tmp_path);

    if !status.success() {
        return Err(io::Error::new(io::ErrorKind::Other, "Extraction failed"));
    }

    Ok(())
}

pub fn replace_binary(new_binary_path: &str, current_binary_path: &str) -> io::Result<()> {
    // Make executable
    let mut perms = fs::metadata(new_binary_path)?.permissions();
    perms.set_mode(0o755);
    fs::set_permissions(new_binary_path, perms)?;

    // Backup
    let backup_path = format!("{}.backup", current_binary_path);
    let _ = fs::remove_file(&backup_path);
    fs::copy(current_binary_path, &backup_path)?;

    // Replace
    fs::remove_file(current_binary_path)?;
    fs::rename(new_binary_path, current_binary_path)?;

    Ok(())
}
