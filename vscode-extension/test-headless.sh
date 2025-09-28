#!/bin/bash

# Script to run VSCode extension tests in headless mode

echo "Running VSCode extension tests in headless mode..."

# Add txtx binary to PATH if it exists
PROJECT_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
if [ -f "$PROJECT_ROOT/target/release/txtx" ]; then
    export PATH="$PROJECT_ROOT/target/release:$PATH"
    export TXTX_LSP_PATH="$PROJECT_ROOT/target/release/txtx"
    echo "Added txtx binary to PATH and TXTX_LSP_PATH: $PROJECT_ROOT/target/release/txtx"
elif [ -f "$PROJECT_ROOT/target/debug/txtx" ]; then
    export PATH="$PROJECT_ROOT/target/debug:$PATH"
    export TXTX_LSP_PATH="$PROJECT_ROOT/target/debug/txtx"
    echo "Added txtx binary to PATH and TXTX_LSP_PATH: $PROJECT_ROOT/target/debug/txtx"
else
    echo "Warning: txtx binary not found in target/release or target/debug"
    echo "Please run 'cargo build --bin txtx' first"
fi

# Also add test fixtures to PATH for tests that spawn txtx directly
VSCODE_EXT_DIR="$(dirname "$0")"
export PATH="$VSCODE_EXT_DIR/test/fixtures:$PATH"

# Detect OS
OS="$(uname -s)"

# Check if xvfb is needed (not on macOS)
if [ "$OS" = "Darwin" ]; then
    # macOS doesn't need xvfb
    echo "Running on macOS - no virtual display needed"
    
    # Compile the extension and tests
    echo "Compiling extension..."
    npm run compile

    echo "Compiling tests..."
    npm run compile-tests

    # Run tests directly
    echo "Starting tests..."
    npm test
else
    # Linux needs xvfb
    if ! command -v xvfb-run &> /dev/null; then
        echo "xvfb-run not found. Please install it manually:"
        echo "  Ubuntu/Debian: sudo apt-get install xvfb"
        echo "  Fedora/RHEL: sudo dnf install xorg-x11-server-Xvfb"
        echo "  Arch: sudo pacman -S xorg-server-xvfb"
        exit 1
    fi
    
    # Compile the extension and tests
    echo "Compiling extension..."
    npm run compile

    echo "Compiling tests..."
    npm run compile-tests

    # Run tests with virtual display
    echo "Starting tests with virtual display..."
    xvfb-run -a npm test
fi

# Capture exit code
EXIT_CODE=$?

if [ $EXIT_CODE -eq 0 ]; then
    echo "✅ Tests passed successfully!"
else
    echo "❌ Tests failed with exit code $EXIT_CODE"
fi

exit $EXIT_CODE