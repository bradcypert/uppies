# Self-Update Feature

uppies can update itself from GitHub releases automatically.

## How It Works

1. **Platform Detection**: Automatically detects your OS and architecture
   - Linux x86_64
   - Linux aarch64
   - macOS x86_64
   - macOS aarch64 (Apple Silicon)

2. **Version Check**: Compares current version with latest GitHub release
   - Uses semantic versioning (the `semver` crate)
   - Only updates if newer version available

3. **Download**: Fetches the correct binary for your platform
   - Uses GitHub Releases API
   - Downloads via `curl`

4. **Installation**: Replaces binary atomically
   - Creates backup of current binary
   - Makes new binary executable (`chmod +x`)
   - Replaces in-place

## Usage

### Basic Self-Update

```bash
uppies self-update
```

This will:
- Check for the latest release
- Compare with current version
- Download and install if newer

### Check Current Version

```bash
uppies version
# or
uppies --version
# or
uppies -v
```

### Custom Repository

By default, uppies looks for releases in `bradcypert/uppies`. To use a different repository:

```bash
export UPPIES_REPO=your-org/your-fork
uppies self-update
```

## Requirements

- **curl**: For downloading releases
- **tar**: For extracting tarballs
- **Write permissions**: On the uppies binary location

## Installation Locations

### System-Wide (requires sudo)

```bash
# Install to /usr/local/bin
sudo cp uppies /usr/local/bin/

# Update (will need sudo)
sudo uppies self-update
```

### User-Local (no sudo needed)

```bash
# Install to ~/.local/bin
mkdir -p ~/.local/bin
cp uppies ~/.local/bin/
export PATH="$HOME/.local/bin:$PATH"

# Update (no sudo needed)
uppies self-update
```

## Troubleshooting

### "Permission denied" Error

If you get a permission error:

1. **Check binary location**:
   ```bash
   which uppies
   ```

2. **Install to user directory** (recommended):
   ```bash
   cp /usr/local/bin/uppies ~/.local/bin/
   export PATH="$HOME/.local/bin:$PATH"
   uppies self-update
   ```

3. **Or use sudo**:
   ```bash
   sudo uppies self-update
   ```

### "Failed to fetch latest release"

Possible causes:
- No internet connection
- curl not installed
- GitHub API rate limit (rare)
- Invalid repository name

Check:
```bash
curl -s https://api.github.com/repos/bradcypert/uppies/releases/latest | grep tag_name
```

### "Unsupported platform"

Currently supported:
- Linux (x86_64, aarch64)
- macOS (x86_64, Apple Silicon)

Windows support planned for Phase 2.

## Security Notes

- Downloads are from GitHub Releases only
- Binary signature verification not yet implemented
- Always review release notes before updating
- Backup binary is kept as `uppies.backup`

## Behind the Scenes

The self-update process:

1. Fetches `https://api.github.com/repos/OWNER/REPO/releases/latest`
2. Parses JSON to find:
   - Latest version number (`tag_name`)
   - Download URL for your platform
3. Downloads tarball to `/tmp/uppies-update-TIMESTAMP/`
4. Extracts binary
5. Makes it executable (`chmod +x`)
6. Creates backup: `uppies.backup`
7. Replaces current binary
8. Cleans up temp files

## Version Embedding

The version number is embedded at compile time via Cargo:

```rust
// embedded in binary
println!("uppies version {}", env!("CARGO_PKG_VERSION"));
```

This ensures the binary always knows its version without external files.
