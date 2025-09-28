#!/bin/bash

set -e

# Get the directory where this script is located
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PLUGIN_DIR="$(dirname "$SCRIPT_DIR")"

cd "$PLUGIN_DIR"

echo "Building tree-sitter parser for txtx..."

# Check if tree-sitter CLI is installed
if ! command -v tree-sitter &> /dev/null; then
    echo "tree-sitter CLI not found. Installing via npm (without node-gyp dependencies)..."
    npm install --ignore-scripts tree-sitter-cli
    npx tree-sitter --version
    TREE_SITTER="npx tree-sitter"
else
    TREE_SITTER="tree-sitter"
fi

# Generate the parser
echo "Generating parser from grammar.js..."
$TREE_SITTER generate

# Create parser directory if it doesn't exist
mkdir -p parser

# Detect OS and set extension
if [[ "$OSTYPE" == "darwin"* ]]; then
    EXT="dylib"
    CC_FLAGS="-dynamiclib"
else
    EXT="so"
    CC_FLAGS="-shared"
fi

# Compile the parser
echo "Compiling parser to parser/txtx.$EXT..."
cc $CC_FLAGS -o parser/txtx.$EXT \
   -I src \
   src/parser.c \
   -fPIC \
   -O2

echo "Build complete! Parser available at: parser/txtx.$EXT"
