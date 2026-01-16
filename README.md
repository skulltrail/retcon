# retcon

> Retroactive Continuity for Git History - A modern TUI for editing commit metadata

[![CI](https://github.com/skulltrail/retcon/actions/workflows/ci.yml/badge.svg)](https://github.com/skulltrail/retcon/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

---

## Important Notice

**retcon** provides a focused set of git history manipulation operations:

- **Metadata editing** - author, date, message (full support)
- **Commit deletion** - remove commits with automatic child reparenting
- **Commit reordering** - move commits up/down in history

It does **NOT** support squashing or splitting commits. For those operations, use `git rebase -i`.

---

## Features

- **TUI Interface** - Clean, intuitive terminal UI for browsing and editing commits
- **Edit Commit Messages** - Modify commit messages in your preferred `$EDITOR`
- **Edit Author Information** - Change author name and email for any commit
- **Edit Commit Dates** - Adjust both author and committer timestamps
- **Delete Commits** - Mark commits for deletion; child commits are automatically reparented
- **Reorder Commits** - Move commits up/down to restructure history order
- **Search & Filter** - Quickly find commits by hash, author, email, or message content
- **Batch Operations** - Edit multiple commits at once using checkboxes or visual selection
- **Visual Selection Mode** - Vim-like visual mode (line-wise `v` and block-wise `Ctrl+v`) for intuitive multi-commit editing
- **Undo/Redo Support** - Full undo/redo stack for all modifications
- **Inline Editing** - Edit fields directly in the table with rich keyboard navigation
- **Safe Operations** - Creates backup refs before rewriting history
- **Dirty Working Tree Handling** - Automatically stashes uncommitted changes during history rewrite
- **Author/Committer Sync** - Editing author fields updates committer fields by default (configurable)
- **Validation** - Email and date format validation before applying changes

---

## Installation

### From crates.io

```bash
cargo install retcon
```

### From source

```bash
git clone https://github.com/skulltrail/retcon
cd retcon
cargo install --path .
```

Both `retcon` and `ret` commands will be available after installation.

---

## Usage

### Basic Usage

Navigate to a git repository and run:

```bash
retcon
# or the shorter alias
ret
```

### Command Line Options

```bash
# Specify a repository path
retcon --path /path/to/repo

# Limit number of commits to load (default: 50)
retcon -n 100
retcon --limit 100

# Keep author and committer fields separate
# (By default, editing author fields also updates committer fields)
retcon --separate-author-committer
retcon -s
```

### Key Bindings

#### Navigation

- `j` / `↓` - Move cursor down
- `k` / `↑` - Move cursor up
- `h` / `←` - Move to previous column
- `l` / `→` - Move to next column
- `g` / `Home` - Jump to first commit
- `G` / `End` - Jump to last commit
- `Ctrl+d` / `Ctrl+u` - Page down/up

#### Editing

- `e` / `Enter` - Start editing current cell
- `Tab` / `Shift+Tab` - Navigate between columns while editing
- `Enter` - Confirm edit
- `Esc` - Cancel edit

#### Selection (for batch editing)

- `Space` - Toggle selection on current commit
- `Ctrl+a` - Select all commits
- `Ctrl+n` - Deselect all commits

#### Visual Mode (Vim-like)

- `v` - Enter line-wise visual mode
- `Ctrl+v` - Enter block-wise visual mode
- `j/k/h/l` - Extend selection
- `e` / `Enter` - Edit selected commits
- `Esc` - Exit visual mode

#### Search & Filter

- `/` - Open search bar
- `Enter` - Apply filter
- `Esc` - Clear filter

#### Undo/Redo

- `u` - Undo last change
- `Ctrl+r` - Redo

#### Delete Commits

- `d` / `x` - Mark/unmark commit for deletion
  - Works on selected commits if any are selected
  - Child commits are automatically reparented to deleted commit's parent

#### Reorder Commits

- `Shift+K` / `Ctrl+k` - Move commit up (earlier in history)
- `Shift+J` / `Ctrl+j` - Move commit down (later in history)
  - Merge commits cannot be reordered
  - Reordering is disabled while filtering

#### Actions

- `w` - Write changes (rewrites history)
- `r` - Reset/discard all pending changes
- `q` - Quit (prompts if there are unsaved changes)
- `?` - Show help screen (scrollable with j/k, Ctrl+d/u)

---

## Development

### Prerequisites

- Rust 1.70 or later
- Git
- Python 3.7+ (for pre-commit hooks)

### Quick Setup

Use the automated setup script:

```bash
git clone https://github.com/skulltrail/retcon
cd retcon
./setup-dev.sh
```

Or use Make:

```bash
make setup
```

This will:

- Install pre-commit hooks
- Set up commit message linting
- Install development tools
- Build the project

### Building from Source

```bash
# Debug build
cargo build

# Release build (optimized)
cargo build --release
make release
```

The binary will be available at `target/release/retcon`.

### Running Tests

```bash
cargo test
# or
make test
```

### Development Build

```bash
cargo run -- --path /path/to/test/repo
# or
make dev
```

### Code Quality

This project uses pre-commit hooks to maintain code quality:

```bash
# Format code
cargo fmt
# or
make fmt

# Run linter
cargo clippy
# or
make lint

# Run all checks before committing
make pre-commit
```

See [CONTRIBUTING.md](CONTRIBUTING.md) for detailed contribution guidelines.

---

## How It Works

1. **Load Commits** - retcon reads commits from your repository using libgit2
2. **Make Changes** - Edit metadata, delete, or reorder commits with full undo/redo support
3. **Apply Changes** - When you write changes (`w`), retcon:
   - Automatically stashes any uncommitted changes in your working tree
   - Creates a backup ref (`refs/original/refs/heads/<branch>`)
   - Rewrites the commit history with your changes
   - Updates your branch to point to the new history
   - Restores your stashed changes

**Note:** After rewriting history, you'll need to force-push if the branch was already pushed to a remote:

```bash
git push --force-with-lease
```

---

## License

MIT License - see [LICENSE](LICENSE) file for details.

---

## Contributing

Contributions welcome! Please feel free to submit a Pull Request.

---

## Safety & Best Practices

- Always review changes before applying (`w`)
- retcon creates backup refs, but you should still backup important work
- Coordinate with your team before rewriting shared history
- Use `--force-with-lease` when pushing rewritten history to avoid overwriting others' work

---

**Built with Rust + Ratatui** | Inspired by the need for quick metadata fixes without full interactive rebase
