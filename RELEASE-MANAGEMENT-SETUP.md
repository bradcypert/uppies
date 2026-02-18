# Release Management Setup Complete ✅

This document describes the automated release management system that has been set up for uppies.

## What Was Set Up

### 1. GitHub Actions Workflows

**`.github/workflows/release.yml`**
- Triggers on every push to `main` branch
- Builds binaries for 4 platforms:
  - Linux x86_64 (musl - static linking via cross)
  - Linux aarch64 (musl - static linking via cross)  
  - macOS x86_64
  - macOS aarch64 (Apple Silicon)
- Runs semantic-release to:
  - Analyze commits
  - Determine version bump
  - Generate changelog
  - Create GitHub release
  - Upload binaries

**`.github/workflows/ci.yml`**
- Runs on PRs and pushes to `main`
- Builds the project (Rust/Cargo)
- Runs test suite
- Checks code formatting and clippy

### 2. Semantic Release Configuration

**`.releaserc.json`**
- Commit analysis using Conventional Commits
- Release rules:
  - `feat:` → minor version bump (1.0.0 → 1.1.0)
  - `fix:`, `perf:` → patch bump (1.0.0 → 1.0.1)
  - `BREAKING CHANGE:` → major bump (1.0.0 → 2.0.0)
- Automated changelog generation
- Version updates in `Cargo.toml`
- GitHub release with binary attachments

### 3. Supporting Files

**`package.json`**
- Defines npm dependencies for semantic-release
- Project metadata

**`scripts/update-cargo-version.js`**
- Node.js script to update version in `Cargo.toml`
- Called automatically by semantic-release

**`RELEASE-PROCESS.md`**
- Complete documentation of the release process
- Commit message format guide
- Examples

### 4. Updated Files

**`.gitignore`**
- Added `node_modules/`
- Added `package-lock.json`
- Added `dist/`
- Added `target/` (Cargo build artifacts)

**`CHANGELOG.md`**
- Restructured for semantic-release compatibility
- Moved content to [Unreleased] section

**`Cargo.toml`**
- Version updated to `1.0.0` for initial release

## How It Works

### Automatic Release Flow

1. Developer commits using conventional format:
   ```bash
   git commit -m "feat: add JSON output mode"
   ```

2. Push to main (or merge PR):
   ```bash
   git push origin main
   ```

3. GitHub Actions automatically:
   - ✅ Runs tests
   - ✅ Analyzes commits since last release
   - ✅ Determines new version (e.g., 1.0.0 → 1.1.0)
   - ✅ Builds binaries for all platforms
   - ✅ Updates CHANGELOG.md
   - ✅ Updates Cargo.toml version
   - ✅ Creates GitHub release
   - ✅ Uploads all binaries
   - ✅ Commits changes back to repo

### Commit Message Format

Follow [Conventional Commits](https://www.conventionalcommits.org/):

```
<type>(<scope>): <description>

[optional body]

[optional footer(s)]
```

**Types:**
- `feat:` - New feature (minor bump)
- `fix:` - Bug fix (patch bump)
- `perf:` - Performance improvement (patch bump)
- `docs:` - Documentation (no release)
- `refactor:` - Code refactoring (no release)
- `test:` - Tests (no release)
- `chore:` - Maintenance (no release)

**Breaking Changes:**
```bash
feat!: change config format

BREAKING CHANGE: Old TOML format no longer supported
```

## Release Artifacts

Each release automatically generates:

- **Git Tag**: `v1.0.0`
- **GitHub Release**: With release notes
- **Binary Tarballs**:
  - `uppies-linux-x86_64.tar.gz`
  - `uppies-linux-aarch64.tar.gz`
  - `uppies-macos-x86_64.tar.gz`
  - `uppies-macos-aarch64.tar.gz`
- **Updated Files**:
  - `CHANGELOG.md` (with new version section)
  - `Cargo.toml` (version field updated)

## First Release

To trigger the first release:

```bash
# Make sure main branch is clean
git status

# Commit all current work with conventional format
git add .
git commit -m "feat: initial release with core functionality"

# Push to main
git push origin main

# GitHub Actions will automatically:
# - Create v1.0.0 release
# - Build and upload binaries
# - Update changelog
```

## Testing Locally

Test the release process locally (without creating a release):

```bash
# Install dependencies
npm install

# Dry-run to see what would happen
npx semantic-release --dry-run

# Check version update script
node scripts/update-cargo-version.js 1.2.3
grep version Cargo.toml
```

## Dependencies

All semantic-release dependencies are installed:

```
@semantic-release/changelog@6.0.3
@semantic-release/exec@7.1.0
@semantic-release/git@10.0.1
conventional-changelog-conventionalcommits@9.1.0
semantic-release@25.0.2
```

## Notes

- All binaries use `cargo build --release`
- Linux binaries are statically linked (musl) using `cross`
- macOS binaries are dynamically linked
- Release notes are auto-generated from commits
- Versions follow [Semantic Versioning](https://semver.org/)

## Ready to Ship! 🚀

The release management system is fully configured and ready to use. Just push commits with conventional messages to `main` and watch the magic happen!
