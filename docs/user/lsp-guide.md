# Txtx Language Server Protocol (LSP) Guide

The txtx LSP provides intelligent code assistance for txtx runbooks in your editor, including real-time validation, auto-completion, hover information, and go-to-definition.

## Why IDE Integration?

### The Problem

Developing blockchain infrastructure without editor support is slow and error-prone:

- **Slow feedback loop**: Edit → Save → Run linter → Read output → Fix → Repeat
- **Context switching**: Jump between editor, terminal, and documentation
- **Cryptic errors**: Runtime errors provide little context about where things went wrong
- **Manual lookups**: Constantly referring to documentation for function signatures
- **Typos and references**: Easy to mistype action names, signer references, or input variables

### The Solution

The LSP brings validation and assistance **directly into your editor**:

- ✅ **Instant feedback**: Errors appear as you type, not after running a command
- ✅ **Stay in flow**: All information available via hover and completion
- ✅ **Jump to definitions**: <kbd>Ctrl+Click</kbd> on any reference to see where it's defined
- ✅ **Discover APIs**: Auto-completion shows available actions and their parameters
- ✅ **Catch errors early**: See undefined references before you even save the file

**Example**: Instead of running `txtx lint`, seeing "undefined signer 'deployer'", then searching through files, the LSP underlines the error in real-time and <kbd>Ctrl+Click</kbd> takes you to where signers are defined.

## Quick Start

### VSCode

1. Install the txtx extension from the marketplace or locally:

   ```bash
   cd vscode-extension
   npm install
   npm run build
   code --install-extension txtx-*.vsix
   ```

2. Open a folder containing `txtx.yml`
3. Start editing `.tx` files - LSP features activate automatically

### Neovim

Add to your config:

```lua
require('lspconfig').txtx.setup {
  cmd = { 'txtx', 'lsp' },
  root_dir = require('lspconfig').util.root_pattern('txtx.yml'),
  filetypes = { 'txtx', 'tx' },
}
```

### Other Editors

Any LSP-compatible editor can use txtx LSP:

```bash
# Start the LSP server
txtx lsp
```

## Features

### 🔍 Real-time Diagnostics

Get instant feedback on errors as you type:

- Syntax errors
- Undefined references
- Type mismatches
- Missing required fields
- Invalid parameters

### 📝 Auto-completion

Context-aware suggestions for:

- Action names and parameters
- Signer types and fields
- Variable references (`var.`, `input.`, `env.`)
- Action outputs (`action.<name>.<field>`)
- Addon functions

### 🎯 Go to Definition

Jump to where symbols are defined:

- <kbd>Ctrl</kbd>+Click (VSCode) or <kbd>gd</kbd> (vim) on:
  - Signer references → signer definition
  - Variable references → variable definition
  - Action references → action definition
  - Input references → manifest or CLI input

### 📖 Hover Information

Hover over symbols to see:

- Parameter types and descriptions
- Variable values and types
- Action output schemas
- Signer configuration details
- Function signatures

### 🔗 Document Links

Click on file paths to open them:

```hcl
action "deploy" "evm::deploy_contract" {
  contract = "./contracts/Token.sol"  # Clickable link
}
```

### 📁 Workspace Support

The LSP understands your entire workspace:

- Reads `txtx.yml` for environment configuration
- Validates across multiple runbook files
- Tracks dependencies between runbooks
- Supports monorepo structures

## Configuration

### VSCode Settings

Configure in `.vscode/settings.json`:

```json
{
  "txtx.trace.server": "off",
  "txtx.maxNumberOfProblems": 100,
  "txtx.enable": true,
  "txtx.validate.onSave": true,
  "txtx.validate.onType": true
}
```

### Environment Resolution

The LSP automatically detects your environment from:

1. `--env` flag in CLI commands
2. `TXTX_ENV` environment variable
3. Default environment in `txtx.yml`
4. Falls back to "development"

## Diagnostic Messages

### Error Severity Levels

- **Error** (Red) - Must fix before running
- **Warning** (Yellow) - Should fix, might cause issues
- **Information** (Blue) - Suggestions and best practices
- **Hint** (Gray) - Optional improvements

### Example Diagnostics

```console
[Error] Undefined signer 'deployer'
  The signer 'deployer' is referenced but not defined.
  Add a signer definition: signer "deployer" "evm::private_key" { ... }

[Warning] Hardcoded private key detected
  Avoid hardcoding sensitive data. Use input variables instead:
  private_key = input.deployer_key

[Info] Variable 'unused_var' is defined but never used
  Consider removing unused variables to keep runbooks clean.
```

## Advanced Features

### Multi-file Workspaces

The LSP handles complex workspace structures:

```console
project/
├── txtx.yml           # Workspace manifest
├── runbooks/
│   ├── deploy.tx      # Can reference ../contracts/
│   └── upgrade.tx     # Can reference other runbooks
├── contracts/
│   └── Token.sol
└── modules/
    └── common.tx      # Shared definitions
```

### Import Resolution

The LSP resolves imports and validates across files:

```hcl
# common.tx
signer "deployer" "evm::private_key" {
  private_key = input.deployer_key
}

# deploy.tx
import "../common.tx"

action "deploy" "evm::deploy_contract" {
  signer = signer.deployer  # LSP knows this is defined in common.tx
}
```

### Dynamic Environment Validation

The LSP validates against the active environment:

```yaml
# txtx.yml
environments:
  development:
    API_URL: "http://localhost:3000"
  production:
    API_URL: "https://api.example.com"
    API_KEY: "required-in-prod"
```

When editing with `production` environment active, the LSP will flag missing `API_KEY` references.

## Performance

### Incremental Updates

The LSP uses incremental parsing for performance:

- Only re-parses changed files
- Caches parsed ASTs
- Debounces rapid changes
- Lazy-loads workspace files

### Large Workspaces

For large workspaces:

1. Limit the number of problems: `"txtx.maxNumberOfProblems": 100`
2. Disable on-type validation: `"txtx.validate.onType": false`
3. Use `.txtxignore` to exclude files

## Troubleshooting

### LSP Not Starting

1. Check txtx is in your PATH:

   ```bash
   which txtx
   ```

2. Verify LSP works standalone:

   ```bash
   txtx lsp --version
   ```

3. Check editor logs:
   - VSCode: Output → txtx Language Server
   - Neovim: `:LspLog`

### No Diagnostics Showing

1. Ensure file has `.tx` extension
2. Check for `txtx.yml` in workspace root
3. Verify no syntax errors prevent parsing
4. Try restarting the LSP

### Incorrect Diagnostics

1. Save all files to ensure LSP has latest content
2. Check active environment matches expectations
3. Restart LSP to clear caches

### Performance Issues

1. Reduce validation frequency
2. Exclude large directories via `.txtxignore`
3. Increase debounce delay in settings

## VSCode Extension Commands

Available through Command Palette (<kbd>Cmd</kbd>+<kbd>Shift</kbd>+<kbd>P</kbd>):

- `txtx: Restart Language Server`
- `txtx: Show Output Channel`
- `txtx: Run Current Runbook`
- `txtx: Validate Workspace`
- `txtx: Generate CLI Command`

## Integration with CI/CD

The same validation engine powers both LSP and CLI:

```yaml
# .github/workflows/validate.yml
steps:
  - uses: actions/checkout@v3
  - run: cargo install txtx-cli
  - run: txtx lint --format json > results.json
  - run: |
      if [ $(jq '.summary.errors' results.json) -gt 0 ]; then
        exit 1
      fi
```

## Sharing Examples

The linter includes a documentation format perfect for sharing validation examples with colleagues or in bug reports:

```bash
txtx lint example.tx --format doc
```

This outputs clean, readable error messages with visual indicators:

```
example.tx:

  6 │ action "deploy" {
  7 │   constructor_args = [
  8 │     flow.missing_field
    │     ^^^^^^^^^^^^^ error: Undefined flow input 'missing_field'
  9 │   ]
 10 │ }
```

### Use Cases

- **Bug Reports**: Share complete context when reporting validation issues
- **Team Communication**: Show colleagues exactly what's failing and where
- **Documentation**: Include validation examples in your project documentation
- **Learning**: Understand txtx validation rules with real examples
- **Testing**: Capture expected validation output for test cases

The format automatically:
- Shows context (2 lines before/after each error)
- Aligns line numbers for readability
- Uses caret indicators (`^^^`) pointing to exact error locations
- Groups errors by file
- Skips irrelevant lines with ellipsis (`⋮`)

This format represents the same errors the LSP shows in your IDE, making it perfect for discussing validation behavior outside the editor.

## Contributing

The LSP implementation is in `crates/txtx-cli/src/cli/lsp/`. Key components:

- `mod.rs` - LSP server setup and message handling
- `diagnostics.rs` - Validation and diagnostic generation
- `handlers/` - Request handlers (completion, hover, etc.)
- `workspace/` - Workspace and document management

See [LSP Architecture](../developer/lsp-architecture.md) for implementation details.
