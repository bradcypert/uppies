use clap::{CommandFactory, Parser, Subcommand, ValueEnum};
use clap_complete::{Shell, generate};
use semver::Version;
use serde::Serialize;
use std::cmp::Ordering;
use std::fs;
use std::io::{self, BufRead, Write};
use std::path::PathBuf;
use std::thread;
use std::time::Duration;

mod config;
mod history;
mod self_update;
mod topo;
mod version;

use crate::config::{App, Config, ScriptConfig};
use crate::version::{CompareMode, needs_update};
use uppies::{run_script, trim_version};

// ---------------------------------------------------------------------------
// CLI definition
// ---------------------------------------------------------------------------

#[derive(Clone, ValueEnum)]
enum OutputFormat {
    Text,
    Json,
}

#[derive(Parser)]
#[command(name = "uppies")]
#[command(about = "app update orchestrator", long_about = None)]
struct Cli {
    /// Path to config file (overrides the default ~/.config/uppies/apps.toml)
    #[arg(long, global = true)]
    config: Option<PathBuf>,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// List all registered apps
    List {
        /// Output format
        #[arg(long, value_enum, default_value = "text")]
        format: OutputFormat,
        /// Only show apps with this tag (may be repeated)
        #[arg(long)]
        tag: Vec<String>,
    },
    /// Check local vs remote versions
    Check {
        /// Output format
        #[arg(long, value_enum, default_value = "text")]
        format: OutputFormat,
        /// Only check apps with this tag (may be repeated)
        #[arg(long)]
        tag: Vec<String>,
        /// Re-run check repeatedly on an interval
        #[arg(long)]
        watch: bool,
        /// Seconds between checks when --watch is active (default: 300)
        #[arg(long, default_value = "300")]
        interval: u64,
    },
    /// Update app(s) if versions differ
    Update {
        /// Name of a specific app to update (omit to update all)
        app: Option<String>,
        /// Bypass version check and force update
        #[arg(long)]
        force: bool,
        /// Show what would be updated without running update scripts
        #[arg(long)]
        dry_run: bool,
        /// Run updates concurrently (respects dependency ordering)
        #[arg(long)]
        parallel: bool,
        /// Only update apps with this tag (may be repeated)
        #[arg(long)]
        tag: Vec<String>,
        /// Skip this app even if it would otherwise be updated (may be repeated)
        #[arg(long)]
        exclude: Vec<String>,
    },
    /// Update uppies itself
    SelfUpdate,
    /// Show version information
    Version,
    /// Open the config file in $VISUAL/$EDITOR
    Edit,
    /// Add a new app to the config interactively
    Add,
    /// Remove a registered app from the config
    Remove {
        /// Name of the app to remove
        app: String,
        /// Skip confirmation prompt
        #[arg(long, short = 'y')]
        yes: bool,
    },
    /// Show update history
    History {
        /// Number of recent entries to display
        #[arg(long, short = 'n', default_value = "20")]
        lines: usize,
    },
    /// Print the path of the history log
    HistoryPath,
    /// Generate shell completion script
    Completions {
        /// Shell to generate completions for
        shell: Shell,
    },
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    let config_override = cli.config;

    match cli.command {
        Commands::List { format, tag } => cmd_list(config_override, format, tag)?,
        Commands::Check {
            format,
            tag,
            watch,
            interval,
        } => cmd_check(config_override, format, tag, watch, interval)?,
        Commands::Update {
            app,
            force,
            dry_run,
            parallel,
            tag,
            exclude,
        } => cmd_update(config_override, app, force, dry_run, parallel, tag, exclude)?,
        Commands::SelfUpdate => cmd_self_update()?,
        Commands::Version => cmd_version(),
        Commands::Edit => cmd_edit(config_override)?,
        Commands::Add => cmd_add(config_override)?,
        Commands::Remove { app, yes } => cmd_remove(config_override, app, yes)?,
        Commands::History { lines } => cmd_history(lines)?,
        Commands::HistoryPath => cmd_history_path()?,
        Commands::Completions { shell } => cmd_completions(shell),
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// list
// ---------------------------------------------------------------------------

#[derive(Serialize)]
struct ListEntry {
    name: String,
    description: Option<String>,
    tags: Vec<String>,
    pinned: bool,
}

fn cmd_list(
    config_override: Option<PathBuf>,
    format: OutputFormat,
    tag_filter: Vec<String>,
) -> anyhow::Result<()> {
    let config = load_config(config_override)?;

    let apps: Vec<&App> = config
        .apps
        .iter()
        .filter(|a| tag_matches(&a.tags, &tag_filter))
        .collect();

    if apps.is_empty() {
        match format {
            OutputFormat::Json => println!("[]"),
            OutputFormat::Text => println!("No apps registered"),
        }
        return Ok(());
    }

    match format {
        OutputFormat::Text => {
            for app in apps {
                let pinned_marker = if app.pinned { " [pinned]" } else { "" };
                let tags_str = if app.tags.is_empty() {
                    String::new()
                } else {
                    format!(" [{}]", app.tags.join(", "))
                };
                if let Some(ref desc) = app.description {
                    println!("{:<20} {}{}{}", app.name, desc, tags_str, pinned_marker);
                } else {
                    println!("{}{}{}", app.name, tags_str, pinned_marker);
                }
            }
        }
        OutputFormat::Json => {
            let entries: Vec<ListEntry> = apps
                .iter()
                .map(|a| ListEntry {
                    name: a.name.clone(),
                    description: a.description.clone(),
                    tags: a.tags.clone(),
                    pinned: a.pinned,
                })
                .collect();
            println!("{}", serde_json::to_string_pretty(&entries)?);
        }
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// check
// ---------------------------------------------------------------------------

#[derive(Serialize)]
struct CheckEntry {
    name: String,
    local: Option<String>,
    remote: Option<String>,
    update_available: Option<bool>,
    pinned: bool,
    error: Option<String>,
}

fn cmd_check(
    config_override: Option<PathBuf>,
    format: OutputFormat,
    tag_filter: Vec<String>,
    watch: bool,
    interval: u64,
) -> anyhow::Result<()> {
    loop {
        run_check(&config_override, &format, &tag_filter)?;
        if !watch {
            break;
        }
        println!(
            "\n(watching — next check in {} seconds, Ctrl-C to stop)",
            interval
        );
        thread::sleep(Duration::from_secs(interval));
        println!();
    }
    Ok(())
}

fn run_check(
    config_override: &Option<PathBuf>,
    format: &OutputFormat,
    tag_filter: &[String],
) -> anyhow::Result<()> {
    let config = load_config(config_override.clone())?;
    config.validate()?;

    let apps: Vec<&App> = config
        .apps
        .iter()
        .filter(|a| tag_matches(&a.tags, tag_filter))
        .collect();

    let mut entries: Vec<CheckEntry> = Vec::new();

    for app in apps {
        if app.pinned {
            match format {
                OutputFormat::Text => {
                    println!("{:<20} (pinned — skipped)", app.name);
                }
                OutputFormat::Json => {
                    entries.push(CheckEntry {
                        name: app.name.clone(),
                        local: None,
                        remote: None,
                        update_available: None,
                        pinned: true,
                        error: None,
                    });
                }
            }
            continue;
        }

        match fetch_versions(app) {
            Err(e) => match format {
                OutputFormat::Text => eprintln!("{}: {}", app.name, e),
                OutputFormat::Json => entries.push(CheckEntry {
                    name: app.name.clone(),
                    local: None,
                    remote: None,
                    update_available: None,
                    pinned: false,
                    error: Some(e.to_string()),
                }),
            },
            Ok((local_ver, remote_ver)) => {
                let update_needed = match needs_update(app.compare_mode, &local_ver, &remote_ver) {
                    Ok(v) => v,
                    Err(e) => {
                        match format {
                            OutputFormat::Text => eprintln!("{}: {}", app.name, e),
                            OutputFormat::Json => entries.push(CheckEntry {
                                name: app.name.clone(),
                                local: Some(local_ver),
                                remote: Some(remote_ver),
                                update_available: None,
                                pinned: false,
                                error: Some(e.to_string()),
                            }),
                        }
                        continue;
                    }
                };

                match format {
                    OutputFormat::Text => {
                        if update_needed {
                            println!(
                                "{:<20} {:<15} → {:<15} (update available)",
                                app.name, local_ver, remote_ver
                            );
                        } else {
                            println!("{:<20} {:<15} (up to date)", app.name, local_ver);
                        }
                    }
                    OutputFormat::Json => {
                        entries.push(CheckEntry {
                            name: app.name.clone(),
                            local: Some(local_ver),
                            remote: Some(remote_ver),
                            update_available: Some(update_needed),
                            pinned: false,
                            error: None,
                        });
                    }
                }
            }
        }
    }

    if matches!(format, OutputFormat::Json) {
        println!("{}", serde_json::to_string_pretty(&entries)?);
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// update
// ---------------------------------------------------------------------------

fn cmd_update(
    config_override: Option<PathBuf>,
    app_name: Option<String>,
    force: bool,
    dry_run: bool,
    parallel: bool,
    tag_filter: Vec<String>,
    exclude: Vec<String>,
) -> anyhow::Result<()> {
    let config = load_config(config_override)?;
    config.validate()?;

    if dry_run {
        println!("(dry run — no update scripts will be executed)");
    }

    // Filter apps by name, tag, pinned, and exclude list.
    let apps: Vec<App> = config
        .apps
        .into_iter()
        .filter(|a| {
            if let Some(ref target) = app_name
                && &a.name != target
            {
                return false;
            }
            if a.pinned {
                println!("{}: pinned — skipping", a.name);
                return false;
            }
            if exclude.contains(&a.name) {
                println!("{}: excluded — skipping", a.name);
                return false;
            }
            tag_matches(&a.tags, &tag_filter)
        })
        .collect();

    if apps.is_empty() {
        println!("No apps to update.");
        return Ok(());
    }

    // Resolve dependency order into levels. Within each level apps are
    // independent and can be run in parallel when --parallel is set.
    let levels = topo::topological_levels(apps)?;

    for level in levels {
        if parallel && level.len() > 1 {
            run_level_parallel(level, force, dry_run)?;
        } else {
            for app in level {
                run_one_update(&app, force, dry_run)?;
            }
        }
    }

    Ok(())
}

/// Runs all apps in a single dependency level concurrently.
fn run_level_parallel(apps: Vec<App>, force: bool, dry_run: bool) -> anyhow::Result<()> {
    let handles: Vec<_> = apps
        .into_iter()
        .map(|app| {
            thread::spawn(
                move || match run_one_update_buffered(&app, force, dry_run) {
                    Ok(buf) => buf,
                    Err(e) => format!("{}: error: {}\n", app.name, e),
                },
            )
        })
        .collect();

    for handle in handles {
        match handle.join() {
            Ok(buf) => print!("{}", buf),
            Err(_) => eprintln!("(thread panicked)"),
        }
    }

    Ok(())
}

/// Like `run_one_update` but collects output into a String instead of printing
/// directly, so parallel threads don't interleave their output.
fn run_one_update_buffered(app: &App, force: bool, dry_run: bool) -> anyhow::Result<String> {
    let mut out = String::new();

    let should_update = if force {
        out.push_str(&format!("{}: forcing update\n", app.name));
        true
    } else {
        let (local_ver, remote_ver) =
            fetch_versions(app).map_err(|e| anyhow::anyhow!("{}: {}", app.name, e))?;
        let update_needed = needs_update(app.compare_mode, &local_ver, &remote_ver)
            .map_err(|e| anyhow::anyhow!("{}: {}", app.name, e))?;

        if update_needed {
            out.push_str(&format!(
                "{}: updating {} → {}\n",
                app.name, local_ver, remote_ver
            ));
            if dry_run {
                out.push_str(&format!(
                    "{}: (dry run — skipping update script)\n",
                    app.name
                ));
            } else {
                // Store versions for history log before running
                let from = local_ver.clone();
                let to = remote_ver.clone();
                out.push_str(&format!("{}: running update script...\n", app.name));
                match run_script(app.update.as_command(), app.timeout_secs) {
                    Ok(res) if res.exit_code == 0 => {
                        out.push_str(&format!("{}: update complete\n", app.name));
                        let _ = history::log_update(&app.name, &from, &to);
                    }
                    _ => out.push_str(&format!("{}: update script failed\n", app.name)),
                }
                return Ok(out);
            }
        } else {
            out.push_str(&format!(
                "{}: already up to date ({})\n",
                app.name, local_ver
            ));
        }
        update_needed
    };

    if should_update && !dry_run && force {
        out.push_str(&format!("{}: running update script...\n", app.name));
        match run_script(app.update.as_command(), app.timeout_secs) {
            Ok(res) if res.exit_code == 0 => {
                out.push_str(&format!("{}: update complete\n", app.name));
                let _ = history::log_update(&app.name, "unknown", "forced");
            }
            _ => out.push_str(&format!("{}: update script failed\n", app.name)),
        }
    }

    Ok(out)
}

/// Sequential single-app update with direct stdout/stderr output.
fn run_one_update(app: &App, force: bool, dry_run: bool) -> anyhow::Result<()> {
    let should_update = if force {
        println!("{}: forcing update", app.name);
        true
    } else {
        let (local_ver, remote_ver) = match fetch_versions(app) {
            Ok(v) => v,
            Err(e) => {
                eprintln!("{}: {}", app.name, e);
                return Ok(());
            }
        };
        let update_needed = match needs_update(app.compare_mode, &local_ver, &remote_ver) {
            Ok(v) => v,
            Err(e) => {
                eprintln!("{}: {}", app.name, e);
                return Ok(());
            }
        };

        if update_needed {
            if dry_run {
                println!(
                    "{}: would update {} → {} (dry run)",
                    app.name, local_ver, remote_ver
                );
                return Ok(());
            }
            println!("{}: updating {} → {}", app.name, local_ver, remote_ver);

            let from = local_ver.clone();
            let to = remote_ver.clone();
            println!("{}: running update script...", app.name);
            match run_script(app.update.as_command(), app.timeout_secs) {
                Ok(res) if res.exit_code == 0 => {
                    println!("{}: update complete", app.name);
                    let _ = history::log_update(&app.name, &from, &to);
                }
                _ => eprintln!("{}: update script failed", app.name),
            }
            return Ok(());
        } else {
            println!("{}: already up to date ({})", app.name, local_ver);
        }
        update_needed
    };

    if should_update && force {
        println!("{}: running update script...", app.name);
        match run_script(app.update.as_command(), app.timeout_secs) {
            Ok(res) if res.exit_code == 0 => {
                println!("{}: update complete", app.name);
                let _ = history::log_update(&app.name, "unknown", "forced");
            }
            _ => eprintln!("{}: update script failed", app.name),
        }
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// self-update
// ---------------------------------------------------------------------------

fn cmd_self_update() -> anyhow::Result<()> {
    let repo = std::env::var("UPPIES_REPO").unwrap_or_else(|_| "bradcypert/uppies".to_string());
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
        exe_path
            .to_str()
            .ok_or_else(|| anyhow::anyhow!("Invalid exe path"))?,
    )?;

    let _ = fs::remove_dir_all(&tmp_dir);
    println!("\n✓ Successfully updated to version {}!", latest_version);
    Ok(())
}

// ---------------------------------------------------------------------------
// version
// ---------------------------------------------------------------------------

fn cmd_version() {
    println!("uppies version {}", self_update::get_current_version());
}

// ---------------------------------------------------------------------------
// edit
// ---------------------------------------------------------------------------

fn cmd_edit(config_override: Option<PathBuf>) -> anyhow::Result<()> {
    let path = resolve_config_path(config_override)?;

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let editor = std::env::var("VISUAL")
        .or_else(|_| std::env::var("EDITOR"))
        .map_err(|_| anyhow::anyhow!("No editor found. Set $VISUAL or $EDITOR."))?;

    let status = std::process::Command::new(&editor).arg(&path).status()?;

    if !status.success() {
        anyhow::bail!("editor exited with status {}", status);
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// add
// ---------------------------------------------------------------------------

fn cmd_add(config_override: Option<PathBuf>) -> anyhow::Result<()> {
    let config_path = resolve_config_path(config_override)?;

    println!("Adding a new app to uppies.");
    println!("Press Enter to skip optional fields.\n");

    let name = prompt_required("App name: ")?;

    // Check for duplicate
    {
        let existing = load_config(Some(config_path.clone()))?;
        if existing.apps.iter().any(|a| a.name == name) {
            anyhow::bail!("An app named '{}' is already registered.", name);
        }
    }

    let description = prompt_optional("Description (optional): ")?;

    let tags_raw = prompt_optional("Tags, comma-separated (optional, e.g. dev,shell): ")?;
    let tags: Vec<String> = tags_raw
        .map(|s| {
            s.split(',')
                .map(|t| t.trim().to_string())
                .filter(|t| !t.is_empty())
                .collect()
        })
        .unwrap_or_default();

    let local = prompt_script("Local version script")?;
    let remote = prompt_script("Remote version script")?;
    let update = prompt_script("Update script")?;

    let compare_mode_str =
        prompt_with_default("Compare mode [string/semver] (default: string): ", "string")?;
    let compare_mode = match compare_mode_str.to_lowercase().as_str() {
        "semver" => CompareMode::Semver,
        _ => CompareMode::String,
    };

    let timeout_secs: Option<u64> =
        prompt_optional("Script timeout in seconds (optional): ")?.and_then(|s| s.parse().ok());

    let pinned_str = prompt_with_default("Pin this app (skip during update/check)? [y/N]: ", "n")?;
    let pinned = matches!(pinned_str.to_lowercase().as_str(), "y" | "yes");

    let depends_raw = prompt_optional("Depends on (comma-separated app names, optional): ")?;
    let depends_on: Vec<String> = depends_raw
        .map(|s| {
            s.split(',')
                .map(|d| d.trim().to_string())
                .filter(|d| !d.is_empty())
                .collect()
        })
        .unwrap_or_default();

    // Build TOML snippet and append to file.
    let snippet = build_app_toml_snippet(
        &name,
        description.as_deref(),
        &tags,
        &local,
        &remote,
        &update,
        compare_mode,
        timeout_secs,
        pinned,
        &depends_on,
    );

    println!("\nPreview of the entry that will be added:\n");
    println!("{}", snippet);
    let confirm = prompt_with_default("Add this entry? [Y/n]: ", "y")?;
    if !matches!(confirm.to_lowercase().as_str(), "y" | "yes" | "") {
        println!("Aborted.");
        return Ok(());
    }

    // Ensure the config directory exists and the file ends with a newline
    // before appending.
    if let Some(parent) = config_path.parent() {
        fs::create_dir_all(parent)?;
    }
    let existing = if config_path.exists() {
        fs::read_to_string(&config_path)?
    } else {
        String::new()
    };

    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&config_path)?;

    // Separate entries with a blank line.
    if !existing.is_empty() && !existing.ends_with('\n') {
        file.write_all(b"\n")?;
    }
    if !existing.is_empty() {
        file.write_all(b"\n")?;
    }
    file.write_all(snippet.as_bytes())?;

    println!("Added '{}' to {}.", name, config_path.display());
    Ok(())
}

/// Builds the TOML text for one `[[app]]` entry.
#[allow(clippy::too_many_arguments)]
fn build_app_toml_snippet(
    name: &str,
    description: Option<&str>,
    tags: &[String],
    local: &ScriptConfig,
    remote: &ScriptConfig,
    update: &ScriptConfig,
    compare_mode: CompareMode,
    timeout_secs: Option<u64>,
    pinned: bool,
    depends_on: &[String],
) -> String {
    let mut s = String::from("[[app]]\n");

    s.push_str(&format!("name = {}\n", toml_quote(name)));

    if let Some(desc) = description {
        s.push_str(&format!("description = {}\n", toml_quote(desc)));
    }

    if !tags.is_empty() {
        let list = tags
            .iter()
            .map(|t| toml_quote(t))
            .collect::<Vec<_>>()
            .join(", ");
        s.push_str(&format!("tags = [{}]\n", list));
    }

    if pinned {
        s.push_str("pinned = true\n");
    }

    if let Some(secs) = timeout_secs {
        s.push_str(&format!("timeout_secs = {}\n", secs));
    }

    if compare_mode == CompareMode::Semver {
        s.push_str("compare = \"semver\"\n");
    }

    if !depends_on.is_empty() {
        let list = depends_on
            .iter()
            .map(|d| toml_quote(d))
            .collect::<Vec<_>>()
            .join(", ");
        s.push_str(&format!("depends_on = [{}]\n", list));
    }

    s.push_str("\n[app.local]\n");
    s.push_str(&script_config_toml(local));

    s.push_str("\n[app.remote]\n");
    s.push_str(&script_config_toml(remote));

    s.push_str("\n[app.update]\n");
    s.push_str(&script_config_toml(update));

    s
}

fn script_config_toml(sc: &ScriptConfig) -> String {
    match sc {
        ScriptConfig::File { file } => format!("file = {}\n", toml_quote(file)),
        ScriptConfig::Inline { inline } => format!("inline = {}\n", toml_quote(inline)),
    }
}

/// Returns a TOML basic string literal for `s`.
/// Escapes backslashes, double-quotes, and common control characters.
fn toml_quote(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for ch in s.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

// ---------------------------------------------------------------------------
// remove
// ---------------------------------------------------------------------------

fn cmd_remove(config_override: Option<PathBuf>, app_name: String, yes: bool) -> anyhow::Result<()> {
    let config_path = resolve_config_path(config_override)?;

    if !config_path.exists() {
        anyhow::bail!("Config file not found: {}", config_path.display());
    }

    // Verify the app exists before prompting.
    let content = fs::read_to_string(&config_path)?;
    let doc: toml_edit::DocumentMut = content
        .parse()
        .map_err(|e| anyhow::anyhow!("Failed to parse config: {}", e))?;

    let exists = doc
        .get("app")
        .and_then(|item| item.as_array_of_tables())
        .map(|arr| {
            arr.iter()
                .any(|t| t.get("name").and_then(|v| v.as_str()) == Some(&app_name))
        })
        .unwrap_or(false);

    if !exists {
        anyhow::bail!("App '{}' not found in config.", app_name);
    }

    if !yes {
        let confirm = prompt_with_default(
            &format!("Remove '{}' from the config? [y/N]: ", app_name),
            "n",
        )?;
        if !matches!(confirm.to_lowercase().as_str(), "y" | "yes") {
            println!("Aborted.");
            return Ok(());
        }
    }

    // Use toml_edit to remove the entry while preserving all other formatting
    // and comments.
    let mut doc: toml_edit::DocumentMut = content
        .parse()
        .map_err(|e| anyhow::anyhow!("Failed to parse config: {}", e))?;

    if let Some(toml_edit::Item::ArrayOfTables(arr)) = doc.get_mut("app") {
        // Compute the index first so we don't hold the iterator borrow while
        // calling `remove`, which needs a mutable borrow of `arr`.
        let idx = arr
            .iter()
            .position(|t| t.get("name").and_then(|v| v.as_str()) == Some(&app_name));
        if let Some(idx) = idx {
            arr.remove(idx);
        }
    }

    fs::write(&config_path, doc.to_string())?;
    println!("Removed '{}' from {}.", app_name, config_path.display());
    Ok(())
}

// ---------------------------------------------------------------------------
// history
// ---------------------------------------------------------------------------

fn cmd_history(n: usize) -> anyhow::Result<()> {
    let lines = history::read_history(n)?;
    if lines.is_empty() {
        println!("No update history yet.");
    } else {
        for line in lines {
            println!("{}", line);
        }
    }
    Ok(())
}

fn cmd_history_path() -> anyhow::Result<()> {
    println!("{}", history::history_path()?.display());
    Ok(())
}

// ---------------------------------------------------------------------------
// completions
// ---------------------------------------------------------------------------

fn cmd_completions(shell: Shell) {
    let mut cmd = Cli::command();
    let name = cmd.get_name().to_string();
    generate(shell, &mut cmd, name, &mut io::stdout());
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn resolve_config_path(override_path: Option<PathBuf>) -> anyhow::Result<PathBuf> {
    if let Some(p) = override_path {
        return Ok(p);
    }
    let base = dirs::config_dir()
        .ok_or_else(|| anyhow::anyhow!("Could not determine config directory"))?;
    Ok(base.join("uppies/apps.toml"))
}

fn load_config(override_path: Option<PathBuf>) -> anyhow::Result<Config> {
    Config::load_from_file(&resolve_config_path(override_path)?)
}

/// Runs both version scripts for an app and returns `(local_ver, remote_ver)`.
fn fetch_versions(app: &App) -> anyhow::Result<(String, String)> {
    let local_out = match run_script(app.local.as_command(), app.timeout_secs) {
        Ok(res) if res.exit_code == 0 => res.stdout,
        Ok(_) => anyhow::bail!("local version script exited non-zero"),
        Err(e) => anyhow::bail!("local version script failed: {}", e),
    };
    let remote_out = match run_script(app.remote.as_command(), app.timeout_secs) {
        Ok(res) if res.exit_code == 0 => res.stdout,
        Ok(_) => anyhow::bail!("remote version script exited non-zero"),
        Err(e) => anyhow::bail!("remote version script failed: {}", e),
    };
    Ok((
        trim_version(&local_out).to_string(),
        trim_version(&remote_out).to_string(),
    ))
}

/// Returns `true` if `app_tags` contains at least one tag from `filter`,
/// or if `filter` is empty (no filtering applied).
fn tag_matches(app_tags: &[String], filter: &[String]) -> bool {
    if filter.is_empty() {
        return true;
    }
    filter.iter().any(|f| app_tags.contains(f))
}

// ---------------------------------------------------------------------------
// Interactive prompt helpers
// ---------------------------------------------------------------------------

fn prompt_required(question: &str) -> anyhow::Result<String> {
    loop {
        let s = prompt_line(question)?;
        if !s.is_empty() {
            return Ok(s);
        }
        println!("  (this field is required)");
    }
}

fn prompt_optional(question: &str) -> anyhow::Result<Option<String>> {
    let s = prompt_line(question)?;
    Ok(if s.is_empty() { None } else { Some(s) })
}

fn prompt_with_default(question: &str, default: &str) -> anyhow::Result<String> {
    let s = prompt_line(question)?;
    Ok(if s.is_empty() { default.to_string() } else { s })
}

fn prompt_line(question: &str) -> anyhow::Result<String> {
    print!("{}", question);
    io::stdout().flush()?;
    let stdin = io::stdin();
    let mut line = String::new();
    stdin.lock().read_line(&mut line)?;
    Ok(line.trim().to_string())
}

/// Prompts the user to configure a script (inline command or file path).
fn prompt_script(label: &str) -> anyhow::Result<ScriptConfig> {
    println!("{}:", label);
    let kind = prompt_with_default("  Type [inline/file] (default: inline): ", "inline")?;
    match kind.to_lowercase().as_str() {
        "file" => {
            let path = prompt_required("  File path: ")?;
            Ok(ScriptConfig::File { file: path })
        }
        _ => {
            let cmd = prompt_required("  Inline command: ")?;
            Ok(ScriptConfig::Inline { inline: cmd })
        }
    }
}
