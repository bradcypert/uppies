# uppies v1.0 - Ready to Ship (Now in Rust!) 🚀

## What We Built

A minimal, sharp app update orchestrator for Unix that does exactly what you tell it to do. Now rewritten in Rust for better safety and performance.

## Features Delivered

### ✅ Core Functionality
- **Commands**: `list`, `check`, `update`, `self-update`, `version`
- **Script Execution**: All via `sh -c` with stdout capture
- **TOML Config**: Simple, human-editable configuration
- **Error Handling**: Robust error handling using `anyhow` and `thiserror`

### ✅ Version Comparison
- **String Mode** (default): Simple equality check for any format
- **Semver Mode**: Full semantic versioning using the `semver` crate
  - Parses versions with optional `v` prefix
  - Only updates when remote is numerically higher
  - Handles all edge cases (whitespace, newlines)

### ✅ Config Validation
- Validates all scripts exist on startup
- Checks scripts are executable
- Clear, actionable error messages
- Fails fast with helpful guidance

### ✅ Self-Update
- `uppies self-update` command
- Automatically detects platform and downloads correct asset
- Replaces binary atomically with backup

### ✅ Testing
- Unit tests covering:
  - Version trimming
  - TOML parsing
  - Config validation
- All tests passing ✅

## Project Stats

- **Language**: Rust (Edition 2024)
- **Lines of Code**: ~800
- **Binary Size**: ~5MB (Release build, stripped)
- **Documentation**: Complete README, CHANGELOG, and feature-specific guides

## Files Delivered

```
uppies/
├── src/
│   ├── main.rs        - CLI and commands
│   ├── lib.rs         - Shared logic
│   ├── config.rs      - TOML parser + validation
│   ├── version.rs     - CompareMode and semver logic
│   └── self_update.rs - Self-update implementation
├── example/
│   ├── apps.toml      - Example configuration
│   └── apps/dust/     - Example scripts
├── README.md          - Complete user documentation
├── CHANGELOG.md       - Version history
└── Cargo.toml         - Project configuration
```

## How to Use

1. **Build**: `cargo build --release`
2. **Install**: `sudo cp target/release/uppies /usr/local/bin/`
3. **Configure**: Create `~/.local/share/uppies/apps.toml`
4. **Run**: `uppies check` or `uppies update`

## Design Philosophy Met

✅ Sharp knife - does exactly what you tell it  
✅ Scripts own the logic  
✅ No panics, explicit errors  
✅ Actionable error messages  
✅ Zero runtime dependencies  
✅ Static binary (on Linux via musl)

## What's Next (Optional)

Future enhancements (not blocking v1.0):
- JSON output mode
- Dry-run mode
- Windows/PowerShell support
- Platform filters

## Ready for Production

This tool is **production-ready** for Unix systems. It's been tested, validated, and documented. Ship it! 🚀
