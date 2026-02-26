use clap::{Parser, Subcommand};
use semver::Version;
use std::cmp::Ordering;
use std::fs;
use std::path::PathBuf;

mod config;
mod self_update;
mod version;

use crate::config::{App, Config};
use crate::version::needs_update;
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
    /// Open the config file in $VISUAL/$EDITOR
    Edit,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    match Cli::parse().command {
        Commands::List => cmd_list()?,
        Commands::Check => cmd_check()?,
        Commands::Update { app, force } => cmd_update(app, force)?,
        Commands::SelfUpdate => cmd_self_update()?,
        Commands::Version => cmd_version(),
        Commands::Edit => cmd_edit()?,
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
        let (local_ver, remote_ver) = match fetch_versions(&app) {
            Ok(v) => v,
            Err(e) => {
                eprintln!("{}: {}", app.name, e);
                continue;
            }
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
            let (local_ver, remote_ver) = match fetch_versions(&app) {
                Ok(v) => v,
                Err(e) => {
                    eprintln!("{}: {}", app.name, e);
                    continue;
                }
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

    match current_sem.cmp(&latest_sem) {
        Ordering::Equal => {
            println!("Already up to date!");
            return Ok(());
        }
        Ordering::Greater => {
            println!("Current version is newer than latest release");
            return Ok(());
        }
        Ordering::Less => {}
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

fn config_path() -> anyhow::Result<PathBuf> {
    let home =
        std::env::var("HOME").map_err(|_| anyhow::anyhow!("HOME environment variable not set"))?;
    Ok(PathBuf::from(home).join(".local/share/uppies/apps.toml"))
}

fn load_config() -> anyhow::Result<Config> {
    Config::load_from_file(&config_path()?)
}

fn cmd_edit() -> anyhow::Result<()> {
    let path = config_path()?;

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let editor = std::env::var("VISUAL")
        .or_else(|_| std::env::var("EDITOR"))
        .map_err(|_| anyhow::anyhow!("No editor found. Set $VISUAL or $EDITOR."))?;

    let status = std::process::Command::new(&editor)
        .arg(&path)
        .status()?;

    if !status.success() {
        anyhow::bail!("editor exited with status {}", status);
    }

    Ok(())
}

/// Runs both version scripts for an app and returns `(local_ver, remote_ver)`.
/// Returns `Err` if either script fails or exits non-zero.
fn fetch_versions(app: &App) -> anyhow::Result<(String, String)> {
    let local_out = match run_script(app.local.as_command()) {
        Ok(res) if res.exit_code == 0 => res.stdout,
        _ => anyhow::bail!("local version script failed"),
    };
    let remote_out = match run_script(app.remote.as_command()) {
        Ok(res) if res.exit_code == 0 => res.stdout,
        _ => anyhow::bail!("remote version script failed"),
    };
    Ok((
        trim_version(&local_out).to_string(),
        trim_version(&remote_out).to_string(),
    ))
}
