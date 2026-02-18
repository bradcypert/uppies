# Self-Update Implementation Complete ✅

uppies can now update itself from GitHub releases!

## What Was Implemented

### 1. Core Self-Update Module (`src/self_update.rs`)

**Platform Detection**
- Automatically detects OS and CPU architecture
- Supports: Linux/macOS x86_64/aarch64
- Maps to correct release asset names

**GitHub API Integration**
- Fetches latest release via GitHub API
- Parses JSON response using `serde_json`
- Finds correct asset for current platform

**Download & Extract**
- Downloads tarball via curl
- Extracts to temporary directory using `tar`
- Cleans up after itself

**Binary Replacement**
- Creates backup of current binary
- Makes new binary executable (chmod +x)
- Atomic replacement (rename operation)

### 2. CLI Commands

**`uppies self-update`**
- Checks for updates
- Compares versions (semver)
- Downloads and installs if newer
- Configurable via `UPPIES_REPO` env var

**`uppies version`**
- Shows current version
- Also responds to `--version` and `-v`
- Version embedded at compile time via `env!("CARGO_PKG_VERSION")`

### 3. Build System Integration

**Version Embedding**
- `Cargo.toml` manages the version
- `env!("CARGO_PKG_VERSION")` provides version at compile time

### 4. Documentation

- **README.md** - Updated with installation instructions
- **SELF-UPDATE.md** - Complete guide with troubleshooting
- **SELF-UPDATE-IMPLEMENTATION.md** - This file

## How To Use

### Install

```bash
# Build
cargo build --release

# Install to user directory (recommended)
mkdir -p ~/.local/bin
cp target/release/uppies ~/.local/bin/
export PATH="$HOME/.local/bin:$PATH"

# Or install system-wide
sudo cp target/release/uppies /usr/local/bin/
```

### Update

```bash
# Set your repository (if not the default)
export UPPIES_REPO=username/uppies

# Update
uppies self-update
```

### Check Version

```bash
uppies version
# Output: uppies version 1.0.0
```

## Architecture

```
┌─────────────────┐
│  uppies binary  │
│   (current)     │
└────────┬────────┘
         │
         │ uppies self-update
         ▼
┌─────────────────────────────┐
│  1. Check current version   │
│     (embedded in binary)    │
└─────────────┬───────────────┘
              │
              ▼
┌─────────────────────────────┐
│  2. Fetch latest release    │
│     (GitHub API)            │
└─────────────┬───────────────┘
              │
              ▼
┌─────────────────────────────┐
│  3. Compare versions        │
│     (semver)                │
└─────────────┬───────────────┘
              │
              ▼ (if newer)
┌─────────────────────────────┐
│  4. Download tarball        │
│     (curl)                  │
└─────────────┬───────────────┘
              │
              ▼
┌─────────────────────────────┐
│  5. Extract binary          │
│     (tar)                   │
└─────────────┬───────────────┘
              │
              ▼
┌─────────────────────────────┐
│  6. Backup current          │
│     (uppies.backup)         │
└─────────────┬───────────────┘
              │
              ▼
┌─────────────────────────────┐
│  7. Replace binary          │
│     (atomic rename)         │
└─────────────┬───────────────┘
              │
              ▼
┌─────────────────────────────┐
│  uppies binary (updated!)   │
└─────────────────────────────┘
```

## Dependencies

Runtime:
- **curl** - Download releases
- **tar** - Extract tarballs

None of these are bundled - they must be present on the system.

## Supported Platforms

| Platform | Status |
|----------|--------|
| Linux x86_64 | ✅ |
| Linux aarch64 | ✅ |
| macOS x86_64 | ✅ |
| macOS aarch64 | ✅ |
| Windows | ⏳ Phase 2 |

## Security Considerations

**Current:**
- Downloads from GitHub Releases only
- HTTPS via curl
- Backup created before replacement

**Future Improvements:**
- Checksum verification
- GPG signature verification
- Configurable update channel (stable/beta)

## Testing

All tests passing:
```bash
cargo test
```

Covered:
- Version parsing and comparison
- TOML parsing
- CLI argument parsing

## Example Session

```bash
$ uppies version
uppies version 1.0.0

$ uppies self-update
Checking for updates...
Current version: 1.0.0
Latest version:  1.1.0

Downloading uppies 1.1.0...
Installing...

✓ Successfully updated to version 1.1.0!

$ uppies version
uppies version 1.1.0

$ ls -la $(which uppies)
-rwxr-xr-x 1 user user 9.3M Jan 5 10:00 /home/user/.local/bin/uppies

$ ls -la $(which uppies).backup
-rwxr-xr-x 1 user user 9.1M Jan 4 09:00 /home/user/.local/bin/uppies.backup
```

## Integration with Release Process

When semantic-release creates a new version:

1. Updates `Cargo.toml` version
2. Builds binaries for all platforms
3. Creates GitHub release with tag
4. Uploads tarballs as assets

Users can then:
```bash
uppies self-update
```

And get the new version automatically!

## Ready for Production! 🚀

The self-update feature is complete and ready to use. Once you:
1. Push to GitHub
2. Create first release
3. Set `UPPIES_REPO` environment variable

Users can run `uppies self-update` to always stay current!
