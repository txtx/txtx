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
    @echo ""
    @echo "Test:"
    @echo "  test-all        - Run all tests"
    @echo "  test-core       - Core library tests"
    @echo "  test-cli        - CLI tests"
    @echo "  test <name>     - Run specific test"
    @echo ""
    @echo "Coverage:"
    @echo "  coverage        - Generate HTML coverage report"
    @echo ""
    @echo "Documentation:"
    @echo "  doc             - Generate and open docs"
    @echo "  doc-all         - Generate docs for all packages"
    @echo ""
    @echo "Architecture:"
    @echo "  arch-c4         - Generate C4 diagrams from code"
    @echo "  arch-view       - View validation architecture diagrams"
    @echo ""
    @echo "Other:"
    @echo "  fmt             - Format code"
    @echo "  clean           - Clean build artifacts"

# Common flags
CLI_FLAGS := "--package txtx-cli --no-default-features --features cli"
CLI_BIN := CLI_FLAGS + " --bin txtx"

# ===== Build Recipes =====

# Build CLI
build:
    cargo build {{CLI_BIN}}

# Build CLI in release mode
build-release:
    cargo build {{CLI_BIN}} --release

# Check code without building
check:
    cargo check {{CLI_BIN}}

# ===== Test Recipes =====

# Run all tests
test-all:
    @echo "🧪 Running all tests..."
    @echo ""
    @echo "📦 Core library tests..."
    @just test-core
    @echo ""
    @echo "🔧 CLI tests..."
    @just test-cli
    @echo ""
    @echo "✅ All tests passed!"

# Core library tests
test-core:
    cargo test --package txtx-core --no-default-features

# CLI tests
test-cli:
    cargo test {{CLI_FLAGS}}

# Run specific test
test name:
    cargo test {{CLI_FLAGS}} {{name}}

# ===== Coverage =====

# Generate HTML coverage report
coverage:
    @echo "📊 Generating coverage report..."
    cargo llvm-cov test {{CLI_BIN}} --html
    @echo "✅ Coverage report: target/llvm-cov/html/index.html"

# ===== Documentation =====

# Generate and open documentation
doc:
    cargo doc {{CLI_FLAGS}} --no-deps --open

# Generate and open documentation for core packages (excludes addons and supervisor-ui)
doc-all:
    cargo doc --package txtx-core --package txtx-addon-kit --package c4-generator --no-default-features --no-deps
    cargo doc --package txtx-cli --no-default-features --features cli --no-deps --open

# ===== Architecture Diagrams =====

# Generate C4 diagrams from code annotations
arch-c4:
    @echo "📊 Generating C4 diagrams from code annotations..."
    @cargo build --package c4-generator --release --quiet
    @./target/release/c4-generator
    @echo ""
    @echo "   (Auto-generated from @c4-* annotations in code)"

# View validation architecture C4 diagrams with Structurizr Lite (generates first, then views)
arch-view:
    @echo "📊 Generating C4 from code annotations..."
    @cargo build --package c4-generator --release --quiet
    @./target/release/c4-generator
    @echo ""
    @if command -v podman >/dev/null 2>&1; then \
        echo "🚀 Starting Structurizr Lite with podman..."; \
        echo "   Viewing: Validation Architecture"; \
        echo "   Open http://localhost:8080 in your browser"; \
        echo ""; \
        podman run -it --rm -p 8080:8080 \
            -v $(pwd)/docs/architecture:/usr/local/structurizr:Z \
            docker.io/structurizr/lite; \
    elif command -v docker >/dev/null 2>&1; then \
        echo "🚀 Starting Structurizr Lite with docker..."; \
        echo "   Viewing: Validation Architecture"; \
        echo "   Open http://localhost:8080 in your browser"; \
        echo ""; \
        docker run -it --rm -p 8080:8080 \
            -v $(pwd)/docs/architecture:/usr/local/structurizr \
            structurizr/lite; \
    else \
        echo "❌ Neither docker nor podman found. Install one of them:"; \
        echo "   brew install podman  # or brew install docker"; \
        exit 1; \
    fi

# ===== Other =====

# Format code
fmt:
    cargo fmt --all

# Clean build artifacts
clean:
    cargo clean
