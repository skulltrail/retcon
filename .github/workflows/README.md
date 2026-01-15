# GitHub Actions Workflows

This directory contains GitHub Actions workflows for CI/CD automation.

## Workflows

### CI Workflow (`ci.yml`)

**Triggers:**
- Push to `main` branch
- Pull requests to `main` branch

**Jobs:**

1. **Test** - Runs on matrix of OS platforms (Ubuntu, macOS, Windows)
   - Checks code formatting with `cargo fmt`
   - Runs linter with `cargo clippy`
   - Builds release binary
   - Runs test suite

2. **Coverage** - Code coverage analysis (Linux only)
   - Generates coverage report using `cargo-llvm-cov`
   - Uploads coverage to Codecov
   - Requires `CODECOV_TOKEN` secret

3. **Lint** - Additional linting checks
   - Checks for outdated dependencies

**Caching:**
- Cargo registry and index
- Build artifacts (`target/` directory)

### Release Workflow (`release.yml`)

**Triggers:**
- Git tags matching `v*` (e.g., `v1.0.0`)

**Jobs:**

1. **Create Release**
   - Generates changelog from conventional commits using git-cliff
   - Creates GitHub release with changelog

2. **Build Release** - Multi-platform binary builds
   - Linux (x86_64-gnu, x86_64-musl)
   - macOS (x86_64, aarch64/M1)
   - Windows (x86_64)
   - Strips binaries for smaller size
   - Creates platform-specific archives (tar.gz for Unix, zip for Windows)
   - Uploads as release assets

3. **Publish Crate**
   - Publishes to crates.io
   - Requires `CARGO_REGISTRY_TOKEN` secret

**Creating a Release:**

```bash
# 1. Update version in Cargo.toml
# 2. Commit changes
git add Cargo.toml
git commit -m "chore(release): prepare for v1.0.0"

# 3. Create and push tag
git tag -a v1.0.0 -m "Release v1.0.0"
git push origin v1.0.0

# 4. GitHub Actions will automatically:
#    - Generate changelog
#    - Build binaries for all platforms
#    - Create GitHub release with artifacts
#    - Publish to crates.io
```

### Security Audit Workflow (`audit.yml`)

**Triggers:**
- Push to `main` (when Cargo.toml/lock changes)
- Pull requests (when Cargo.toml/lock changes)
- Weekly schedule (Monday at 00:00 UTC)
- Manual trigger via workflow_dispatch

**Jobs:**

1. **Cargo Audit** - Security vulnerability scanning
   - Scans dependencies for known vulnerabilities
   - Uses RustSec Advisory Database

2. **Cargo Deny** - Dependency policy enforcement
   - License compliance checking
   - Duplicate dependency detection
   - Source verification
   - Configuration in `deny.toml`

3. **Dependency Review** - PR dependency analysis
   - Runs only on pull requests
   - Reviews new dependencies for security issues
   - Fails on moderate+ severity issues

4. **Security Scan** - Trivy vulnerability scanner
   - Scans filesystem for vulnerabilities
   - Uploads results to GitHub Security tab (SARIF format)

## Dependabot Configuration

**File:** `.github/dependabot.yml`

Automatically creates pull requests for:
- Cargo dependency updates (weekly, Monday 09:00)
- GitHub Actions updates (weekly, Monday 09:00)

**Grouping:**
- Patch updates grouped together
- Minor updates grouped together
- Major updates separate

## Required Secrets

Configure these in GitHub repository settings (Settings → Secrets and variables → Actions):

| Secret | Description | Required For |
|--------|-------------|--------------|
| `CODECOV_TOKEN` | Codecov upload token | CI coverage upload |
| `CARGO_REGISTRY_TOKEN` | crates.io API token | Publishing releases |
| `GITHUB_TOKEN` | Auto-provided by GitHub | Release creation |

### Getting Tokens

**Codecov Token:**
1. Sign up at https://codecov.io
2. Add your GitHub repository
3. Copy the upload token
4. Add as `CODECOV_TOKEN` secret

**Cargo Registry Token:**
1. Login to crates.io
2. Go to Account Settings → API Tokens
3. Create new token
4. Add as `CARGO_REGISTRY_TOKEN` secret

## Action Versions

All actions are pinned to specific versions for security and reproducibility:

- `actions/checkout@v4`
- `actions/cache@v4`
- `dtolnay/rust-toolchain@v1`
- `taiki-e/install-action@v2`
- `codecov/codecov-action@v4`
- `actions/create-release@v1`
- `actions/upload-release-asset@v1`
- `actions/dependency-review-action@v4`
- `aquasecurity/trivy-action@0.28.0`
- `github/codeql-action/upload-sarif@v3`

## Local Testing

Before pushing, you can run these checks locally:

```bash
# Format check
cargo fmt --all -- --check

# Linting
cargo clippy --all-targets --all-features -- -D warnings

# Tests
cargo test --verbose

# Security audit
cargo install cargo-audit
cargo audit

# License/dependency check
cargo install cargo-deny
cargo deny check

# Coverage (requires llvm-tools-preview)
rustup component add llvm-tools-preview
cargo install cargo-llvm-cov
cargo llvm-cov --all-features --workspace --lcov --output-path lcov.info
```

## Conventional Commits

The release workflow expects conventional commit format:

- `feat:` - New features
- `fix:` - Bug fixes
- `docs:` - Documentation changes
- `style:` - Code style changes
- `refactor:` - Code refactoring
- `perf:` - Performance improvements
- `test:` - Test additions/changes
- `chore:` - Maintenance tasks
- `ci:` - CI/CD changes

Example:
```bash
git commit -m "feat: add interactive commit editing"
git commit -m "fix: resolve memory leak in git operations"
```

## Troubleshooting

**Coverage upload fails:**
- Ensure `CODECOV_TOKEN` is set
- Check Codecov service status

**Release fails:**
- Ensure tag matches `v*` pattern
- Check `CARGO_REGISTRY_TOKEN` is valid
- Verify version in Cargo.toml matches tag

**Security audit fails:**
- Review failing advisories
- Update dependencies or add exceptions in `deny.toml`
- Document why exceptions are needed

**Build fails on specific platform:**
- Check platform-specific dependencies
- Review error logs in Actions tab
- Test locally with target: `cargo build --release --target <target-triple>`
