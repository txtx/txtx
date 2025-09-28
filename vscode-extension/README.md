# txtx Language Server Extension

VSCode extension providing language support for txtx runbook files (.tx).

## Features

- **Syntax Highlighting** for .tx files
- **Hover Documentation** for functions and actions
- **Go to Definition** for inputs and variables
- **Auto-completion** for txtx constructs
- **Diagnostics** and error reporting

## Setup for Development

### Building the LSP Server

First, build the txtx CLI with LSP support:

```bash
# From the project root
cargo build --package txtx-cli --release --no-default-features --features cli

# Verify the LSP command is available
./target/release/txtx lsp --help
```

### Configuring VSCode

The extension needs to know where to find the txtx executable. You have several options:

#### Option 1: Workspace Settings (Recommended for Development)

Add to your workspace's `.vscode/settings.json`:
```json
{
  "txtx.lspPath": "${workspaceFolder}/target/release/txtx"
}
```

This uses a workspace-relative path that works across different machines.

#### Option 2: Use Environment Variable

Set the `TXTX_LSP_PATH` environment variable before starting VSCode:

```bash
export TXTX_LSP_PATH=/path/to/txtx/target/release/txtx
code
```

#### Option 3: Add to PATH

Add the txtx binary to your system PATH:

```bash
# Add to your shell profile (.bashrc, .zshrc, etc.)
export PATH="/path/to/txtx/target/release:$PATH"
```

### Running the Extension

1. Open the extension folder in VSCode:
   ```bash
   cd vscode-extension
   code .
   ```

2. Press `F5` to launch a new VSCode window with the extension loaded

3. Open a `.tx` file to activate the extension

4. Check the Output panel (View → Output → "txtx Language Server") for logs

## Testing Hover Functionality

Create a test file `test.tx`:

```hcl
addon "evm" "latest" {
    chain_id = 11155111
}

variable "contract" {
    // Hover over evm::get_contract_from_foundry_project to see docs
    value = evm::get_contract_from_foundry_project("SimpleStorage")
}

action "deploy" "evm::deploy_contract" {
    // Hover over evm::deploy_contract to see action documentation
    contract = variable.contract
}

action "call" "evm::call_contract" {
    // Hover over evm::call_contract for detailed parameter info
    contract_address = action.deploy.contract_address
    function_name = "set"
    function_args = [42]
}
```

## Packaging for Distribution

```bash
# Install vsce if not already installed
npm install -g @vscode/vsce

# Package the extension
vsce package

# This creates a .vsix file that can be installed
```

## Installing the VSIX

```bash
# Install via command line
code --install-extension txtx-lsp-extension-*.vsix

# Or install through VSCode UI:
# 1. Open Extensions view (Ctrl+Shift+X)
# 2. Click "..." menu → "Install from VSIX..."
# 3. Select the .vsix file
```

## Troubleshooting

### LSP Server Not Starting

1. Check the Output panel for error messages
2. Verify the txtx binary path is correct:
   ```bash
   # Test the binary directly
   /path/to/txtx lsp --help
   ```
3. Ensure the binary has LSP support (built with `--features cli`)

### Hover Not Working

1. Ensure the file has a `.tx` extension
2. Check that the LSP server is running (Output panel)
3. Try reloading the VSCode window (Ctrl+Shift+P → "Developer: Reload Window")

### Performance Issues

- The first hover request may be slow as the server initializes
- Large files may take longer to process
- Check the Output panel for any error messages

## Development

### Running Tests

```bash
# Run all tests
npm test

# Run tests in headless mode (for CI)
xvfb-run -a npm test

# Run specific test suite
npm test -- --grep "Hover"
```

### Debugging

1. Set breakpoints in the TypeScript code
2. Press F5 to launch the extension
3. Use the Debug Console to inspect variables

## Configuration Options

| Setting | Description | Default |
|---------|-------------|---------|
| `txtx.lspPath` | Path to the txtx executable | System PATH |
| `txtx.trace.server` | LSP communication tracing | "off" |

## Known Issues

- Hover documentation requires the txtx binary to be built with the latest changes
- Some complex nested function calls may not show hover correctly

## Contributing

See the main project README for contribution guidelines.