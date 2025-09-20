# Doctor Command Demo Fixtures

This directory contains demonstration fixtures for the `txtx doctor` command, showcasing its ability to catch common runbook errors before runtime.

## Overview

The doctor command is a static analysis tool that validates txtx runbooks, checking for:
- References to non-existent action outputs
- Missing input values in environment configuration
- Invalid syntax patterns
- Common mistakes that lead to runtime errors

## Structure

```
doctor_demo/
├── runbooks/
│   ├── correct_transfer.tx      # Example of correct usage
│   └── problematic_transfer.tx  # Common mistakes to avoid
├── txtx.yml                     # Manifest with test environment
├── doctor_demo.sh               # Basic demo script
├── doctor_with_links_demo.sh    # Demo with documentation links
└── test_doctor.sh              # Automated test script
```

## Demo Scripts

### `doctor_demo.sh`
Basic demonstration of the doctor command:
- Shows error detection for non-existent output fields
- Demonstrates helpful error messages
- Validates input references

Run with:
```bash
./doctor_demo.sh
```

### `doctor_with_links_demo.sh`
Enhanced demo showing documentation links:
- Includes links to txtx documentation
- Shows how to fix common errors
- Provides context for each error type

Run with:
```bash
./doctor_with_links_demo.sh
```

### `test_doctor.sh`
Automated test validating doctor command functionality:
- Ensures doctor catches expected errors
- Validates error message format
- Tests both correct and problematic patterns

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
   ```
   Error: Field 'from' does not exist on action 'transfer' (evm::send_eth)
   The send_eth action only outputs: tx_hash
   ```

2. **Missing inputs**
   ```
   Error: Input 'input.gas_price' is not defined in environment 'testing'
   Add 'gas_price' to the 'testing' environment in your txtx.yml file
   ```

3. **Invalid reference patterns**
   ```
   Error: Cannot access field 'from' on 'tx_hash' - tx_hash is a string value
   ```

## Using the Doctor Command

```bash
# Check all runbooks in manifest
txtx doctor

# Check specific runbook
txtx doctor problematic_transfer

# Check with specific environment
txtx doctor --env testing problematic_transfer

# Check a file directly
txtx doctor ./runbooks/problematic_transfer.tx
```

## Why This Matters

Before the doctor command, these errors would only surface at runtime with unhelpful messages like:
- "DependencyNotComputed"
- "Failed to evaluate expression"
- "Unknown error occurred"

Now developers get immediate, actionable feedback during development, saving hours of debugging time.