use serde::Deserialize;
use std::fs;
use std::process::Command;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Platform {
    LinuxX86_64,
    LinuxAarch64,
    MacosX86_64,
    MacosAarch64,
}

impl Platform {
    pub fn current() -> anyhow::Result<Self> {
        let os = std::env::consts::OS;
        let arch = std::env::consts::ARCH;

        match (os, arch) {
            ("linux", "x86_64") => Ok(Self::LinuxX86_64),
            ("linux", "aarch64") => Ok(Self::LinuxAarch64),
            ("macos", "x86_64") => Ok(Self::MacosX86_64),
            ("macos", "aarch64") => Ok(Self::MacosAarch64),
            _ => anyhow::bail!("Unsupported platform: {}-{}", os, arch),
        }
    }

    pub fn asset_name(&self) -> &'static str {
        match self {
            Self::LinuxX86_64 => "uppies-linux-x86_64.tar.gz",
            Self::LinuxAarch64 => "uppies-linux-aarch64.tar.gz",
            Self::MacosX86_64 => "uppies-macos-x86_64.tar.gz",
            Self::MacosAarch64 => "uppies-macos-aarch64.tar.gz",
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

pub fn fetch_latest_release(repo: &str) -> anyhow::Result<ReleaseInfo> {
    let url = format!("https://api.github.com/repos/{}/releases/latest", repo);

    let output = Command::new("curl")
        .arg("-sL")
        .arg("-H")
        .arg("Accept: application/vnd.github+json")
        .arg(&url)
        .output()?;

    if !output.status.success() {
        anyhow::bail!("Failed to fetch release info");
    }

    let release: ReleaseInfo = serde_json::from_slice(&output.stdout)?;
    Ok(release)
}

pub fn download_and_extract(url: &str, dest_dir: &str) -> anyhow::Result<()> {
    let tmp_path = format!("{}/uppies-download.tar.gz", dest_dir);

    let status = Command::new("curl")
        .arg("-sL")
        .arg("-o")
        .arg(&tmp_path)
        .arg(url)
        .status()?;

    if !status.success() {
        anyhow::bail!("Download failed");
    }

    let status = Command::new("tar")
        .arg("-xzf")
        .arg(&tmp_path)
        .arg("-C")
        .arg(dest_dir)
        .status()?;

    let _ = fs::remove_file(&tmp_path);

    if !status.success() {
        anyhow::bail!("Extraction failed");
    }

    Ok(())
}

pub fn replace_binary(new_binary_path: &str, current_binary_path: &str) -> anyhow::Result<()> {
    // Backup existing binary
    let backup_path = format!("{}.backup", current_binary_path);
    let _ = fs::remove_file(&backup_path);
    fs::copy(current_binary_path, &backup_path)?;

    // Stage in the same directory as the target so rename is always on the same
    // filesystem (rename(2) is atomic; cross-device rename would fail with EXDEV).
    let staged_path = format!("{}.new", current_binary_path);
    fs::copy(new_binary_path, &staged_path)?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&staged_path)?.permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&staged_path, perms)?;
    }

    // Atomic replace
    fs::rename(&staged_path, current_binary_path)?;

    Ok(())
}
