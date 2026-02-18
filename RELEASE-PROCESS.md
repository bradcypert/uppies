# Release Process

This project uses [semantic-release](https://github.com/semantic-release/semantic-release) for automated versioning and releases.

## Commit Message Format

Follow the [Conventional Commits](https://www.conventionalcommits.org/) specification:

```
<type>(<scope>): <subject>

<body>

<footer>
```

### Types

- **feat**: A new feature (triggers minor version bump)
- **fix**: A bug fix (triggers patch version bump)
- **perf**: Performance improvement (triggers patch version bump)
- **docs**: Documentation changes (no release)
- **style**: Code style changes (no release)
- **refactor**: Code refactoring (no release)
- **test**: Adding or updating tests (no release)
- **chore**: Maintenance tasks (no release)
- **ci**: CI/CD changes (no release)

### Examples

```bash
# New feature (1.0.0 -> 1.1.0)
git commit -m "feat: add JSON output mode"

# Bug fix (1.1.0 -> 1.1.1)
git commit -m "fix: handle empty version strings correctly"

# Breaking change (1.1.1 -> 2.0.0)
git commit -m "feat!: change config format to YAML

BREAKING CHANGE: TOML configs are no longer supported"

# Documentation (no release)
git commit -m "docs: update installation instructions"
```

## Release Workflow

1. **Development**: Work on feature branches
2. **Pull Request**: Create PR to `main` branch
3. **CI Check**: Automated tests (cargo test, fmt, clippy) run on PR
4. **Merge**: PR is merged to `main`
5. **Automatic Release**: GitHub Action triggers:
   - Analyzes commits since last release
   - Determines new version number
   - Builds binaries for all platforms:
     - Linux x86_64 (musl)
     - Linux aarch64 (musl)
     - macOS x86_64
     - macOS aarch64 (Apple Silicon)
   - Updates CHANGELOG.md
   - Updates version in Cargo.toml
   - Creates GitHub release with:
     - Release notes
     - Binary attachments
     - Git tag

## Manual Release (if needed)

If you need to trigger a release manually:

```bash
# Install dependencies
npm install

# Set GitHub token
export GITHUB_TOKEN=your_token_here

# Run semantic-release
npx semantic-release
```

## Version Numbers

This project follows [Semantic Versioning](https://semver.org/):

- **MAJOR**: Breaking changes
- **MINOR**: New features (backward compatible)
- **PATCH**: Bug fixes (backward compatible)

Format: `MAJOR.MINOR.PATCH` (e.g., `1.2.3`)

## Build Artifacts

Each release includes pre-built binaries:

- `uppies-linux-x86_64.tar.gz`
- `uppies-linux-aarch64.tar.gz`
- `uppies-macos-x86_64.tar.gz`
- `uppies-macos-aarch64.tar.gz`

All binaries are statically linked where possible (musl on Linux) for maximum portability.
