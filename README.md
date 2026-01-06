# uppies

A sharp, minimal app update orchestrator for Unix.

## Philosophy

**uppies is NOT:**
- A package manager
- A build system
- An auto-updater daemon (at least initially)
- A sandboxed execution environment

**uppies IS:**
- An orchestrator
- A registry of "how to update this thing"
- A thin wrapper around scripts
- Opinionated about *when* to run scripts, not *how* they work

Think: "git config + cron + sh, but coherent."

## Installation

### From Release Binaries

Download the latest release for your platform:

```bash
# Linux x86_64
curl -L https://github.com/bradcypert/uppies/releases/latest/download/uppies-linux-x86_64.tar.gz | tar xz
sudo mv uppies /usr/local/bin/

# macOS (Apple Silicon)
curl -L https://github.com/bradcypert/uppies/releases/latest/download/uppies-macos-aarch64.tar.gz | tar xz
sudo mv uppies /usr/local/bin/
```

### From Source

```bash
git clone https://github.com/bradcypert/uppies
cd uppies
zig build -Doptimize=ReleaseSafe
sudo cp zig-out/bin/uppies /usr/local/bin/
```

### Updating

Once installed, uppies can update itself:

```bash
uppies self-update
```

## Usage

```bash
uppies list              # List all registered apps
uppies check             # Check local vs remote versions
uppies update [app]      # Update app(s) if versions differ
uppies update --force    # Bypass version check
uppies self-update       # Update uppies itself from GitHub releases
```

## Configuration

Create `~/.local/share/uppies/apps.toml`:

```toml
[[app]]
name = "dust"
description = "dust media server"
compare = "string"  # Optional: "string" (default) or "semver"

[app.local]
script = "/home/user/.local/share/uppies/apps/dust/local_version.sh"

[app.remote]
script = "/home/user/.local/share/uppies/apps/dust/remote_version.sh"

[app.update]
script = "/home/user/.local/share/uppies/apps/dust/update.sh"


[[app]]
name = "fd"
description = "find alternative"
compare = "semver"  # Use semantic versioning comparison

[app.local]
script = "/home/user/.local/share/uppies/apps/fd/local_version.sh"

[app.remote]
script = "/home/user/.local/share/uppies/apps/fd/remote_version.sh"

[app.update]
script = "/home/user/.local/share/uppies/apps/fd/update.sh"
```

### Version Comparison Modes

- **`string`** (default): Simple string equality check
  - If `local != remote` → update available
  - Works with any version format (git SHAs, dates, etc.)

- **`semver`**: Semantic version comparison
  - Parses versions as `MAJOR.MINOR.PATCH`
  - Supports optional `v` prefix (e.g., `v1.2.3`)
  - Only shows updates when remote version is *newer*
  - Example: `1.2.3` vs `1.2.4` → update available
  - Example: `2.0.0` vs `1.9.9` → up to date (already newer)

## Scripts

Each app requires three scripts:

### local_version.sh
Outputs the currently installed version to stdout:
```bash
#!/bin/sh
dust --version | awk '{print $2}'
```

### remote_version.sh
Outputs the latest available version to stdout:
```bash
#!/bin/sh
curl -s https://api.github.com/repos/bootandy/dust/releases/latest | \
  jq -r .tag_name
```

### update.sh
Performs the actual update:
```bash
#!/bin/sh
curl -L "https://github.com/bootandy/dust/releases/latest/download/dust-x86_64-unknown-linux-gnu.tar.gz" | \
  tar xz -C ~/.local/bin/
```

## Filesystem Layout

```
~/.local/share/uppies/
├── apps.toml           # Configuration file
├── apps/
│   └── dust/
│       ├── local_version.sh
│       ├── remote_version.sh
│       └── update.sh
└── logs/               # (future)
```

## Script Rules

- Scripts must write **only** the version to stdout
- Trailing newline allowed
- Exit code ≠ 0 = failure
- All scripts run via `sh -c`
- stderr is inherited
- Scripts must be executable (`chmod +x`)

## Config Validation

uppies validates your configuration on startup:
- All scripts must exist
- All scripts must be executable
- App names must not be empty
- Clear error messages point to the problem

```bash
$ uppies list
Config validation error: Script is not executable (chmod +x)
```

## Version Comparison

**String mode** (default):
- Simple equality check
- Works with any version format

**Semver mode**:
- Parses `MAJOR.MINOR.PATCH` format
- Supports `v` prefix
- Only updates when remote is newer

Set in config:
```toml
[[app]]
name = "myapp"
compare = "semver"  # or "string"
```

## Design Principles

1. **Scripts own the logic** - uppies just orchestrates
2. **No panics** - explicit error handling
3. **Actionable errors** - tell users what went wrong
4. **Zero config assumptions** - all paths explicit
5. **Sharp knife** - does exactly what you tell it

## Example Output

```bash
$ uppies check
dust     1.0.0 → 1.2.1   (update available)
fd       v8.7.0 → v8.7.1 (update available)

$ uppies update dust
dust: updating 1.0.0 → 1.2.1
dust: running update script...
dust: update complete

$ uppies check
dust     1.2.1           (up to date)
fd       v8.7.0 → v8.7.1 (update available)
```

## Platform Support

- ✅ Linux/macOS (Phase 1)
- ⏳ Windows via pwsh (Phase 2)

## Building

Requires Zig 0.15.2+

```bash
zig build                # Debug build
zig build -Doptimize=ReleaseSafe  # Release build
zig build test           # Run tests
```
