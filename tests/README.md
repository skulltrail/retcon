# Test Suite Documentation

This directory contains the integration tests for retcon.

## Test Structure

- **Unit Tests**: Located in each module file using `#[cfg(test)]` modules
  - `src/git/commit.rs` - Tests for commit data structures and serialization
  - `src/git/validation.rs` - Tests for email and date validation
  - `src/git/repository.rs` - Tests for repository operations
  - `src/git/rewrite.rs` - Tests for history rewriting logic
  - `src/state/app_state.rs` - Tests for application state management

- **Integration Tests**: Located in `tests/` directory
  - `tests/integration_test.rs` - End-to-end workflow tests

## Running Tests

### Run All Tests
```bash
cargo test
```

### Run Only Unit Tests
```bash
cargo test --lib
```

### Run Only Integration Tests
```bash
cargo test --test '*'
```

### Run a Specific Test
```bash
cargo test test_name
```

### Run Tests with Output
```bash
cargo test -- --nocapture
```

### Run Tests in Serial (for git repo tests)
Tests that create git repositories use the `serial_test` crate to prevent
interference between tests:

```bash
cargo test -- --test-threads=1
```

## Code Coverage

### Using cargo-llvm-cov (Recommended)

Install:
```bash
cargo install cargo-llvm-cov
```

Generate HTML coverage report:
```bash
cargo coverage
# or
cargo llvm-cov --html --open
```

Generate LCOV report:
```bash
cargo coverage-report
# or
cargo llvm-cov --lcov --output-path target/coverage/lcov.info
```

For CI (excludes UI and main):
```bash
cargo coverage-ci
```

### Using cargo-tarpaulin (Alternative)

Install:
```bash
cargo install cargo-tarpaulin
```

Run with configuration:
```bash
cargo tarpaulin --config .config/tarpaulin.toml
```

Or run with default settings:
```bash
cargo tarpaulin --out Html --output-dir target/coverage
```

## Test Coverage Goals

- **git module**: >80% coverage
- **state module**: >80% coverage
- **validation**: >90% coverage
- **Overall**: >70% coverage

## Writing New Tests

### Unit Test Template

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_something() {
        // Arrange
        let input = create_test_data();

        // Act
        let result = function_under_test(input);

        // Assert
        assert_eq!(result, expected);
    }
}
```

### Integration Test Template

```rust
use retcon::{git::Repository, Result};
use serial_test::serial;

#[test]
#[serial]
fn test_workflow() -> Result<()> {
    // Setup test repository
    let (_temp_dir, repo_path) = create_test_repo();

    // Test your workflow
    let repo = Repository::open(&repo_path)?;

    // Assertions
    assert!(repo.load_commits(10)?.len() > 0);

    Ok(())
}
```

## Test Fixtures

Helper functions for creating test data:

- `create_test_repo()` - Creates a temporary git repository
- `create_test_commit()` - Creates test commit data
- `create_test_state()` - Creates test application state

## Continuous Integration

Tests are automatically run in CI on:
- Every push to main branch
- Every pull request
- Scheduled nightly builds

Coverage reports are uploaded to Codecov (if configured).

## Debugging Tests

To see detailed test output:
```bash
RUST_LOG=debug cargo test -- --nocapture
```

To run a specific test with backtrace:
```bash
RUST_BACKTRACE=1 cargo test test_name
```

## Common Issues

### Serial Test Failures
If tests that modify git repositories fail intermittently, ensure they're marked with `#[serial]` from the `serial_test` crate.

### Cleanup Failures
Temporary directories are automatically cleaned up when the `TempDir` is dropped. If you see "directory not empty" errors, ensure the repository is not holding file handles.

### Git Configuration
Test repositories are created with minimal configuration. If you need specific git config, add it in the test setup.
