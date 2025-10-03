# Justfile for txtx project
# Run with: just <recipe-name>

# Default recipe - show available commands grouped by category
default:
    @echo "txtx Build Recipes"
    @echo ""
    @echo "Build:"
    @echo "  build           - Build CLI"
    @echo "  build-release   - Build CLI (release mode)"
    @echo "  check           - Check code without building"
    @echo "  lint-doc        - Show validation errors with doc format"
    @echo ""
    @echo "Test:"
    @echo "  cli-unit        - CLI unit tests"
    @echo "  cli-int         - CLI integration tests"
    @echo "  lint-unit       - Linter unit tests"
    @echo "  lint-int        - Linter integration tests"
    @echo "  lsp-unit        - LSP unit tests"
    @echo "  lsp-int         - LSP integration tests"
    @echo "  test <name>     - Run specific test"
    @echo "  test-verbose    - Run tests with output"
    @echo "  watch           - Watch and run tests"
    @echo ""
    @echo "Coverage:"
    @echo "  coverage        - Generate HTML coverage report"
    @echo "  coverage-ci     - Generate JSON coverage for CI"
    @echo "  coverage-test   - Coverage for specific test"
    @echo ""
    @echo "Analysis:"
    @echo "  complexity-high - Find high complexity functions"
    @echo "  complexity-file - Analyze specific file"
    @echo ""
    @echo "Documentation:"
    @echo "  doc             - Generate and open docs"
    @echo "  doc-all         - Generate docs for all packages"
    @echo ""
    @echo "Architecture:"
    @echo "  arch-c4         - Generate C4 diagrams from code"
    @echo "  arch-view       - View linter C4 diagrams (default)"
    @echo "  arch-view-linter - View linter architecture"
    @echo "  arch-view-lsp   - View LSP architecture"
    @echo "  arch-modules    - Generate module dependency graph"
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

# ===== Documentation =====
# Generate and open documentation
doc:
    cargo doc {{CLI_FLAGS}} --no-deps --open

# Generate documentation for all packages
doc-all:
    cargo doc --workspace --no-deps

# ===== Architecture Diagrams =====
# Generate C4 diagrams from code annotations
arch-c4:
    @echo "📊 Generating C4 diagrams from code annotations..."
    @cargo build --package c4-generator --release --quiet
    @./target/release/c4-generator
    @echo ""
    @echo "   (Auto-generated from @c4-* annotations in code)"

# View linter C4 diagrams with Structurizr Lite (generates first, then views)
arch-view-linter:
    @echo "📊 Generating C4 from code annotations..."
    @cargo build --package c4-generator --release --quiet
    @./target/release/c4-generator
    @echo ""
    @if command -v podman >/dev/null 2>&1; then \
        echo "🚀 Starting Structurizr Lite with podman..."; \
        echo "   Viewing: Linter Architecture"; \
        echo "   Open http://localhost:8080 in your browser"; \
        echo ""; \
        podman run -it --rm -p 8080:8080 \
            -v $(pwd)/docs/architecture/linter:/usr/local/structurizr:Z \
            docker.io/structurizr/lite; \
    elif command -v docker >/dev/null 2>&1; then \
        echo "🚀 Starting Structurizr Lite with docker..."; \
        echo "   Viewing: Linter Architecture"; \
        echo "   Open http://localhost:8080 in your browser"; \
        echo ""; \
        docker run -it --rm -p 8080:8080 \
            -v $(pwd)/docs/architecture/linter:/usr/local/structurizr \
            structurizr/lite; \
    else \
        echo "❌ Neither docker nor podman found. Install one of them:"; \
        echo "   brew install podman  # or brew install docker"; \
        exit 1; \
    fi

# View LSP C4 diagrams with Structurizr Lite
arch-view-lsp:
    @echo "📊 Viewing LSP Architecture..."
    @if command -v podman >/dev/null 2>&1; then \
        echo "🚀 Starting Structurizr Lite with podman..."; \
        echo "   Viewing: LSP Architecture"; \
        echo "   Open http://localhost:8080 in your browser"; \
        echo ""; \
        podman run -it --rm -p 8080:8080 \
            -v $(pwd)/docs/architecture/lsp:/usr/local/structurizr:Z \
            docker.io/structurizr/lite; \
    elif command -v docker >/dev/null 2>&1; then \
        echo "🚀 Starting Structurizr Lite with docker..."; \
        echo "   Viewing: LSP Architecture"; \
        echo "   Open http://localhost:8080 in your browser"; \
        echo ""; \
        docker run -it --rm -p 8080:8080 \
            -v $(pwd)/docs/architecture/lsp:/usr/local/structurizr \
            structurizr/lite; \
    else \
        echo "❌ Neither docker nor podman found. Install one of them:"; \
        echo "   brew install podman  # or brew install docker"; \
        exit 1; \
    fi

# View all C4 diagrams (alias for linter, use arch-view-lsp for LSP)
arch-view: arch-view-linter

# Generate module dependency graph (requires cargo-modules and graphviz)
arch-modules:
    @echo "📊 Generating module dependency graph..."
    @cargo modules generate graph --with-types --package txtx-cli | dot -Tpng > docs/architecture/modules.png 2>/dev/null || \
        (echo "❌ Error: Install cargo-modules and graphviz:" && \
         echo "   cargo install cargo-modules" && \
         echo "   brew install graphviz  # or apt-get install graphviz" && \
         exit 1)
    @echo "✅ Generated: docs/architecture/modules.png"
