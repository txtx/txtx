# txtx Doctor Command

## Overview

The `txtx doctor` command is a comprehensive validation tool that helps developers identify and fix issues in their txtx runbooks before execution. It provides clear, actionable feedback with examples and suggestions for resolving problems.

## Command Usage

```bash
# Validate all runbooks in the workspace
txtx doctor

# Validate a specific runbook
txtx doctor my_runbook

# Validate with custom manifest path
txtx doctor --manifest-file-path ./custom/txtx.yml

# Validate with specific environment
txtx doctor --environment production

# Output in different formats
txtx doctor --format json        # JSON output for CI/CD
txtx doctor --format quickfix    # For editor integration
```

## Features

### 1. **Validation**

- **Manifest validation**: Checks txtx.yml structure and content
- **Runbook resolution**: Verifies all runbook files can be found
- **Input validation**: Ensures all required inputs are defined
- **Action validation**: Verifies action types exist and outputs are valid
- **Dependency checking**: Detects circular dependencies and missing references
- **Type checking**: Validates input types match expected values

### 2. **Multiple Output Formats**

#### Terminal Format (Default)

Pretty-printed output with colors and formatting:

```console
✓ No issues found!
```

Or with issues:

```console
Found 2 issue(s):

runbooks/deploy.tx:19:23 error[1]: Input 'input.private_key' is not defined in environment 'default'
   Add 'private_key' to your txtx.yml file
   Documentation: https://docs.txtx.sh/concepts/manifest#environments

runbooks/deploy.tx:25:15 error[2]: Action 'transfer' of type 'evm::send_eth' does not have output field 'result'
   Available outputs: tx_hash
```

#### JSON Format

Machine-readable format for CI/CD integration:

```json
{
  "issues": [
    {
      "severity": "error",
      "category": "input_validation",
      "message": "Input 'input.private_key' is not defined",
      "location": {
        "file": "runbooks/deploy.tx",
        "line": 19,
        "column": 23
      },
      "suggestion": "Add 'private_key' to your txtx.yml file",
      "documentation_link": "https://docs.txtx.sh/concepts/manifest#environments"
    }
  ],
  "summary": {
    "total_issues": 1,
    "errors": 1,
    "warnings": 0
  }
}
```

#### Quickfix Format

For editor integration (Vim, Neovim, VS Code):

```conosole
runbooks/deploy.tx:19:23: error: Input 'input.private_key' is not defined
runbooks/deploy.tx:25:15: error: Action 'transfer' does not have output field 'result'
```

### 3. **Clickable Error Locations**

In terminals that support hyperlinks (VS Code, iTerm2, modern terminals), error locations are clickable and will open the file at the exact line and column in your editor.

Example output with clickable links:

```console
runbooks/transfer.tx:21:23 error: accessing non-existent output field
                     ^^^^^ Click to jump to location
```

### 4. **Environment-Aware Validation**

The doctor command understands txtx's environment system:

```bash
# Validate using specific environment
txtx doctor --environment production

# Doctor checks variable inheritance
# Variables are resolved in order: specified env → global → defaults
```

Example manifest with environments:

```yaml
environments:
  global:
    rpc_url: "https://sepolia.infura.io/v3/YOUR_KEY"

  production:
    private_key: ${{ env.PRIVATE_KEY }}
    contract_address: "0x123..."

  development:
    private_key: "0xtest..."
    contract_address: "0x456..."
```

### 5. **Multi-File Runbook Support**

Doctor validates runbooks that span multiple files:

```yaml
runbooks:
  deploy:
    - setup.tx
    - deploy_contracts.tx
    - configure.tx
```

## Common Issues and Solutions

### 1. Missing Input Variables

**Issue:**

```console
Input 'input.private_key' is not defined in environment 'production'
```

**Solution:**
Add the missing input to your txtx.yml:

```yaml
environments:
  production:
    private_key: ${{ env.PRIVATE_KEY }}
```

### 2. Invalid Action Output Access

**Issue:**

```console
Action 'transfer' of type 'evm::send_eth' does not have output field 'result'
Available outputs: tx_hash
```

**Solution:**

Use the correct output field:

```hcl
output "transaction_hash" {
  value = action.transfer.tx_hash  // Not .result
}
```

### 3. Undefined Action References

**Issue:**

```console
Reference to undefined action 'deploy_contract'
```

**Solution:**

Ensure the action is defined before referencing it:

```hcl
action "deploy_contract" "evm::deploy_contract" {
  // ... configuration
}

// Now you can reference it
output "contract_address" {
  value = action.deploy_contract.contract_address
}
```

### 4. Circular Dependencies

**Issue:**

```console
Circular dependency detected: action1 → action2 → action3 → action1
```

**Solution:**
Restructure your runbook to eliminate circular references.

## Integration with Development Workflow

### VS Code Integration

The txtx VS Code extension automatically runs doctor validation and shows issues in the Problems panel. Issues are clickable for quick navigation.

### CI/CD Integration

Add doctor validation to your CI pipeline:

```yaml
# GitHub Actions example
- name: Validate txtx runbooks
  run: |
    txtx doctor --format json > doctor-report.json
    if [ $? -ne 0 ]; then
      cat doctor-report.json
      exit 1
    fi
```

### Pre-commit Hook

Add doctor validation as a git pre-commit hook:

```bash
#!/bin/bash
# .git/hooks/pre-commit
txtx doctor --format quickfix
if [ $? -ne 0 ]; then
  echo "txtx doctor found issues. Please fix them before committing."
  exit 1
fi
```

## Best Practices

1. **Run doctor before executing runbooks** - Catch issues early
2. **Use in CI/CD** - Prevent broken runbooks from reaching production
3. **Configure your editor** - Use quickfix format for seamless integration
4. **Check all environments** - Validate each environment separately
5. **Fix issues immediately** - Doctor issues often indicate runtime failures

## Exit Codes

- `0` - No issues found
- `1` - Validation errors found
- `2` - Doctor command errors (e.g., manifest not found)

## Examples

### Basic Validation

```bash
$ txtx doctor
🏥 Txtx Doctor Results

📊 Summary:
   Runbooks checked: 3
   Total validations: 127
   
✓ No issues found!
```

### With Errors

```bash
$ txtx doctor deploy
🏥 Txtx Doctor Results

📊 Summary:
   Runbooks checked: 1
   Actions validated: 5
   Outputs validated: 3

📋 Issues found:
   ❌ Errors: 2
   ⚠️  Warnings: 0

❌ Input Validation Issues (1 issue):

  runbooks/deploy.tx:19:23 error[1]: Input 'input.api_key' is not defined
  💡 Add 'api_key' to your txtx.yml file
  📝 Example:
     environments:
       default:
         api_key: "your-api-key"

❌ Output Validation Issues (1 issue):

  runbooks/deploy.tx:45:15 error[2]: Invalid output access
  Action 'send_funds' (type 'evm::send_eth') only provides: tx_hash
  You tried to access: from_address
```

### JSON Output for Scripts

```bash
$ txtx doctor --format json | jq '.summary'
{
  "total_issues": 2,
  "errors": 2,
  "warnings": 0,
  "runbooks_checked": 1
}
```

## Troubleshooting

### Doctor can't find my runbook

- Check that the runbook is defined in txtx.yml
- Verify the file path is correct relative to the manifest
- Ensure the .tx file exists

### Doctor shows no output

- Try running with `--format terminal` explicitly
- Check if stdout is being redirected
- Verify the txtx binary is up to date

### False positives

- Ensure you're using the latest version of txtx
- Check if custom addons are properly registered
- Verify environment variables are set correctly

## See Also

- [DOCTOR_ARCHITECTURE.md](DOCTOR_ARCHITECTURE.md) - Implementation details for developers
- [txtx Documentation](https://docs.txtx.sh) - Complete txtx documentation
