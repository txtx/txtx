# Linter Command Demo Fixtures

This directory contains demonstration fixtures for the `txtx lint` command, showcasing its ability to catch common runbook errors before runtime.

## Overview

The lint command is a static analysis tool that validates txtx runbooks, checking for:

- References to non-existent action outputs
- Missing input values in environment configuration
- Invalid syntax patterns
- Common mistakes that lead to runtime errors
- Generates CLI commands for runbook execution (--gen-cli)

## Structure

```console
lint_demo/
├── runbooks/
│   ├── correct_transfer.tx      # Example of correct usage
│   ├── problematic_transfer.tx  # Common mistakes to avoid
│   └── markdown_fixture.md      # Markdown content for testing
└── txtx.yml                     # Manifest with test environment
```

## Running the Demos

### Basic Linter Check

Check for errors in the problematic runbook:

```bash
# Check the problematic runbook
txtx lint ./runbooks/problematic_transfer.tx

# Expected output shows errors like:
# Error: Field 'from' does not exist on action 'transfer' (evm::send_eth)
# Available fields: tx_hash
```

### Validate Correct Usage

```bash
# Check the correct runbook (should pass)
txtx lint ./runbooks/correct_transfer.tx

# Expected: No errors
```

### Generate CLI Templates

The lint command can generate CLI templates showing what inputs are needed:

```bash
# Generate CLI for undefined variables only
txtx lint ./runbooks/correct_transfer.tx --gen-cli

# Output:
# txtx run correct_transfer \
#   --input ALICE_PRIVATE_KEY="$ALICE_PRIVATE_KEY" \
#   --input ETHEREUM_CHAIN_ID="$ETHEREUM_CHAIN_ID" \
#   --input ETHEREUM_NETWORK_URL="$ETHEREUM_NETWORK_URL" \
#   --input RECIPIENT_ADDRESS="$RECIPIENT_ADDRESS"

# Generate CLI with all variables (including resolved values)
txtx lint ./runbooks/correct_transfer.tx --gen-cli-full

# Generate CLI with some inputs pre-filled
txtx lint ./runbooks/correct_transfer.tx --gen-cli \
  --input ETHEREUM_CHAIN_ID=1 \
  --input ETHEREUM_NETWORK_URL=https://mainnet.infura.io
```

## Runbooks

### `correct_transfer.tx`

Shows the correct way to use `send_eth`:

- Only accesses `tx_hash` output (which exists)
- Uses proper input references
- Demonstrates best practices

### `problematic_transfer.tx`

Contains common mistakes developers make:

- Trying to access `action.transfer.from` (doesn't exist)
- Attempting to use `action.transfer.value` (not an output)
- Missing or undefined input references

## Common Errors Detected

1. **Non-existent output fields**

   ```text
   Error: Field 'from' does not exist on action 'transfer' (evm::send_eth)
   The send_eth action only outputs: tx_hash
   ```

2. **Missing inputs**

   ```text
   Error: Input 'input.gas_price' is not defined in environment 'testing'
   Add 'gas_price' to the 'testing' environment in your txtx.yml file
   ```

3. **Invalid reference patterns**

   ```text
   Error: Cannot access field 'from' on 'tx_hash' - tx_hash is a string value
   ```

## Using the Linter Command

```bash
# Check all runbooks in manifest
txtx lint

# Check specific runbook
txtx lint problematic_transfer

# Check with specific environment
txtx lint --env testing problematic_transfer

# Check a file directly
txtx lint ./runbooks/problematic_transfer.tx
```

## Why This Matters

Before the lint command, these errors would only surface at runtime with unhelpful messages like:

- "DependencyNotComputed"
- "Failed to evaluate expression"
- "Unknown error occurred"

Now developers get immediate, actionable feedback during development, saving hours of debugging time.
