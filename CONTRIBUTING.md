# Contributing to retcon

Thank you for your interest in contributing to retcon! This document provides guidelines and setup instructions for contributors.

## Development Setup

### Prerequisites

- Rust 1.70 or later
- Git
- Python 3.7+ (for pre-commit hooks)

### Initial Setup

1. Clone the repository:
   ```bash
   git clone https://github.com/yourusername/retcon.git
   cd retcon
   ```

2. Build the project:
   ```bash
   cargo build
   ```

3. Run tests:
   ```bash
   cargo test
   ```

## Pre-commit Hooks

This project uses [pre-commit](https://pre-commit.com/) to maintain code quality and consistency. Pre-commit hooks automatically check your code before each commit.

### Installing Pre-commit Hooks

1. Install pre-commit (one-time setup):
   ```bash
   pip install pre-commit
   ```

   Or using pipx (recommended):
   ```bash
   pipx install pre-commit
   ```

   Or using Homebrew (macOS):
   ```bash
   brew install pre-commit
   ```

2. Install the git hooks (run from project root):
   ```bash
   pre-commit install
   ```

3. Install the commit-msg hook for conventional commits:
   ```bash
   pre-commit install --hook-type commit-msg
   ```

### What the Hooks Check

The pre-commit hooks will automatically run on staged files before each commit:

- **Rust formatting**: Ensures code follows `rustfmt` style (`cargo fmt --check`)
- **Clippy linting**: Catches common mistakes and enforces best practices
- **Trailing whitespace**: Removes unnecessary whitespace
- **End-of-file fixer**: Ensures files end with a newline
- **YAML/TOML validation**: Validates configuration files
- **Conventional commits**: Enforces commit message format

### Running Hooks Manually

Run all hooks on all files (useful before pushing):
```bash
pre-commit run --all-files
```

Run specific hook:
```bash
pre-commit run cargo-fmt --all-files
pre-commit run cargo-clippy --all-files
```

Update hooks to latest versions:
```bash
pre-commit autoupdate
```

Skip hooks (not recommended, use sparingly):
```bash
git commit --no-verify -m "your message"
```

## Commit Message Format

This project follows [Conventional Commits](https://www.conventionalcommits.org/) specification.

### Format

```
type(scope): subject

[optional body]

[optional footer]
```

### Types

- `feat`: New feature
- `fix`: Bug fix
- `docs`: Documentation changes
- `style`: Code style changes (formatting, missing semi-colons, etc.)
- `refactor`: Code refactoring
- `perf`: Performance improvements
- `test`: Adding or updating tests
- `build`: Build system changes (Cargo.toml, dependencies)
- `ci`: CI/CD changes
- `chore`: Maintenance tasks
- `revert`: Revert a previous commit

### Rules

- Type must be lowercase
- Subject must be lowercase and start immediately after the colon
- Subject minimum length: 10 characters
- Subject maximum length: 72 characters
- Header (type + scope + subject) maximum: 100 characters
- Subject should not end with a period
- Use imperative mood: "add feature" not "adds feature" or "added feature"

### Examples

Good commit messages:
```
feat(ui): add sorting to commit table
fix(git): handle empty repositories correctly
docs: update installation instructions in README
refactor(state): simplify app state management
perf(render): optimize commit list rendering
test(git): add tests for rebase operations
build: update ratatui to version 0.29
ci: add rust formatting check to GitHub Actions
chore: update dependencies
```

Breaking changes:
```
feat(api)!: remove deprecated rebase function

BREAKING CHANGE: The old rebase API has been removed.
Use the new rewrite API instead.
```

### Scope (Optional but Recommended)

Common scopes for this project:
- `ui`: User interface components
- `git`: Git operations
- `state`: Application state management
- `cli`: Command-line interface
- `config`: Configuration handling
- `editor`: Text editor integration
- `theme`: Theming and styling

## Code Style

### Rust

- Follow the official [Rust Style Guide](https://doc.rust-lang.org/nightly/style-guide/)
- Use `cargo fmt` to format code automatically
- Address all `cargo clippy` warnings
- Write descriptive variable and function names
- Add documentation comments for public APIs

### General

- Keep lines under 100 characters when possible
- Use 4 spaces for indentation (enforced by rustfmt)
- Remove trailing whitespace
- End files with a newline

## Testing

Run all tests:
```bash
cargo test
```

Run with output:
```bash
cargo test -- --nocapture
```

Run specific test:
```bash
cargo test test_name
```

## Building

Debug build:
```bash
cargo build
```

Release build (optimized):
```bash
cargo build --release
```

## Running

Run from source:
```bash
cargo run
```

Run with arguments:
```bash
cargo run -- [args]
```

## Troubleshooting

### Pre-commit hooks fail

If hooks fail:
1. Read the error message carefully
2. Fix the issues reported
3. Stage the fixes: `git add .`
4. Try committing again

### Cargo clippy warnings

Fix all Clippy warnings before committing. If you believe a warning is a false positive, you can allow it with:
```rust
#[allow(clippy::lint_name)]
```

But document why the warning should be ignored.

### Formatting issues

Run `cargo fmt` to automatically format your code:
```bash
cargo fmt
```

## Questions?

If you have questions or need help, please open an issue on GitHub.
