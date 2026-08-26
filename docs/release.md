# Release Process

This document describes the release process and maintenance procedures for Stardrift. It's primarily a reference for how the automated release system works.

## Overview

Stardrift uses [cargo-release](https://github.com/crate-ci/cargo-release) for version management and GitHub Actions for automated builds and artifact publishing.

## Version Numbering

The project follows Rust ecosystem conventions for semantic versioning:

| Version | Stage | Description |
|---------|-------|-------------|
| `0.0.x` | Experimental | Early development, frequent breaking changes |
| `0.x.y` | Pre-1.0 | API may still evolve |
| `1.0.0` | Stable | First stable release with API stability commitment |

The project is currently in the `0.0.x` experimental phase.

## Creating a Release

### Prerequisites

- Clean working directory (`git status` shows no changes)
- On the `main` branch
- `cargo-release` installed: `cargo install cargo-release`

### Release Commands

```bash
# Review changes since last release
cargo release changes

# Dry run (shows what would happen without making changes)
cargo release patch

# Execute the release
cargo release patch --execute

# Push to GitHub (triggers automated builds)
git push origin main
git push origin --tags
```

### Release Types

| Command | Version Change | Use Case |
|---------|---------------|----------|
| `cargo release patch` | 0.0.66 → 0.0.67 | Bug fixes, minor changes |
| `cargo release minor` | 0.0.x → 0.1.0 | New features (once stable) |
| `cargo release major` | 0.x.y → 1.0.0 | Breaking changes (once stable) |

## What Happens During Release

When you run `cargo release patch --execute`, the following happens automatically:

1. **Version bump**: Updates version in `Cargo.toml`
2. **Changelog update**: Adds new version header with current date to `CHANGELOG.md`
3. **Lock file update**: Regenerates `Cargo.lock` with new version
4. **Git commit**: Creates signed commit with message `chore: release vX.Y.Z`
5. **Git tag**: Creates signed tag `vX.Y.Z`

### Release Configuration

Release behavior is configured in `Cargo.toml` under `[package.metadata.release]` (see the file for the authoritative settings). The important behaviors: commits and tags are signed, nothing is published to crates.io, releases are only allowed from `main`, and `push = false` — pushing the commit and tag to GitHub is a deliberate manual step. The changelog rewrite is driven by a `pre-release-replacements` rule on `CHANGELOG.md`.

## Automated Builds

When a version tag is pushed to GitHub, the release workflow automatically:

1. Runs the CI workflow (the release is gated on it passing)

2. Builds binaries for all platforms:
   - Linux (x86_64, ARM64)
   - Windows (x86_64, ARM64)
   - macOS (Apple Silicon)
   - WebAssembly

3. Creates release artifacts:
   - `.tar.gz` archives for Linux/macOS
   - `.zip` archives for Windows
   - `.dmg` disk images for macOS
   - WASM package (`.tar.gz`) for web deployment

4. Generates SHA256 checksums and build provenance attestations (see [Build Provenance](#build-provenance))

5. Publishes everything to the GitHub Releases page, with release notes extracted from the changelog

6. Deploys the WASM build to GitHub Pages

## Build Provenance

All release binaries include cryptographic attestations proving they were built by the official GitHub Actions workflow. This provides supply chain security.

### Verifying Binaries

```bash
# Install GitHub CLI: https://cli.github.com/

# Verify a downloaded binary
gh attestation verify stardrift-0.0.67-x86_64-unknown-linux-gnu.tar.gz \
  --repo emilyst/stardrift
```

Successful verification confirms:
- The binary was built from the official repository
- The build used the official GitHub Actions workflow
- The binary hasn't been modified since building

### Standards

Attestations use:
- [Sigstore](https://sigstore.dev/) for cryptographic signing
- [SLSA](https://slsa.dev/) (Supply-chain Levels for Software Artifacts) framework

## Changelog Management

The changelog follows [Keep a Changelog](https://keepachangelog.com/) conventions.

### During Development

Add entries under the `[Unreleased]` section:

```markdown
## [Unreleased]

### Added
- New feature description

### Changed
- Change description

### Fixed
- Bug fix description
```

### During Release

`cargo-release` automatically:
1. Renames `[Unreleased]` to `[X.Y.Z] - YYYY-MM-DD`
2. Creates a new empty `[Unreleased]` section

## Troubleshooting

### Release fails due to dirty working directory

```bash
# Check what's changed
git status

# Either commit or stash changes
git stash
cargo release patch --execute
git stash pop
```

### Tag already exists

If a tag already exists (from a failed release attempt):

```bash
# Delete local tag
git tag -d v0.0.67

# Delete remote tag (if pushed)
git push origin :refs/tags/v0.0.67

# Retry release
cargo release patch --execute
```

### Build fails in GitHub Actions

1. Check the Actions tab for error details
2. Fix the issue locally
3. Create a new patch release with the fix

## Pre-built Package Locations

After a release, binaries are available at:

```
https://github.com/emilyst/stardrift/releases/tag/vX.Y.Z
```

### Package Naming Convention

```
stardrift-{version}-{target}.{ext}
```

Examples:
- `stardrift-0.0.67-x86_64-unknown-linux-gnu.tar.gz`
- `stardrift-0.0.67-x86_64-pc-windows-msvc.zip`
- `stardrift-0.0.67-aarch64-apple-darwin.dmg`

## See Also

- [cargo-release documentation](https://github.com/crate-ci/cargo-release)
- [Keep a Changelog](https://keepachangelog.com/)
- [Semantic Versioning](https://semver.org/)
- [SLSA Framework](https://slsa.dev/)
