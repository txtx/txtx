# txtx Language Server Protocol (LSP) User Guide

## Overview

The txtx LSP provides intelligent IDE support for txtx runbook files (`.tx`) and manifest files (`txtx.yml`). It offers features like auto-completion, go-to-definition, hover information, and real-time validation.

## Features

### Core Features
- ✅ **Syntax Highlighting** - Via TextMate grammar
- ✅ **Document Synchronization** - Real-time tracking of file changes
- ✅ **Go-to-Definition** - Jump from `input.variable_name` to its definition in `txtx.yml`
- ✅ **Hover Information** - See variable values, function documentation, and action details
- ✅ **Auto-completion** - Complete input variables, functions, and action types
- ✅ **Real-time Diagnostics** - Validation errors as you type
- ✅ **Multi-file Support** - Works with multi-file runbooks
- ✅ **Workspace Support** - Automatically discovers txtx.yml in parent directories

### Hover Documentation
Hover over any:
- **Variables** - Shows current value and environment
- **Functions** - Shows parameters, return type, and examples (e.g., `evm::encode_calldata`)
- **Action Types** - Shows required inputs and available outputs (e.g., `evm::deploy_contract`)
- **Input References** - Shows the resolved value from txtx.yml

## Installation

### Prerequisites
- Visual Studio Code (recommended) or any LSP-compatible editor
- txtx CLI installed and in PATH

### VSCode Extension Installation

#### Option 1: From VSIX Package
```bash
# Build the extension
cd vscode-extension
npm install
npm run package  # Creates txtx-lsp-0.0.1.vsix

# Install
code --install-extension txtx-lsp-0.0.1.vsix
```

#### Option 2: Development Mode
```bash
# Open extension in VSCode
cd vscode-extension
code .

# Press F5 to launch a new VSCode window with the extension loaded
```

### Building the LSP

The LSP is built into the txtx CLI:

```bash
# Build txtx with LSP support
cargo build --package txtx-cli --release

# The LSP is available via
./target/release/txtx lsp
```

## Configuration

### VSCode Settings

```json
{
  // Custom path to txtx binary (optional)
  "txtx.lspPath": "/path/to/txtx",
  
  // Enable verbose logging for debugging
  "txtx.trace.server": "verbose",
  
  // File associations
  "files.associations": {
    "*.tx": "txtx",
    "txtx.yml": "yaml"
  }
}
```

### Environment Variables

The extension automatically detects txtx in this order:
1. `txtx.lspPath` setting
2. `TXTX_BIN` environment variable
3. `./target/debug/txtx` (development binary)
4. `./target/release/txtx` (release binary)
5. `txtx` in system PATH

## Usage

### Basic Usage

1. **Open a txtx project** containing `txtx.yml`
2. **Open any `.tx` file** - The LSP will activate automatically
3. **Start typing** - You'll see:
   - Auto-completions after typing `input.`
   - Error squiggles for undefined variables
   - Hover tooltips on functions and actions

### Go-to-Definition

- **Click + Cmd/Ctrl** on any `input.variable_name`
- **Press F12** with cursor on the reference
- **Right-click** → "Go to Definition"

This will jump to the variable definition in `txtx.yml`.

### Auto-completion

Completions appear automatically when you type:
- `input.` - Shows all available input variables
- `evm::` - Shows all EVM functions and actions
- `action.` - Shows all defined actions in the runbook
- `variable.` - Shows all defined variables

### Hover Information

Hover over any identifier to see:

```hcl
// Hovering over a function
evm::encode_calldata("transfer", ["0x123...", 100])
// Shows: Function signature, parameter types, return value, example usage

// Hovering over an action type
action "deploy" "evm::deploy_contract" {
// Shows: Required inputs, available outputs, documentation link

// Hovering over a variable reference
input.private_key
// Shows: Current value (masked for secrets), which environment it comes from
```

## Troubleshooting

### Extension Not Loading

1. **Check Developer Tools**: Help → Toggle Developer Tools → Console
   - Look for extension activation errors
   - Check for missing dependencies

2. **Verify txtx is accessible**:
   ```bash
   txtx --version
   ```

3. **Check extension logs**: View → Output → Select "txtx Language Server"

### LSP Not Starting

1. **Enable verbose logging**:
   ```json
   { "txtx.trace.server": "verbose" }
   ```

2. **Check the output panel** for startup errors

3. **Test LSP directly**:
   ```bash
   echo '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}' | txtx lsp
   ```

### Features Not Working

#### Go-to-Definition Not Working
- Ensure `txtx.yml` exists in the project root or parent directory
- Check syntax: use `input.` (singular), not `inputs.`
- Verify the variable exists in the manifest

#### Hover Not Showing
- Ensure file has `.tx` extension
- Try reloading the VSCode window: Cmd+Shift+P → "Developer: Reload Window"
- Check if hovering exactly on the identifier

#### Completions Not Appearing
- Completions trigger after typing `input.`, `action.`, etc.
- Try manually triggering: Ctrl+Space
- Check if the document was saved at least once

### Performance Issues

- The LSP uses synchronous processing and should be very responsive
- If slow, check for very large manifest files
- Enable verbose logging to identify bottlenecks

## Editor Integration

### VSCode (Recommended)
Full support via the official extension with all features.

### Neovim
```lua
-- In your Neovim config
require'lspconfig'.txtx.setup{
  cmd = {'txtx', 'lsp'},
  filetypes = {'txtx'},
  root_dir = require'lspconfig'.util.root_pattern('txtx.yml', 'Txtx.toml'),
}
```

### Helix
```toml
# In languages.toml
[[language]]
name = "txtx"
scope = "source.txtx"
file-types = ["tx"]
language-server = { command = "txtx", args = ["lsp"] }
```

### Other Editors
Any editor supporting the Language Server Protocol can use txtx LSP:
- **Command**: `txtx lsp`
- **Communication**: stdio
- **Root markers**: `txtx.yml` or `Txtx.toml`

## Tips and Tricks

### 1. Multi-file Runbook Navigation
The LSP understands multi-file runbooks defined in txtx.yml:
```yaml
runbooks:
  deploy:
    - setup.tx
    - deploy.tx
    - verify.tx
```
Variables and actions are resolved across all files.

### 2. Environment-Aware Completion
The LSP searches for variables in priority order:
1. Specified environment (e.g., `production`)
2. `global` environment
3. `default` environment

### 3. Quick Validation
Save your file to trigger validation. Errors appear:
- In the Problems panel (Ctrl+Shift+M)
- As red squiggles in the editor
- In the output panel with details

### 4. Function Documentation
All addon functions have built-in documentation. Hover over any function to see:
- Full signature
- Parameter descriptions
- Return value type
- Usage examples

## Reporting Issues

If you encounter issues:

1. **Collect logs**:
   - Set `"txtx.trace.server": "verbose"`
   - Copy relevant output from "txtx Language Server" output panel

2. **Create minimal reproduction**:
   - Smallest `.tx` file that shows the problem
   - Relevant `txtx.yml` content

3. **Report at**: https://github.com/sst/opencode/issues

## See Also

- [LSP_ARCHITECTURE.md](LSP_ARCHITECTURE.md) - Implementation details for developers
- [DOCTOR_COMMAND.md](DOCTOR_COMMAND.md) - Validation command that powers diagnostics
- [txtx Documentation](https://docs.txtx.sh) - General txtx documentation