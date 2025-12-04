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

# Generate CLI command template with minimal inputs defined
txtx lint my_runbook --gen-cli

# Generate CLI command template with all inputs defined
txtx lint my_runbook --gen-cli-full

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
| `--init` | Initialize a `.txtxlint.yml` config file |
| `--disable-rule` | Disable specific rules |
| `--only-rule` | Run only specific rules |

## Configuration

> **Note**: Configuration file support is experimental and may change.

Initialize a linter config file:

```bash
txtx lint --init
```

This creates `.txtxlint.yml` with recommended defaults:

```yaml
extends: "txtx:recommended"
rules:
  undefined_input: error
  cli_input_override: info
ignore:
  - "examples/**"
  - "tests/**"
```

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
