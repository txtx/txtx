# Linter Configuration Guide

The txtx linter validates your runbooks and manifests for common errors and best practices.

## Current Configuration Options

### Command-Line Options

The linter is currently configured through command-line flags:

```bash
# Lint a specific runbook
txtx lint path/to/runbook.tx

# Lint using a specific manifest
txtx lint --manifest path/to/txtx.yml

# Use a specific environment from manifest
txtx lint --env production

# Provide CLI inputs (overrides manifest values)
txtx lint --input api_key=test123 --input region=us-west-1

# Choose output format
txtx lint --format stylish    # Default: colored, grouped by file
txtx lint --format json        # Machine-readable JSON
txtx lint --format compact     # One-line per violation
txtx lint --format doc         # Documentation format with context
```

### Output Formats

**Stylish** (default) - Colored, grouped by file with context:
```
runbook.tx:
  8:5  error  Undefined input 'api_key'  undefined-input
  12:3 warning CLI input overrides manifest  cli-override
```

**JSON** - Machine-readable for CI/CD integration:
```json
{
  "files": [
    {
      "path": "runbook.tx",
      "violations": [
        {
          "rule": "undefined-input",
          "severity": "error",
          "message": "Undefined input 'api_key'",
          "line": 8,
          "column": 5
        }
      ]
    }
  ]
}
```

**Compact** - One violation per line:
```
runbook.tx:8:5: error: Undefined input 'api_key' (undefined-input)
runbook.tx:12:3: warning: CLI input overrides manifest (cli-override)
```

**Doc** - For documentation with code context:
```
runbook.tx:

  6 │ action "deploy" {
  7 │   constructor_args = [
  8 │     flow.api_key
    │     ^^^^^^^^^^^^ error: Undefined input 'api_key'
  9 │   ]
 10 │ }
```

## Validation Rules

### Currently Implemented Rules

| Rule ID | Description | Severity |
|---------|-------------|----------|
| `undefined-input` | Input variables must be defined in manifest | error |
| `cli-override` | Warns when CLI inputs override manifest values | warning |

### Rule Behavior

**undefined-input** - Detects references to inputs that aren't defined:
```hcl
# This will error if 'database_url' is not in manifest
action "migrate" {
  url = input.database_url
}
```

**cli-override** - Warns when CLI inputs shadow manifest values:
```bash
# If api_key is defined in manifest, this warns
txtx lint --input api_key=override_value
```

## Environment-Based Validation

The linter validates against a specific txtx environment:

```bash
# Validate using production environment inputs
txtx lint --env production

# Validate using staging environment inputs
txtx lint --env staging

# Use global environment (default)
txtx lint
```

**Important**: txtx environments are defined in `txtx.yml` manifest files, not OS environment variables. The linter validates against the inputs defined in your manifest's environment configuration.

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

## Troubleshooting

### Common Issues

**"Manifest not found"**
```bash
# Specify manifest location explicitly
txtx lint --manifest path/to/txtx.yml
```

**"Environment not found"**
```bash
# Check available environments
txtx ls-envs

# Use correct environment name
txtx lint --env production
```

**"Undefined input" errors**
- Ensure inputs are defined in your manifest under `environments.global.inputs` or `environments.<env>.inputs`
- Check for typos in input names
- Verify you're using the correct environment with `--env`

## See Also

- [Linter Guide](./linter-guide.md) - Complete usage guide with examples
- [LSP Guide](./lsp-guide.md) - Real-time validation in your editor
- [Linter Architecture](../architecture/linter/architecture.md) - Technical implementation details

---

## 🚧 Future Configuration Features

The following features are planned but not yet implemented. See [internal/linter-plugin-system.md](../internal/linter-plugin-system.md) for details.

### Planned: Configuration Files

Future support for `.txtxlint.yml` configuration files:

```yaml
# Future: .txtxlint.yml
rules:
  undefined-input: error
  undefined-signer: error
  cli-override: warning
```

### Planned: Rule Management

- Enable/disable individual rules
- Customize rule severity levels
- Rule-specific configuration options
- Built-in presets (recommended, strict, minimal)

### Planned: Inline Rule Control

```hcl
# Future: inline rule disabling
# txtx-lint-disable-next-line undefined-variable
variable "dynamic" {
  value = env.MIGHT_NOT_EXIST
}
```

### Planned: Extended Rules

Additional validation rules in development:
- `undefined-signer` - Validate signer references
- `undefined-action` - Validate action references
- `undefined-variable` - Validate variable references
- `invalid-action-type` - Validate action types
- `sensitive-data` - Detect hardcoded secrets
- `input-naming` - Enforce naming conventions

### Planned: Plugin System

Custom rule plugins for organization-specific validation:

```yaml
# Future: plugin configuration
plugins:
  - ./custom-rules
```
