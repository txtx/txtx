# txtx Linter

The `txtx lint` command validates runbooks before execution, catching errors instantly without gas fees or wasted time.

## Quick Start

```bash
# Lint all runbooks in workspace
txtx lint

# Lint a specific runbook
txtx lint my_runbook

# Validate against an environment
txtx lint --env production

# Generate CLI command template
txtx lint my_runbook --gen-cli
```

## Options

| Option | Description |
|--------|-------------|
| `-m, --manifest-file-path` | Path to txtx.yml |
| `--env` | Environment to validate against |
| `--input KEY=VALUE` | Provide CLI input values |
| `-f, --format` | Output format: `stylish`, `json`, `github`, `csv` |
| `--gen-cli` | Generate CLI command template for undefined inputs |
| `--gen-cli-full` | Generate CLI template with all inputs |
| `--config` | Path to linter config file |
| `--disable-rule` | Disable specific rules |
| `--only-rule` | Run only specific rules |

## Validation

The linter checks:

- **Syntax** - HCL parsing and structure
- **References** - Undefined signers, actions, variables
- **Circular dependencies** - Variables referencing each other
- **Inputs** - Missing or undefined environment inputs
- **Naming conventions** - snake_case enforcement
- **Sensitive data** - Hardcoded keys and secrets

## Output Formats

**stylish** (default) - Human-readable with colors and context

**json** - Machine-readable for CI/CD integration

**github** - GitHub Actions annotations

**csv** - Spreadsheet-compatible output
