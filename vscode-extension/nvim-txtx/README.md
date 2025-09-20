# nvim-txtx

Neovim plugin for txtx - Web3 infrastructure automation tool. Provides syntax highlighting, LSP support, and intelligent workspace navigation.

## Features

- 🎨 **Syntax highlighting** via Tree-sitter for `.tx` runbook files
- 🔧 **LSP support** (completions, diagnostics, hover, etc.) for both `.tx` and `txtx.yml` files
- 📝 **Intelligent workspace discovery** - automatically finds and parses `txtx.yml` manifest files
- 🔍 **Go-to-definition navigation**:
  - From manifest `location` fields → runbook files
  - From runbook `input.var` references → manifest definitions
  - From runbook `env.var` references → environment definitions
- ✏️ **Cross-file rename** - rename variables across manifest and all runbooks
- 🌍 **Environment management** - switch between environments defined in manifest
- 📋 **Workspace commands** for validating and exploring your txtx project
- 🚀 **Automatic parser compilation** on installation

## File Types

- **`.tx` files**: txtx runbook files with custom syntax (Tree-sitter highlighting)
- **`txtx.yml`/`txtx.yaml`**: YAML manifest files that define projects, runbooks, and environments

## Requirements

- Neovim >= 0.9.0
- `txtx` CLI installed (`cargo install --path crates/txtx-cli`)
- C compiler (gcc/clang) for building Tree-sitter parser
- Optional: `yq` for enhanced YAML parsing

## Installation

### Using [lazy.nvim](https://github.com/folke/lazy.nvim)

```lua
{
  "txtx/nvim-txtx",
  ft = { "txtx", "yaml" },
  build = "./scripts/build.sh",
  config = function()
    require("txtx").setup()
  end,
}
```

### Using [packer.nvim](https://github.com/wbthomason/packer.nvim)

```lua
use {
  'txtx/nvim-txtx',
  ft = { 'txtx', 'yaml' },
  run = './scripts/build.sh',
  config = function()
    require('txtx').setup()
  end
}
```

## Configuration

```lua
require('txtx').setup({
  -- LSP configuration
  lsp = {
    enabled = true,
    cmd = { "txtx", "lsp" },
    settings = {},
    capabilities = nil,
    on_attach = nil,
  },
  
  -- Tree-sitter configuration
  treesitter = {
    enabled = true,
  },
  
  -- Workspace features
  workspace = {
    enabled = true,
  },
  
  -- Navigation features
  navigation = {
    enabled = true,
  }
})
```

## Key Mappings

When in a txtx-related file (`.tx` or `txtx.yml`), the following mappings are available:

| Mapping | Description |
|---------|-------------|
| `gd` or `<C-]>` | Go to definition (manifest ↔ runbook navigation) |
| `gr` | Find all references across workspace |
| `<leader>rn` or `<F2>` | Rename symbol across all files |
| `K` | Show hover information (LSP) |
| `<leader>ca` | Code actions (LSP) |
| `<leader>f` | Format file (LSP) |

## Commands

### Workspace Commands

- `:TxtxSelectEnvironment` - Select active environment from manifest
- `:TxtxShowManifest` - Display parsed manifest structure
- `:TxtxListRunbooks` - List and open runbooks in workspace
- `:TxtxOpenRunbook` - Open a runbook from the manifest
- `:TxtxGotoManifest` - Navigate to workspace manifest file
- `:TxtxValidateWorkspace` - Check manifest and runbook consistency

### Utility Commands

- `:TxtxInfo` - Show plugin and workspace information
- `:TxtxCheck` - Run `txtx check` on current file
- `:TxtxDescribe` - Run `txtx describe` on current file
- `:TxtxBuildParser` - Build Tree-sitter parser

## Workspace Structure

The plugin understands the following txtx workspace structure:

```
project/
├── txtx.yml          # Manifest file (required)
├── deploy.tx         # Runbook files
├── setup.tx
└── modules/
    └── common.tx
```

### Manifest File (txtx.yml)

```yaml
name: my-project
id: my-project-id

runbooks:
  - name: Deploy Contract
    id: deploy
    location: deploy.tx
    description: Deploy smart contract to network
  
  - name: Setup Environment
    id: setup
    location: setup.tx
    description: Initialize environment

environments:
  default:
    network_url: "http://localhost:8545"
    private_key: "0x..."
  
  testnet:
    network_url: "https://testnet.example.com"
    private_key: "0x..."
```

### Navigation Examples

1. **Manifest → Runbook**: Place cursor on `location: deploy.tx` and press `gd` to open the runbook file

2. **Runbook → Manifest**: In a runbook, place cursor on `${input.network_url}` and press `gd` to jump to the environment definition

3. **Find References**: Place cursor on any variable name and press `gr` to see all uses across the workspace

4. **Rename**: Place cursor on a variable and press `<leader>rn` to rename it everywhere

## Workspace Discovery

The plugin automatically discovers your txtx workspace:

1. When opening a `.tx` file, it searches upward for `txtx.yml` or `txtx.yaml`
2. Stops searching at `.git` directory (workspace root)
3. Parses the manifest and builds a workspace context
4. Provides intelligent completions and navigation based on the manifest

## Troubleshooting

### Syntax highlighting not working
1. Run `:TxtxInfo` to check parser status
2. Run `:TxtxBuildParser` to rebuild the parser
3. Restart Neovim

### LSP not connecting
1. Ensure txtx CLI is installed: `cargo install --path crates/txtx-cli`
2. Check if txtx is in PATH: `which txtx`
3. Run `:TxtxInfo` to see LSP status
4. Check `:LspLog` for error messages

### Navigation not working
1. Ensure you have a valid `txtx.yml` in your project
2. Run `:TxtxValidateWorkspace` to check for issues
3. Run `:TxtxInfo` to see workspace status

## License

MIT