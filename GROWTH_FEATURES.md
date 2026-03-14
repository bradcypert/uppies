# Growth Features for uppies

This document identifies high-value features to grow the project based on analysis of the current codebase, philosophy, and user needs.

---

## High Priority

### 1. `uppies add` — Interactive App Configuration Wizard
**Problem:** Users must manually edit TOML to add new apps, which is error-prone.
**Solution:** An interactive `uppies add <name>` command that prompts for the three required scripts and writes the entry to `apps.toml`.
**Impact:** Dramatically lowers the barrier to entry for new users.

### 2. Parallel Updates (`--parallel`)
**Problem:** `uppies update` runs apps sequentially, which is slow when managing many tools.
**Solution:** Add a `--parallel [N]` flag to run updates concurrently (optionally bounded to N threads).
**Impact:** Core workflow becomes much faster for power users with 10+ managed apps.

### 3. Dry Run Mode (`--dry-run`)
**Problem:** Users can't preview what would be updated before committing to changes.
**Solution:** `uppies update --dry-run` runs version checks and prints what *would* be updated without executing update scripts.
**Impact:** Safer workflow; essential for automation/CI contexts.

### 4. App Groups / Tags
**Problem:** Users managing many apps may want to update subsets (e.g., only "dev tools" or "shell plugins").
**Solution:** Allow tagging apps in config (`tags = ["dev", "shell"]`) and filtering by tag: `uppies update --tag dev`.
**Impact:** Makes the tool practical at scale without sacrificing simplicity.

### 5. Shell Completion Generation
**Problem:** No tab completion support, which hurts discoverability and usability.
**Solution:** `uppies completions <bash|zsh|fish|powershell>` using `clap_complete` (already using clap).
**Impact:** Low-effort, high-quality-of-life improvement; standard for modern CLI tools.

---

## Medium Priority

### 6. Update History / Audit Log
**Problem:** No record of when apps were updated or what version they moved from/to.
**Solution:** Append a log entry to `~/.local/share/uppies/history.log` on each update: `2026-03-14 dust 0.8.1 -> 0.8.3`.
**Impact:** Useful for debugging regressions; adds accountability.

### 7. Machine-Readable Output (`--format json`)
**Problem:** `uppies check` and `uppies list` output human-readable text that's hard to parse in scripts.
**Solution:** Add `--format json` (and possibly `--format csv`) flags for structured output.
**Impact:** Unlocks integration with dashboards, status bars, and automation scripts.

### 8. Per-App Script Timeout
**Problem:** A hung script in a remote version check can block `uppies check` indefinitely.
**Solution:** Add an optional `timeout_secs` field per app (and a global default) that kills scripts exceeding the limit.
**Impact:** Makes the tool safe to run in cron jobs and CI pipelines.

### 9. `uppies check --watch` / Periodic Check Mode
**Problem:** Users must remember to run `uppies check` manually.
**Solution:** `uppies check --watch <interval>` polls for updates on a schedule and prints notifications when found. (Distinct from a daemon — it's foreground and exits on Ctrl+C.)
**Impact:** Enables "leave it running in a terminal pane" usage pattern without requiring a daemon.

### 10. Version Pinning (`pinned = true`)
**Problem:** Some apps should never auto-update (e.g., pinned to a known-good version).
**Solution:** Add a `pinned = true` config field that causes `uppies update` and `uppies check` to skip that app (with a visible notice).
**Impact:** Supports workflows where stability matters more than currency.

---

## Lower Priority / Longer Term

### 11. Multiple Config Profiles
**Problem:** Users may want separate configs for different contexts (work vs. personal, per-machine).
**Solution:** `--config <path>` flag to override the default config location; allow `include` directives in TOML to compose multiple files.

### 12. Rollback Support
**Problem:** An update script might install a broken version with no way back.
**Solution:** Before running an update, record the current local version. Add `uppies rollback <app>` that re-runs the update script targeting the previous known version (if the update script supports a version argument).

### 13. `uppies remove <app>`
**Problem:** Removing an app requires manual TOML editing.
**Solution:** `uppies remove <app>` removes the entry from `apps.toml` (with confirmation prompt).

### 14. Dependency Ordering
**Problem:** Some apps must be updated before others (e.g., a plugin manager before its plugins).
**Solution:** Add an optional `depends_on = ["other-app"]` field that ensures topological ordering during `uppies update`.

### 15. Windows Support (Phase 2 — already planned)
**Problem:** README notes Windows as Phase 2.
**Solution:** Replace `sh -c` script execution with `cmd /c` or PowerShell on Windows; test CI on Windows runners. The build pipeline already produces Windows binaries.

---

## Summary Table

| Feature | Effort | Impact | Priority |
|---|---|---|---|
| `uppies add` wizard | Medium | High | High |
| Parallel updates | Medium | High | High |
| Dry run mode | Low | High | High |
| App groups/tags | Medium | High | High |
| Shell completions | Low | Medium | High |
| Update history log | Low | Medium | Medium |
| JSON output | Low | Medium | Medium |
| Script timeouts | Low | Medium | Medium |
| `--watch` mode | Medium | Medium | Medium |
| Version pinning | Low | Medium | Medium |
| Multi-profile config | Medium | Low | Low |
| Rollback support | High | Medium | Low |
| `uppies remove` | Low | Low | Low |
| Dependency ordering | High | Low | Low |
| Windows support | High | High | Low (planned) |
