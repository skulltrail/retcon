#!/usr/bin/env bash
# Development environment setup script for retcon

set -e  # Exit on error

echo "Setting up retcon development environment..."
echo ""

# Check for Rust
if ! command -v cargo &> /dev/null; then
    echo "Error: Rust is not installed."
    echo "Please install Rust from https://rustup.rs/"
    exit 1
fi

echo "✓ Rust is installed: $(rustc --version)"

# Check for Python
if ! command -v python3 &> /dev/null; then
    echo "Error: Python 3 is not installed."
    echo "Please install Python 3.7 or later."
    exit 1
fi

echo "✓ Python is installed: $(python3 --version)"

# Install pre-commit
echo ""
echo "Installing pre-commit..."

if command -v pipx &> /dev/null; then
    echo "Using pipx to install pre-commit..."
    pipx install pre-commit
elif command -v pip3 &> /dev/null; then
    echo "Using pip3 to install pre-commit..."
    pip3 install --user pre-commit
else
    echo "Error: Neither pipx nor pip3 found."
    echo "Please install pip3 first."
    exit 1
fi

# Verify pre-commit installation
if ! command -v pre-commit &> /dev/null; then
    echo "Error: pre-commit installation failed."
    echo "Please ensure ~/.local/bin is in your PATH."
    exit 1
fi

echo "✓ pre-commit is installed: $(pre-commit --version)"

# Install git hooks
echo ""
echo "Installing git hooks..."
pre-commit install
pre-commit install --hook-type commit-msg

echo "✓ Git hooks installed"

# Install commitlint (Node.js based - optional)
echo ""
if command -v npm &> /dev/null; then
    echo "Installing commitlint (optional, for better commit message validation)..."
    npm install --save-dev @commitlint/cli @commitlint/config-conventional
    echo "✓ commitlint installed"
else
    echo "ℹ npm not found. Skipping commitlint installation."
    echo "  (Using conventional-pre-commit instead, which is sufficient)"
fi

# Build the project
echo ""
echo "Building retcon..."
cargo build

echo ""
echo "✓ Development environment setup complete!"
echo ""
echo "Next steps:"
echo "  1. Run tests: cargo test"
echo "  2. Run the app: cargo run"
echo "  3. Read CONTRIBUTING.md for contribution guidelines"
echo ""
echo "Pre-commit hooks are now active and will run automatically on each commit."
echo "To run hooks manually: pre-commit run --all-files"
