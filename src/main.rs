use clap::{Parser, Subcommand};
use semver::Version;
use std::fs;

mod config;
mod self_update;
mod version;

use crate::config::{App, Config};
use crate::version::CompareMode;
use uppies::{run_script, trim_version};

#[derive(Parser)]
#[command(name = "uppies")]
#[command(about = "app update orchestrator", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// List all registered apps
    List,
    /// Check local vs remote versions
    Check,
    /// Update app(s) if versions differ
    Update {
        /// Name of the app to update
        app: Option<String>,
        /// Bypass version check
        #[arg(long)]
        force: bool,
    },
    /// Update uppies itself
    SelfUpdate,
    /// Show version information
    Version,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    match Cli::parse().command {
        Commands::List => cmd_list()?,
        Commands::Check => cmd_check()?,
        Commands::Update { app, force } => cmd_update(app, force)?,
        Commands::SelfUpdate => cmd_self_update()?,
        Commands::Version => cmd_version(),
    }
    Ok(())
}

fn cmd_list() -> anyhow::Result<()> {
    let config = load_config()?;
    if config.apps.is_empty() {
        println!("No apps registered");
    } else {
        for app in config.apps {
            if let Some(desc) = app.description {
                println!("{:<20} {}", app.name, desc);
            } else {
                println!("{}", app.name);
            }
        }
    }
    Ok(())
}

fn cmd_check() -> anyhow::Result<()> {
    let config = load_config()?;
    config.validate()?;

    for app in config.apps {
        let Some((local_ver, remote_ver)) = fetch_versions(&app) else {
            continue;
        };
        let update_needed = match needs_update(app.compare_mode, &local_ver, &remote_ver) {
            Ok(v) => v,
            Err(e) => {
                eprintln!("{}: {}", app.name, e);
                continue;
            }
        };

        if update_needed {
            println!(
                "{:<20} {:<15} → {:<15} (update available)",
                app.name, local_ver, remote_ver
            );
        } else {
            println!("{:<20} {:<15} (up to date)", app.name, local_ver);
        }
    }
    Ok(())
}

fn cmd_update(app_name: Option<String>, force: bool) -> anyhow::Result<()> {
    let config = load_config()?;
    config.validate()?;

    for app in config.apps {
        if let Some(ref target) = app_name
            && &app.name != target
        {
            continue;
        }

        let should_update = if force {
            true
        } else {
            let Some((local_ver, remote_ver)) = fetch_versions(&app) else {
                continue;
            };
            let update_needed = match needs_update(app.compare_mode, &local_ver, &remote_ver) {
                Ok(v) => v,
                Err(e) => {
                    eprintln!("{}: {}", app.name, e);
                    continue;
                }
            };

            if update_needed {
                println!("{}: updating {} → {}", app.name, local_ver, remote_ver);
            } else {
                println!("{}: already up to date ({})", app.name, local_ver);
            }
            update_needed
        };

        if should_update {
            println!("{}: running update script...", app.name);
            match run_script(app.update.as_command()) {
                Ok(res) if res.exit_code == 0 => println!("{}: update complete", app.name),
                _ => eprintln!("{}: update script failed", app.name),
            }
        }
    }
    Ok(())
}

fn cmd_self_update() -> anyhow::Result<()> {
    let repo =
        std::env::var("UPPIES_REPO").unwrap_or_else(|_| "bradcypert/uppies".to_string());
    let current_version = self_update::get_current_version();
    println!("Checking for updates...");

    let release = self_update::fetch_latest_release(&repo)?;
    let latest_version = trim_version(&release.version);

    println!("Current version: {}", current_version);
    println!("Latest version:  {}", latest_version);

    let current_sem = Version::parse(current_version)?;
    let latest_sem = Version::parse(latest_version)?;

    if current_sem >= latest_sem {
        println!(
            "{}",
            if current_sem == latest_sem {
                "Already up to date!"
            } else {
                "Current version is newer than latest release"
            }
        );
        return Ok(());
    }

    println!("\nDownloading uppies {}...", release.version);

    let platform = self_update::Platform::current()?;
    let asset_name = platform.asset_name();
    let asset = release
        .assets
        .into_iter()
        .find(|a| a.name == asset_name)
        .ok_or_else(|| anyhow::anyhow!("No asset found for platform: {}", asset_name))?;

    let tmp_dir = format!(
        "/tmp/uppies-update-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_secs()
    );
    fs::create_dir_all(&tmp_dir)?;

    self_update::download_and_extract(&asset.browser_download_url, &tmp_dir)?;

    let exe_path = std::env::current_exe()?;
    let new_binary = format!("{}/uppies", tmp_dir);

    println!("Installing...");
    self_update::replace_binary(
        &new_binary,
        exe_path.to_str().ok_or_else(|| anyhow::anyhow!("Invalid exe path"))?,
    )?;

    let _ = fs::remove_dir_all(&tmp_dir);
    println!("\n✓ Successfully updated to version {}!", latest_version);
    Ok(())
}

fn cmd_version() {
    println!("uppies version {}", self_update::get_current_version());
}

fn load_config() -> anyhow::Result<Config> {
    let home =
        std::env::var("HOME").map_err(|_| anyhow::anyhow!("HOME environment variable not set"))?;
    let path = std::path::PathBuf::from(home).join(".local/share/uppies/apps.toml");
    Config::load_from_file(&path)
}

/// Runs both version scripts for an app and returns `(local_ver, remote_ver)`.
/// Prints an error and returns `None` if either script fails.
fn fetch_versions(app: &App) -> Option<(String, String)> {
    let local_out = match run_script(app.local.as_command()) {
        Ok(res) if res.exit_code == 0 => res.stdout,
        _ => {
            eprintln!("{}: local version script failed", app.name);
            return None;
        }
    };
    let remote_out = match run_script(app.remote.as_command()) {
        Ok(res) if res.exit_code == 0 => res.stdout,
        _ => {
            eprintln!("{}: remote version script failed", app.name);
            return None;
        }
    };
    Some((
        trim_version(&local_out).to_string(),
        trim_version(&remote_out).to_string(),
    ))
}

/// Returns `true` if an update is needed, `false` if up to date.
/// Returns `Err` if semver versions could not be parsed.
fn needs_update(
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
