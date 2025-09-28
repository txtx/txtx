# Common Validation Errors

This document showcases common validation errors you might encounter when writing txtx runbooks. All examples are generated using `txtx lint --format doc`.

## Table of Contents

- [Undefined Flow Input](#undefined-flow-input)
- [Undefined Variable](#undefined-variable)
- [Circular Dependencies](#circular-dependencies)
- [Missing Required Input](#missing-required-input)
- [Type Mismatches](#type-mismatches)
- [Undefined Signer](#undefined-signer)

## Undefined Flow Input

When you reference a flow field that doesn't exist in any flow definition:

**Example:**

```hcl
flow "deployment" {
  chain_id = "1"
  api_url = "https://api.example.com"
}

action "deploy" {
  constructor_args = [
    flow.missing_field
  ]
}
```

**Error output:**

```
example.tx:

  6 │ action "deploy" {
  7 │   constructor_args = [
  8 │     flow.missing_field
    │     ^^^^^^^^^^^^^ error: Undefined flow input 'missing_field'
  9 │   ]
 10 │ }
```

**Fix:** Ensure the field is defined in your flow, or update the reference to use an existing field like `flow.chain_id`.

---

## Undefined Variable

Referencing a variable that hasn't been defined:

**Example:**

```hcl
action "deploy" {
  network = variable.network_id
}
```

**Error output:**

```
example.tx:

  1 │ action "deploy" {
  2 │   network = variable.network_id
    │             ^^^^^^^^^^^^^^^^^^^ error: Undefined variable 'network_id'
  3 │ }
```

**Fix:** Define the variable before using it:

```hcl
variable "network_id" {
  value = "mainnet"
}

action "deploy" {
  network = variable.network_id
}
```

---

## Circular Dependencies

When variables or actions depend on each other in a circle:

**Example:**

```hcl
variable "a" {
  value = variable.b
}

variable "b" {
  value = variable.a
}
```

**Error output:**

```
example.tx:

  1 │ variable "a" {
  2 │   value = variable.b
    │           ^^^^^^^^^^ error: Circular dependency detected: a -> b -> a
  3 │ }
```

**Fix:** Break the circular dependency by removing one of the references or restructuring your variables.

---

## Missing Required Input

When manifest defines required inputs that aren't provided:

**Manifest (txtx.yml):**

```yaml
environments:
  production:
    inputs:
      api_key: required
```

**Runbook:**

```hcl
action "call_api" {
  url = "https://api.example.com"
  # Missing: api_key = input.api_key
}
```

**Error output:**

```
example.tx:

  1 │ action "call_api" {
    │ ^^^^^^^^^^^^^^^^^^^ error: Required input 'api_key' not used in runbook
  2 │   url = "https://api.example.com"
  3 │ }
```

**Fix:** Use the required input from the manifest:

```hcl
action "call_api" {
  url = "https://api.example.com"
  api_key = input.api_key
}
```

---

## Type Mismatches

When a value doesn't match the expected type:

**Example:**

```hcl
variable "amount" {
  value = "not_a_number"
}

action "transfer" {
  amount = variable.amount  // Expected: number
}
```

**Error output:**

```
example.tx:

  5 │ action "transfer" {
  6 │   amount = variable.amount
    │            ^^^^^^^^^^^^^^^ error: Type mismatch: expected number, got string
  7 │ }
```

**Fix:** Ensure the variable has the correct type:

```hcl
variable "amount" {
  value = 100
}
```

---

## Undefined Signer

Referencing a signer that isn't defined in the manifest:

**Example:**

```hcl
action "deploy" {
  signer = signer.deployer
}
```

**Error output (without manifest):**

```
example.tx:

  2 │   signer = signer.deployer
    │            ^^^^^^^^^^^^^^^ error: Undefined signer 'deployer'
```

**Fix:** Define the signer in your manifest (txtx.yml):

```yaml
environments:
  global:
    signers:
      deployer:
        mnemonic: $DEPLOYER_MNEMONIC
```

---

## Using the Doc Format

All examples in this document were generated using:

```bash
txtx lint example.tx --format doc
```

This format is ideal for:
- Creating bug reports with full context
- Documenting validation behavior
- Sharing examples with your team
- Understanding error messages

The format shows:
- 2 lines of context before/after errors
- Aligned line numbers
- Caret indicators (`^^^`) pointing to exact error locations
- Clear error messages

## See Also

- [Linter Documentation](../user/lsp-guide.md#sharing-examples)
- [LSP Features](../lint-lsp-features.md)
- [txtx Language Reference](https://docs.txtx.sh)
