# txtx Linter Guide

The `txtx lint` command provides validation for your txtx runbooks, catching errors before runtime and suggesting improvements.

## Why Use the Linter?

### The Problem

Smart contract deployments and blockchain operations are **expensive** and **irreversible**:

- **Time**: Deploying a contract, waiting for confirmation, then discovering a configuration error wastes precious development time
- **Cost**: Every failed transaction costs gas fees - errors can add up to hundreds of dollars in wasted fees
- **Risk**: Configuration mistakes in production can lead to vulnerable deployments, compromised funds, or permanent lockups
- **Debugging**: Runtime errors in blockchain operations are cryptic and hard to diagnose

### The Solution

The linter catches these issues **before execution**:

- ✅ **Instant feedback**: Find errors in seconds, not minutes
- ✅ **Zero cost**: No gas fees wasted on preventable errors
- ✅ **Security**: Detect hardcoded keys and sensitive data before deployment
- ✅ **Confidence**: Deploy knowing your configuration is valid

**Example**: A missing environment variable that would cause a runtime error after 3 contract deployments (and associated gas costs) is caught immediately by the linter.

## Quick Start

```bash
# Lint a specific runbook
txtx lint path/to/runbook.tx

# Lint all runbooks in a workspace
txtx lint

# Generate CLI template for a runbook
txtx lint runbook.tx --gen-cli
```

## Command Line Options

```bash
txtx lint [OPTIONS] [RUNBOOK]
```

### Options

| Option | Description |
|--------|-------------|
| `--manifest-path PATH` | Path to txtx.yml (default: ./txtx.yml) |
| `--env ENV` | Environment to validate against |
| `--input KEY=VALUE` | CLI input overrides (triggers warnings) |
| `--format FORMAT` | Output format: `stylish` (default), `compact`, `json`, `doc` |
| `--gen-cli` | Generate CLI command template |
| `--gen-cli-full` | Generate CLI template with all options |
| `--fix` | Automatically fix fixable issues |
| `--no-color` | Disable colored output |

### Examples

```bash
# Lint using a specific manifest
txtx lint --manifest-path path/to/txtx.yml

# Validate against production environment
txtx lint --env production

# Provide CLI inputs (overrides manifest values)
txtx lint --input api_key=test123 --input region=us-west-1

# Use JSON format for CI/CD
txtx lint --format json

# Generate CLI command template
txtx lint deploy.tx --gen-cli
```

## Validation Features

The linter performs multiple levels of validation:

- **Syntax validation** - HCL parsing and structure
- **Semantic validation** - Action parameters, types, and references
- **Cross-reference validation** - Ensures all references (signers, actions, variables) exist
- **Environment validation** - Verifies environment variables are defined
- **Security checks** - Warns about hardcoded sensitive data

## Available Rules

The linter includes both **HCL validation** (syntax, structure, references) and **input validation rules** (environment-specific checks).

### HCL Validation (from txtx-core)

These checks run automatically and validate:
- **Syntax errors**: Invalid HCL structure
- **Undefined references**: Signers, actions, variables that don't exist
- **Action type format**: Must be `namespace::action` (e.g., `evm::deploy_contract`)
- **Circular dependencies**: Variables that reference each other in a loop

### Input Validation Rules

#### `input-defined` (Error)
Detects references to input variables that aren't defined in the manifest.

```hcl
variable "deployer" {
  value = input.DEPLOYER_KEY  # Error if DEPLOYER_KEY not in manifest
}
```

**Fix**: Add the input to your manifest's environment section:
```yaml
environments:
  production:
    inputs:
      DEPLOYER_KEY: "..."
```

#### `cli-input-override` (Warning)
Warns when CLI inputs override manifest environment values.

```bash
# manifest.yml defines CHAIN_ID=1 for production
txtx lint --env production --input CHAIN_ID=11155111  # Warning: overriding manifest value
```

**Rationale**: CLI overrides can lead to inconsistent deployments across environments.

#### `input-naming-convention` (Warning)
Checks for naming convention issues in input names.

```hcl
variable "api" {
  value = input._API_KEY  # Warning: starts with underscore
}

variable "chain" {
  value = input.CHAIN-ID  # Warning: contains hyphens
}
```

**Fix**: Use SCREAMING_SNAKE_CASE without leading underscores or hyphens:
- `_API_KEY` → `API_KEY`
- `CHAIN-ID` → `CHAIN_ID`

#### `sensitive-data` (Warning)
Detects potential sensitive data keywords in input names.

```hcl
variable "auth" {
  value = input.API_PASSWORD  # Warning: contains "password"
}

variable "access" {
  value = input.SECRET_TOKEN  # Warning: contains "secret" and "token"
}
```

**Detected patterns**: `password`, `secret`, `key`, `token`, `credential`

**Rationale**: Helps identify inputs that should be handled with extra care and never hardcoded.

### Severity Levels

#### Errors (Must Fix)

- Undefined signers, actions, or variables
- Invalid action parameters
- Type mismatches
- Missing required fields

#### Warnings (Should Fix)

- Hardcoded private keys or sensitive data
- Unused variables or outputs
- Deprecated syntax

#### Info (Suggestions)

- Naming convention violations
- Performance improvements
- Best practices

## Output Formats

### Stylish (Default)

Human-readable with color and context:

```console
✗ path/to/runbook.tx
  12:5  error  Undefined signer 'deployer'  undefined-reference
  25:3  warn   Hardcoded private key        security/no-hardcoded-keys

✗ 1 error, 1 warning
```

### Compact

One violation per line:

```console
path/to/runbook.tx:12:5: error - Undefined signer 'deployer' (undefined-reference)
path/to/runbook.tx:25:3: warning - Hardcoded private key (security/no-hardcoded-keys)
```

### JSON

Machine-readable for CI/CD integration:

```json
{
  "files": [
    {
      "path": "path/to/runbook.tx",
      "errors": 1,
      "warnings": 1,
      "messages": [
        {
          "line": 12,
          "column": 5,
          "severity": "error",
          "message": "Undefined signer 'deployer'",
          "rule": "undefined-reference"
        }
      ]
    }
  ],
  "summary": {
    "errors": 1,
    "warnings": 1,
    "files": 1
  }
}
```

### Doc

Documentation format with code context:

```console
runbook.tx:

  6 │ action "deploy" {
  7 │   constructor_args = [
  8 │     flow.api_key
    │     ^^^^^^^^^^^^ error: Undefined input 'api_key'
  9 │   ]
 10 │ }
```

## CLI Generation

The linter can generate CLI command templates for your runbooks:

### Basic Template

```bash
txtx lint deploy.tx --gen-cli
```

Output:

```bash
txtx run deploy \
  --input DEPLOYER_KEY="..." \
  --input TOKEN_ADDRESS="..."
```

### Full Template with Descriptions

```bash
txtx lint deploy.tx --gen-cli-full
```

Output:

```bash
txtx run deploy \
  --input DEPLOYER_KEY="..." `# Private key for deployment` \
  --input TOKEN_ADDRESS="..." `# Address of the token contract` \
  --env production
```

## Environment Validation

When using a workspace with environments, the linter validates against specific environments:

```bash
# Validate against production environment
txtx lint --env production

# Validate against development (with different requirements)
txtx lint --env development
```

### Environment Variable Validation

The linter checks that all `input.*` references have corresponding values:

```hcl
# runbook.tx
variable "api_key" {
  value = input.API_KEY  # Linter ensures API_KEY is defined
}
```

```yaml
# txtx.yml
environments:
  production:
    inputs:
      API_KEY: "prod-key-value"
  development:
    inputs:
      API_KEY: "dev-key-value"
```

**Important**: txtx environments are defined in `txtx.yml` manifest files, not OS environment variables. The linter validates against the inputs defined in your manifest's environment configuration.

## Common Issues and Solutions

### Issue: Undefined Signer

```console
error: Undefined signer 'deployer'
```

**Solution**: Add the signer definition to your runbook:

```hcl
signer "deployer" "evm::private_key" {
  private_key = input.DEPLOYER_KEY
}

action "deploy" "evm::deploy_contract" {
  signer = signer.deployer  # Now references existing signer
}
```

### Issue: Invalid Action Output Reference

```console
error: Action 'send_eth' only provides 'tx_hash' output
```

**Solution**: Reference only available outputs for the action type:

```hcl
action "send" "evm::send_eth" {
  // ...
}

output "transaction_hash" {
  value = action.send.tx_hash  # Correct field
}
```

### Issue: Missing Input

```console
error: Input 'API_KEY' not found
```

**Solution**: Add the input to your manifest's environment configuration:

```yaml
environments:
  production:
    inputs:
      API_KEY: "..."
```

### Issue: Circular Dependency

```console
error: Circular dependency detected: var_a -> var_b -> var_a
```

**Solution**: Break the circular reference by restructuring your variables. Variables in txtx are evaluated as a DAG - each variable can only reference other variables that don't create a cycle.

## Integration with CI/CD

### GitHub Actions Example

```yaml
name: Lint
on: [push, pull_request]

jobs:
  lint:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - name: Install txtx
        run: curl -L https://txtx.sh/install.sh | sh
      - name: Lint runbooks
        run: txtx lint --format json --env production
```

### Exit Codes

- `0` - No violations found
- `1` - Violations found (errors or warnings)
- `2` - Linter error (invalid manifest, parse errors, etc.)

## Integration with Editors

The linter powers real-time validation in editors through LSP:

- **VSCode**: Install the txtx extension for real-time linting
- **Neovim**: Use the included LSP configuration
- **Other editors**: Any LSP-compatible editor works

## Best Practices

1. **Run before commits**: Add to your pre-commit hooks
2. **Validate all environments**: Test against each target environment
3. **Fix warnings**: They often prevent future errors
4. **Use in CI/CD**: Ensure runbooks are valid before deployment
5. **Generate CLI templates**: Document required inputs for users

## Performance Tips

- The linter caches parsed files for faster subsequent runs
- Use specific file paths when iterating on a single runbook
- JSON output is fastest for CI/CD integration

## Troubleshooting

### Linter finds no runbooks

Ensure you're in a directory with `txtx.yml` or specify `--manifest-path`.

### Environment validation not working

Specify the environment explicitly with `--env`.

### Manifest not found

```bash
# Specify manifest location explicitly
txtx lint --manifest-path path/to/txtx.yml
```

### Environment not found

```bash
# Check available environments
txtx ls-envs

# Use correct environment name
txtx lint --env production
```

### Undefined input errors

- Ensure inputs are defined in your manifest under `environments.global.inputs` or `environments.<env>.inputs`
- Check for typos in input names
- Verify you're using the correct environment with `--env`

### False positives

Some dynamic patterns might trigger false positives. Use inline comments to suppress:

```hcl
# txtx-lint-disable-next-line undefined-reference
action "dynamic" "evm::call" {
  // ...
}
```
