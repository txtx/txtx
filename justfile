# Justfile for txtx project
# Run with: just <recipe-name>

# Default recipe to list available commands
default:
    @just --list

# Set common RUSTFLAGS for suppressing warnings during development
export RUST_DEV_FLAGS := "-A unused_assignments -A unused_variables -A dead_code -A unused_imports"

# ===== CLI Tests =====
# All CLI tests (unit + integration)
cli:
    RUSTFLAGS="{{RUST_DEV_FLAGS}}" cargo test --package txtx-cli --no-default-features --features cli

# CLI unit tests only
cli-unit:
    RUSTFLAGS="{{RUST_DEV_FLAGS}}" cargo test --package txtx-cli --bin txtx --no-default-features --features cli

# CLI integration tests only
cli-int:
    RUSTFLAGS="{{RUST_DEV_FLAGS}}" cargo test --package txtx-cli --tests --no-default-features --features cli

# ===== Doctor Tests =====
# All doctor tests (unit + integration)
dr:
    @echo "Running all Doctor tests (unit + integration)..."
    @just dr-unit
    @just dr-int

# Doctor unit tests only
dr-unit:
    RUSTFLAGS="{{RUST_DEV_FLAGS}}" cargo test --package txtx-cli --bin txtx --no-default-features --features cli cli::doctor::

# Doctor integration tests only
dr-int:
    RUSTFLAGS="{{RUST_DEV_FLAGS}}" cargo test --package txtx-cli --test doctor_tests_builder --no-default-features --features cli

# ===== LSP Tests =====
# All LSP tests (unit + integration)
lsp:
    @echo "Running all LSP tests (unit + integration)..."
    @just lsp-unit
    @just lsp-int

# LSP unit tests only
lsp-unit:
    RUSTFLAGS="{{RUST_DEV_FLAGS}}" cargo test --package txtx-cli --bin txtx --no-default-features --features cli cli::lsp::

# LSP integration tests only
lsp-int:
    RUSTFLAGS="{{RUST_DEV_FLAGS}}" cargo test --package txtx-cli --test lsp_tests_builder --no-default-features --features cli

# ===== Build Commands =====
build-cli:
    cargo build --package txtx-cli --no-default-features --features cli

build-cli-release:
    cargo build --package txtx-cli --no-default-features --features cli --release

# Test with specific verbosity
test-dev-verbose:
    RUSTFLAGS="-A unused_assignments -A unused_variables -A dead_code -A unused_imports" \
    cargo test --package txtx-cli --no-default-features --features cli -- --nocapture

# Run all CLI tests in dev mode (no warnings)
test-dev-all:
    @echo "Running all CLI tests with warnings suppressed..."
    @just test-dev-cli

# Quick test for iterative development
test-quick:
    @just test-dev-cli-unit

# Clean build artifacts
clean:
    cargo clean

# Check code without building
check:
    cargo check --package txtx-cli --no-default-features --features cli

# Format code
fmt:
    cargo fmt --all

# Run clippy linter
clippy:
    cargo clippy --package txtx-cli --no-default-features --features cli

# Run clippy with all warnings allowed (for development)
clippy-dev:
    RUSTFLAGS="-A unused_assignments -A unused_variables -A dead_code -A unused_imports" \
    cargo clippy --package txtx-cli --no-default-features --features cli

# Run specific test by name (usage: just test-by-name <test_name>)
test-by-name TEST_NAME:
    RUSTFLAGS="-A unused_assignments -A unused_variables -A dead_code -A unused_imports" \
    cargo test --package txtx-cli --no-default-features --features cli {{TEST_NAME}}

# Run tests matching a pattern (usage: just test-match <pattern>)
test-match PATTERN:
    RUSTFLAGS="-A unused_assignments -A unused_variables -A dead_code -A unused_imports" \
    cargo test --package txtx-cli --no-default-features --features cli -- {{PATTERN}}

# Watch for changes and run tests (requires cargo-watch)
watch:
    cargo watch -x "test --package txtx-cli --no-default-features --features cli"

# Watch for changes and run tests with warnings suppressed
watch-dev:
    RUSTFLAGS="-A unused_assignments -A unused_variables -A dead_code -A unused_imports" \
    cargo watch -x "test --package txtx-cli --no-default-features --features cli"

# ===== Documentation Commands =====
# Generate documentation for txtx-cli
doc:
    cargo doc --package txtx-cli --no-default-features --features cli --no-deps

# Generate documentation and open in browser
doc-open:
    cargo doc --package txtx-cli --no-default-features --features cli --no-deps --open

# Generate documentation for all workspace packages
doc-all:
    cargo doc --workspace --no-deps

# Generate documentation with private items included
doc-private:
    cargo doc --package txtx-cli --no-default-features --features cli --no-deps --document-private-items

# Generate documentation for txtx-core
doc-core:
    cargo doc --package txtx-core --no-deps

# Generate documentation for txtx-test-utils
doc-test-utils:
    cargo doc --package txtx-test-utils --no-deps

# Clean and regenerate all documentation
doc-clean:
    rm -rf target/doc && cargo doc --workspace --no-deps
