# Justfile for txtx project
# Run with: just <recipe-name>

# Default recipe - show available commands grouped by category
default:
    @echo "txtx Build Recipes"
    @echo ""
    @echo "Build:"
    @echo "  build           - Build CLI"
    @echo "  build-release   - Build CLI (release mode)"
    @echo "  install         - Install txtx CLI with all features"
    @echo "  check           - Check code without building"
    @echo "  lint-doc        - Show validation errors with doc format"
    @echo ""
    @echo "Test - CLI:"
    @echo "  cli-unit        - CLI unit tests"
    @echo "  cli-int         - CLI integration tests"
    @echo "  cli-all         - All CLI tests (unit + integration)"
    @echo ""
    @echo "Test - Linter:"
    @echo "  lint-unit       - Linter unit tests"
    @echo "  lint-int        - Linter integration tests"
    @echo ""
    @echo "Test - LSP:"
    @echo "  lsp-unit        - LSP unit tests"
    @echo "  lsp-int         - LSP integration tests"
    @echo "  lsp-hcl-diag    - HCL diagnostic extraction tests"
    @echo "  lsp-validation  - LSP validation pipeline tests"
    @echo ""
    @echo "Test - Validation:"
    @echo "  val-core        - Validation core tests"
    @echo "  val-utils       - Validation utils tests"
    @echo "  val-all         - All validation tests"
    @echo ""
    @echo "Test - Other Packages:"
    @echo "  test-core       - txtx-core unit tests"
    @echo "  test-addon-kit  - txtx-addon-kit unit tests"
    @echo ""
    @echo "Test - General:"
    @echo "  test <name>     - Run specific test"
    @echo "  test-verbose    - Run tests with output"
    @echo "  watch           - Watch and run tests"
    @echo ""
    @echo "Coverage:"
    @echo "  coverage        - Generate HTML coverage report"
    @echo "  coverage-ci     - Generate JSON coverage for CI"
    @echo "  coverage-test   - Coverage for specific test"
    @echo ""
    @echo "Examples:"
    @echo "  builder-example - Run enhanced builder example"
    @echo ""
    @echo "Analysis:"
    @echo "  complexity-high - Find high complexity functions"
    @echo "  complexity-file - Analyze specific file"
    @echo ""
    @echo "Documentation:"
    @echo "  doc             - Generate and open docs"
    @echo "  doc-all         - Generate docs for all packages"
    @echo ""
    @echo "Other:"
    @echo "  fmt             - Format code"
    @echo "  clean           - Clean build artifacts"

# Common flags
CLI_FLAGS := "--package txtx-cli --no-default-features --features cli"
CLI_BIN := CLI_FLAGS + " --bin txtx"
CLI_TESTS := CLI_FLAGS + " --tests"

# Set common RUSTFLAGS for suppressing warnings during development
export RUST_DEV_FLAGS := "-A unused_assignments -A unused_variables -A dead_code -A unused_imports"

# ===== CLI Tests =====
# CLI unit tests only
cli-unit:
    RUSTFLAGS="{{RUST_DEV_FLAGS}}" cargo test {{CLI_BIN}}

# CLI integration tests only
cli-int:
    RUSTFLAGS="{{RUST_DEV_FLAGS}}" cargo test {{CLI_TESTS}}

# All CLI tests (unit + integration)
cli-all:
    RUSTFLAGS="{{RUST_DEV_FLAGS}}" cargo test {{CLI_FLAGS}}

# ===== Linter Tests =====

# Linter unit tests only
lint-unit:
    RUSTFLAGS="{{RUST_DEV_FLAGS}}" cargo test {{CLI_BIN}} cli::linter_impl::

# Linter integration tests only
lint-int:
    RUSTFLAGS="{{RUST_DEV_FLAGS}}" cargo test --package txtx-cli --test linter_tests_builder --no-default-features --features cli

# ===== LSP Tests =====

# LSP unit tests only
lsp-unit:
    RUSTFLAGS="{{RUST_DEV_FLAGS}}" cargo test {{CLI_BIN}} cli::lsp::

# LSP integration tests only
lsp-int:
    RUSTFLAGS="{{RUST_DEV_FLAGS}}" cargo test --package txtx-cli --test lsp_tests_builder --no-default-features --features cli

# HCL diagnostic extraction tests
lsp-hcl-diag:
    RUSTFLAGS="{{RUST_DEV_FLAGS}}" cargo test {{CLI_BIN}} cli::lsp::tests::hcl_diagnostics_test

# LSP validation pipeline tests
lsp-validation:
    RUSTFLAGS="{{RUST_DEV_FLAGS}}" cargo test {{CLI_BIN}} cli::lsp::tests::validation_integration_test

# ===== Validation Tests =====

# Validation core tests
val-core:
    cargo test --no-default-features --package txtx-core --lib validation::hcl_validator::tests

# Validation utils tests
val-utils:
    cargo test --no-default-features --package txtx-test-utils

# All validation tests
val-all:
    cargo test --no-default-features --lib --tests --package txtx-core --package txtx-test-utils

# ===== Other Package Tests =====

# txtx-core unit tests
test-core:
    cargo test --package txtx-core --lib

# txtx-addon-kit unit tests
test-addon-kit:
    cargo test --package txtx-addon-kit --lib

# ===== Code Coverage =====
# Generate HTML coverage report
coverage:
    @cargo llvm-cov --html {{CLI_FLAGS}}
    @echo "Coverage report: target/llvm-cov/html/index.html"

# Generate coverage for CI (JSON format)
coverage-ci:
    @cargo llvm-cov --json --summary-only {{CLI_FLAGS}}

# Generate coverage for specific test
coverage-test TEST:
    @cargo llvm-cov --html {{CLI_FLAGS}} -- {{TEST}}

# ===== Code Complexity =====
# Find high complexity functions (cyclomatic > 10 or cognitive > 20)
complexity-high:
    @echo "Finding high complexity functions..."
    @rust-code-analysis-cli -m -O json \
        -p crates/txtx-cli/src \
        -p crates/txtx-core/src | \
        jq -s -r '.[] | . as $file | .spaces[]? | select(.metrics.cyclomatic.sum > 10 or .metrics.cognitive.sum > 20) | "\($file.name):\(.name)\n  Cyclomatic: \(.metrics.cyclomatic.sum // 0)\n  Cognitive: \(.metrics.cognitive.sum // 0)\n  Lines: \(.start_line // 0)-\(.end_line // 0)\n"' 2>/dev/null || echo "No high complexity functions found"

# Analyze complexity of a specific file
complexity-file FILE:
    @echo "Analyzing complexity of {{FILE}}..."
    @rust-code-analysis-cli -m -O json -p {{FILE}} | \
        jq -r '"File: \(.name)\n  Cyclomatic: \(.metrics.cyclomatic.sum // 0)\n  Cognitive: \(.metrics.cognitive.sum // 0)\n  SLOC: \(.metrics.loc.sloc // 0)\n\nFunctions with complexity > 5:\n" + ([ .spaces[]? | select(.metrics.cyclomatic.sum > 5 or .metrics.cognitive.sum > 10) | "  \(.name) (lines \(.start_line)-\(.end_line))\n    Cyclomatic: \(.metrics.cyclomatic.sum // 0), Cognitive: \(.metrics.cognitive.sum // 0)" ] | join("\n"))' || echo "Error analyzing file"

# ===== Build Commands =====
build:
    cargo build {{CLI_FLAGS}}

build-release:
    cargo build {{CLI_FLAGS}} --release

# Install txtx CLI with all features (supervisor UI + OVM)
install:
    cargo install --path crates/txtx-cli --features supervisor_ui --features ovm --locked --force

# ===== Development Commands =====
# Check code without building
check:
    cargo check {{CLI_FLAGS}}

# Format code
fmt:
    cargo fmt --all

# Run specific test by name
test TEST_NAME:
    RUSTFLAGS="{{RUST_DEV_FLAGS}}" cargo test {{CLI_FLAGS}} {{TEST_NAME}}

# Run tests with output visible
test-verbose TEST_NAME="":
    RUSTFLAGS="{{RUST_DEV_FLAGS}}" cargo test {{CLI_FLAGS}} {{TEST_NAME}} -- --nocapture

# Watch for changes and run tests (requires cargo-watch)
watch:
    RUSTFLAGS="{{RUST_DEV_FLAGS}}" cargo watch -x "test {{CLI_FLAGS}}"

# Clean build artifacts
clean:
    cargo clean

# Lint file with documentation format (shareable examples)
lint-doc FILE:
    cargo run --package txtx-cli --no-default-features --features cli --bin txtx -- lint {{FILE}} --format doc

# ===== Examples =====
# Run enhanced builder example from txtx-test-utils
builder-example:
    cargo run --example enhanced_builder_example --package txtx-test-utils

# ===== Documentation =====
# Generate and open documentation
doc:
    cargo doc {{CLI_FLAGS}} --no-deps --open

# Generate documentation for all packages
doc-all:
    cargo doc --workspace --no-deps

