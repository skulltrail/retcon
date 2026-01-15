# Makefile for retcon - Retroactive Continuity CLI
# Mirrors CI workflows for local development
# Compatible with macOS and Linux

# Project configuration
PROJECT_NAME := retcon
CARGO := cargo
CARGO_FLAGS :=
RUST_BACKTRACE := 1

# Colors for output
COLOR_RESET := \033[0m
COLOR_BOLD := \033[1m
COLOR_GREEN := \033[32m
COLOR_YELLOW := \033[33m
COLOR_BLUE := \033[34m

# Default target
.DEFAULT_GOAL := help

# Phony targets (targets that don't represent files)
.PHONY: help build release test test-unit test-integration coverage \
        fmt fmt-check lint lint-strict audit deny \
        check ci pre-commit pre-commit-run pre-commit-update \
        clean install setup \
        run dev watch

##@ Core Targets

build: ## Build debug binary
	@echo "$(COLOR_BLUE)Building debug binary...$(COLOR_RESET)"
	$(CARGO) build $(CARGO_FLAGS)

release: ## Build release binary with optimizations
	@echo "$(COLOR_BLUE)Building release binary...$(COLOR_RESET)"
	$(CARGO) build --release $(CARGO_FLAGS)

test: ## Run all tests
	@echo "$(COLOR_BLUE)Running all tests...$(COLOR_RESET)"
	RUST_BACKTRACE=$(RUST_BACKTRACE) $(CARGO) test --all-features

test-unit: ## Run unit tests only
	@echo "$(COLOR_BLUE)Running unit tests...$(COLOR_RESET)"
	RUST_BACKTRACE=$(RUST_BACKTRACE) $(CARGO) test --lib --all-features

test-integration: ## Run integration tests only
	@echo "$(COLOR_BLUE)Running integration tests...$(COLOR_RESET)"
	RUST_BACKTRACE=$(RUST_BACKTRACE) $(CARGO) test --test '*' --all-features

coverage: ## Generate code coverage report
	@echo "$(COLOR_BLUE)Generating coverage report...$(COLOR_RESET)"
	@command -v cargo-tarpaulin >/dev/null 2>&1 || { \
		echo "$(COLOR_YELLOW)cargo-tarpaulin not found. Run 'make setup' to install.$(COLOR_RESET)"; \
		exit 1; \
	}
	$(CARGO) tarpaulin --out Html --out Stdout --output-dir coverage --all-features --workspace --timeout 300
	@echo "$(COLOR_GREEN)Coverage report generated in coverage/$(COLOR_RESET)"

##@ Quality Targets

fmt: ## Format code using rustfmt
	@echo "$(COLOR_BLUE)Formatting code...$(COLOR_RESET)"
	$(CARGO) fmt --all

fmt-check: ## Check code formatting (CI mode)
	@echo "$(COLOR_BLUE)Checking code formatting...$(COLOR_RESET)"
	$(CARGO) fmt --all -- --check

lint: ## Run clippy lints
	@echo "$(COLOR_BLUE)Running clippy...$(COLOR_RESET)"
	$(CARGO) clippy --all-targets --all-features -- -W clippy::all

lint-strict: ## Run clippy with deny warnings (CI mode)
	@echo "$(COLOR_BLUE)Running clippy (strict mode)...$(COLOR_RESET)"
	$(CARGO) clippy --all-targets --all-features -- -D warnings -W clippy::all -W clippy::pedantic

audit: ## Run security audit
	@echo "$(COLOR_BLUE)Running security audit...$(COLOR_RESET)"
	@command -v cargo-audit >/dev/null 2>&1 || { \
		echo "$(COLOR_YELLOW)cargo-audit not found. Run 'make setup' to install.$(COLOR_RESET)"; \
		exit 1; \
	}
	$(CARGO) audit

deny: ## Run cargo-deny checks
	@echo "$(COLOR_BLUE)Running cargo-deny...$(COLOR_RESET)"
	@command -v cargo-deny >/dev/null 2>&1 || { \
		echo "$(COLOR_YELLOW)cargo-deny not found. Run 'make setup' to install.$(COLOR_RESET)"; \
		exit 1; \
	}
	$(CARGO) deny check

##@ Combined Targets

check: fmt-check lint-strict test ## Run all checks (fmt-check + lint-strict + test)
	@echo "$(COLOR_GREEN)$(COLOR_BOLD)All checks passed!$(COLOR_RESET)"

ci: check audit deny ## Full CI simulation (check + audit + deny)
	@echo "$(COLOR_GREEN)$(COLOR_BOLD)CI simulation completed successfully!$(COLOR_RESET)"

pre-commit: fmt lint test ## Run pre-commit checks (fmt + lint + test)
	@echo "$(COLOR_GREEN)$(COLOR_BOLD)Pre-commit checks passed!$(COLOR_RESET)"

##@ Utility Targets

clean: ## Clean build artifacts
	@echo "$(COLOR_BLUE)Cleaning build artifacts...$(COLOR_RESET)"
	$(CARGO) clean
	rm -rf coverage/
	@echo "$(COLOR_GREEN)Clean complete!$(COLOR_RESET)"

install: release ## Install binary locally
	@echo "$(COLOR_BLUE)Installing $(PROJECT_NAME)...$(COLOR_RESET)"
	$(CARGO) install --path .
	@echo "$(COLOR_GREEN)$(PROJECT_NAME) installed successfully!$(COLOR_RESET)"

setup: ## Install development dependencies
	@echo "$(COLOR_BLUE)Installing development dependencies...$(COLOR_RESET)"
	@echo "Installing cargo-tarpaulin (code coverage)..."
	@$(CARGO) install cargo-tarpaulin || echo "$(COLOR_YELLOW)cargo-tarpaulin installation failed$(COLOR_RESET)"
	@echo "Installing cargo-audit (security auditing)..."
	@$(CARGO) install cargo-audit || echo "$(COLOR_YELLOW)cargo-audit installation failed$(COLOR_RESET)"
	@echo "Installing cargo-deny (dependency verification)..."
	@$(CARGO) install cargo-deny || echo "$(COLOR_YELLOW)cargo-deny installation failed$(COLOR_RESET)"
	@echo "Installing cargo-watch (file watching)..."
	@$(CARGO) install cargo-watch || echo "$(COLOR_YELLOW)cargo-watch installation failed$(COLOR_RESET)"
	@echo ""
	@echo "$(COLOR_BLUE)Setting up pre-commit hooks...$(COLOR_RESET)"
	@bash setup-dev.sh || echo "$(COLOR_YELLOW)Pre-commit setup failed$(COLOR_RESET)"
	@echo "$(COLOR_GREEN)Setup complete! Development tools installed.$(COLOR_RESET)"

pre-commit-run: ## Run pre-commit hooks on all files
	@echo "$(COLOR_BLUE)Running pre-commit hooks...$(COLOR_RESET)"
	@command -v pre-commit >/dev/null 2>&1 || { \
		echo "$(COLOR_YELLOW)pre-commit not found. Run 'make setup' to install.$(COLOR_RESET)"; \
		exit 1; \
	}
	pre-commit run --all-files

pre-commit-update: ## Update pre-commit hooks to latest versions
	@echo "$(COLOR_BLUE)Updating pre-commit hooks...$(COLOR_RESET)"
	@command -v pre-commit >/dev/null 2>&1 || { \
		echo "$(COLOR_YELLOW)pre-commit not found. Run 'make setup' to install.$(COLOR_RESET)"; \
		exit 1; \
	}
	pre-commit autoupdate

##@ Development Targets

run: build ## Build and run the application
	@echo "$(COLOR_BLUE)Running $(PROJECT_NAME)...$(COLOR_RESET)"
	$(CARGO) run

dev: ## Run in development mode with debug output
	@echo "$(COLOR_BLUE)Running in development mode...$(COLOR_RESET)"
	RUST_BACKTRACE=1 RUST_LOG=debug $(CARGO) run

watch: ## Watch for changes and rebuild
	@echo "$(COLOR_BLUE)Watching for changes...$(COLOR_RESET)"
	@command -v cargo-watch >/dev/null 2>&1 || { \
		echo "$(COLOR_YELLOW)cargo-watch not found. Run 'make setup' to install.$(COLOR_RESET)"; \
		exit 1; \
	}
	$(CARGO) watch -x build

##@ Help

help: ## Display this help message
	@echo "$(COLOR_BOLD)retcon - Retroactive Continuity CLI$(COLOR_RESET)"
	@echo ""
	@echo "$(COLOR_BOLD)Usage:$(COLOR_RESET)"
	@echo "  make $(COLOR_BLUE)<target>$(COLOR_RESET)"
	@echo ""
	@awk 'BEGIN {FS = ":.*##"; printf ""} \
		/^[a-zA-Z_-]+:.*?##/ { printf "  $(COLOR_BLUE)%-18s$(COLOR_RESET) %s\n", $$1, $$2 } \
		/^##@/ { printf "\n$(COLOR_BOLD)%s$(COLOR_RESET)\n", substr($$0, 5) } ' $(MAKEFILE_LIST)
	@echo ""
	@echo "$(COLOR_BOLD)Examples:$(COLOR_RESET)"
	@echo "  make setup        # First time setup - install dev tools"
	@echo "  make pre-commit   # Before committing - format, lint, test"
	@echo "  make check        # Run CI checks locally"
	@echo "  make coverage     # Generate coverage report"
	@echo ""
	@echo "$(COLOR_BOLD)Quick Reference:$(COLOR_RESET)"
	@echo "  Development:  make dev"
	@echo "  Before commit: make pre-commit"
	@echo "  CI simulation: make ci"
	@echo "  Clean slate:   make clean"
	@echo ""
