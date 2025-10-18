# txtx Linter: Validation Rule System Proposal

## Executive Summary

This proposal outlines a phased approach to building an extensible, multi-chain validation system for txtx. The system will enable protocol-specific validation rules while maintaining a low barrier for teams and developers to add custom rules.

**Current State**: Basic input validation with static rules
**Target State**: Extensible validation supporting protocol-specific and team-defined rules
**Initial Milestone**: Ship current implementation, establish architecture for future expansion

---

## Background

### Current Implementation (Milestone 1 - Ready for PR)

The linter currently validates txtx runbooks at two levels:

1. **HCL Validation** (syntax, action types, circular dependencies)
2. **Input Validation** (undefined inputs, naming conventions, CLI overrides)

**Architecture:**

```text
┌─────────────────┐
│  Linter Entry   │
│   Point (CLI)   │
└────────┬────────┘
         │
         ▼
┌─────────────────────────────────┐
│   Workspace Analyzer            │
│  • Discovers runbooks           │
│  • Loads manifest               │
│  • Resolves environments        │
└────────┬────────────────────────┘
         │
         ▼
┌─────────────────────────────────┐
│   Validation Engine             │
│  ┌───────────────────────────┐  │
│  │ HCL Validator             │  │
│  │ • Syntax validation       │  │
│  │ • Action type checking    │  │
│  │ • Dependency graph        │  │
│  └───────────────────────────┘  │
│  ┌───────────────────────────┐  │
│  │ Input Validator           │  │
│  │ • Rule: InputDefined      │  │
│  │ • Rule: NamingConvention  │  │
│  │ • Rule: CliOverride       │  │
│  │ • Rule: SensitiveData     │  │
│  └───────────────────────────┘  │
└─────────────────────────────────┘
```

**Key Files:**

- `crates/txtx-cli/src/cli/linter/rules.rs` - Validation rules (refactored to function pointers)
- `crates/txtx-cli/src/cli/linter/validator.rs` - Validation engine
- `crates/txtx-core/src/validation/` - Core validation infrastructure

**Recent Improvements (Completed):**

- ✅ Refactored from trait objects to function pointers (zero-cost abstractions)
- ✅ Used `Cow<'static, str>` to avoid allocating static messages
- ✅ Separated severity from validation outcomes
- ✅ Split lifetimes for better type expressiveness (`'env`, `'content`)
- ✅ Made sensitive patterns data-driven (const arrays)

---

## Problem Statement

As txtx expands to support multiple blockchain protocols (EVM, Solana/SVM, Bitcoin, Stacks, etc.), we need validation that:

1. **Protocol-Aware**: Different chains have different constraints
   - EVM: gas limits, chain IDs, address formats (0x...)
   - Solana: program IDs, account ownership, rent exemption
   - Bitcoin: UTXO management, script sizes, fee rates
   - Stacks: contract names, clarity types, STX values

2. **Team-Customizable**: Organizations need to enforce their own policies
   - Forbidden operations (e.g., `selfdestruct`, `delegatecall`)
   - Value limits (e.g., max 1 ETH per transaction)
   - Approval requirements (e.g., large transfers need multi-sig)
   - Environment-specific rules (stricter for production)

3. **Low Barrier**: Adding rules shouldn't require deep txtx knowledge
   - Protocol developers should extend their own addons
   - Teams should define rules via configuration files
   - Rules should be testable in isolation

4. **Performant**: Validation should be fast for LSP real-time usage
   - Only run protocol rules when addons are active
   - Compile patterns once, not per-validation
   - Support parallel validation where possible

---

## Proposed Architecture

### Phase 1: Foundation (Milestone 1 - Current PR) ✅

**Goal**: Ship stable input validation with clean architecture

**Components:**

```rust
// Input-level validation (current implementation)
fn validate_input_defined(ctx: &ValidationContext) -> Option<ValidationIssue>
fn validate_naming_convention(ctx: &ValidationContext) -> Option<ValidationIssue>
fn validate_cli_override(ctx: &ValidationContext) -> Option<ValidationIssue>
fn validate_sensitive_data(ctx: &ValidationContext) -> Option<ValidationIssue>

// Simple, fast, zero-cost abstractions
type RuleFn = fn(&ValidationContext) -> Option<ValidationIssue>;
const DEFAULT_RULES: &[RuleFn] = &[...];
```

**What's Included:**

- ✅ Input validation (undefined, naming, CLI overrides, sensitive data)
- ✅ Multiple output formats (plain, JSON, GitHub, CSV)
- ✅ Workspace analysis (manifest discovery, environment resolution)
- ✅ LSP integration ready
- ✅ Comprehensive test coverage

**What's NOT Included:**

- ❌ Protocol-specific rules (EVM gas limits, Solana rent, etc.)
- ❌ Action-level validation (beyond type checking)
- ❌ Team configuration files (YAML/JSON rule definitions)
- ❌ External rule plugins

**Success Criteria:**

- All existing tests pass
- No performance regression
- LSP integration works
- Documentation updated

---

### Phase 2: Protocol Validation (Milestone 2)

**Goal**: Enable addons to provide protocol-specific rules

**Design Approach**: **Trait-Based Extensibility**

Unlike input validation (which has a fixed set of rules), protocol validation needs dynamic dispatch because:

- Addons are loaded dynamically at runtime
- Different addons provide different rules
- Rules need access to addon-specific context (specs, types, etc.)

**Architecture:**

```rust
// Protocol rules validate ACTION instances (not just inputs)
pub trait ProtocolValidationRule: Send + Sync {
    /// Unique identifier
    fn id(&self) -> RuleIdentifier;

    /// Does this rule apply to this action type?
    fn applies_to_action(&self, action_type: &str) -> bool;

    /// Validate an action instance
    fn validate_action(
        &self,
        action: &ActionContext,
        manifest: &WorkspaceManifest,
    ) -> Option<ValidationIssue>;
}

pub struct ActionContext<'a> {
    pub action_name: &'a str,
    pub action_type: &'a str,  // "evm::eth_call"
    pub spec: &'a CommandSpecification,
    pub inputs: &'a HashMap<String, Value>,
    pub environment: Option<&'a str>,
}
```

**Addon Integration:**

```rust
// Add to Addon trait (txtx-addon-kit/src/lib.rs)
pub trait Addon: Debug + Sync + Send {
    // ... existing methods ...

    /// Protocol-specific validation rules
    fn get_validation_rules(&self) -> Vec<Box<dyn ProtocolValidationRule>> {
        vec![]  // Default: no custom rules
    }
}
```

**Example: EVM Rules**

```rust
// addons/evm/src/validation.rs
pub struct EvmGasLimitRule;

impl ProtocolValidationRule for EvmGasLimitRule {
    fn id(&self) -> RuleIdentifier {
        RuleIdentifier::External("evm_gas_limit".into())
    }

    fn applies_to_action(&self, action_type: &str) -> bool {
        action_type.starts_with("evm::")
    }

    fn validate_action(
        &self,
        ctx: &ActionContext,
        _manifest: &WorkspaceManifest,
    ) -> Option<ValidationIssue> {
        // Only check contract calls
        if ctx.action_type != "evm::eth_call" {
            return None;
        }

        // Warn if gas_limit not specified
        if !ctx.inputs.contains_key("gas_limit") {
            return Some(ValidationIssue {
                rule: self.id(),
                severity: Severity::Warning,
                message: Cow::Borrowed("Gas limit not specified for contract call"),
                help: Some(Cow::Borrowed(
                    "Add gas_limit to prevent out-of-gas failures"
                )),
                example: Some("gas_limit = \"100000\"".to_string()),
            });
        }

        None
    }
}

// More EVM rules
pub struct EvmChainIdRule;     // Ensure chain_id matches network
pub struct EvmAddressRule;      // Validate 0x address format
pub struct EvmValueLimitRule;   // Warn on large value transfers

// Register in addon
impl Addon for EvmNetworkAddon {
    fn get_validation_rules(&self) -> Vec<Box<dyn ProtocolValidationRule>> {
        vec![
            Box::new(EvmGasLimitRule),
            Box::new(EvmChainIdRule),
            Box::new(EvmAddressRule),
            Box::new(EvmValueLimitRule),
        ]
    }
}
```

**Validation Flow:**

```text
1. Load runbook
2. Parse HCL → extract actions
3. Load addons used in runbook
4. Collect rules:
   - Core input rules (static)
   - Protocol rules from addons (dynamic)
5. For each action:
   - Run applicable protocol rules
6. For each input reference:
   - Run input rules
7. Aggregate results → format output
```

**Performance Optimizations:**

- Filter rules by `applies_to_action()` before running
- Use `AddonScope` to skip rules for inactive addons
- Cache addon rules (loaded once per linter instance)
- Parallel validation using rayon (future)

**Success Criteria:**

- EVM addon provides 3+ working rules
- Rules only run when EVM addon is active
- No performance regression for runbooks without protocols
- Documentation for addon developers

---

### Phase 3: Team Rules Configuration (Milestone 3)

**Goal**: Enable teams to define custom rules via YAML/JSON

**Use Cases:**

- Enforce organizational policies (forbidden actions)
- Set value limits (max transfer amounts)
- Require approvals (multi-sig for large transfers)
- Environment-specific constraints (stricter prod rules)

**Configuration Format:**

```yaml
# .txtx/rules.yml or txtx.yml
version: "1.0"
team: "DeFi Safety Team"

rules:
  # Forbidden actions
  - type: forbidden_action
    protocol: evm
    actions: ["eth_selfdestruct", "eth_delegatecall"]
    severity: error
    message: "These functions are forbidden by security policy"

  # Value limits
  - type: max_value
    protocol: evm
    action_pattern: "eth_.*"  # Regex
    input_name: "value"
    max_value: "1000000000000000000"  # 1 ETH in wei
    severity: error
    message: "Transaction value exceeds team limit (1 ETH)"

  # Required inputs
  - type: require_input
    protocol: evm
    action_pattern: "eth_call|eth_send"
    input_name: "gas_limit"
    environments: ["production"]
    severity: warning
    message: "Gas limit should be explicit in production"

  # Input validation
  - type: input_pattern
    protocol: evm
    input_name: "recipient"
    pattern: "^0x[a-fA-F0-9]{40}$"
    severity: error
    message: "Invalid Ethereum address format"
```

**Implementation:**

```rust
// txtx-core/src/validation/team_rules.rs
#[derive(Debug, Deserialize)]
pub struct TeamRulesConfig {
    pub version: String,
    pub team: Option<String>,
    pub rules: Vec<RuleSpec>,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
pub enum RuleSpec {
    #[serde(rename = "forbidden_action")]
    ForbiddenAction {
        protocol: String,
        actions: Vec<String>,
        severity: Severity,
        message: String,
    },

    #[serde(rename = "max_value")]
    MaxValue {
        protocol: String,
        action_pattern: String,
        input_name: String,
        max_value: String,
        severity: Severity,
        message: String,
    },

    #[serde(rename = "require_input")]
    RequireInput {
        protocol: String,
        action_pattern: String,
        input_name: String,
        environments: Option<Vec<String>>,
        severity: Severity,
        message: String,
    },

    #[serde(rename = "input_pattern")]
    InputPattern {
        protocol: String,
        input_name: String,
        pattern: String,
        severity: Severity,
        message: String,
    },
}

// Compiled rules (regex patterns cached)
pub struct CompiledTeamRule {
    spec: RuleSpec,
    action_matcher: Option<Regex>,
    pattern_matcher: Option<Regex>,
}

impl CompiledTeamRule {
    fn compile(spec: RuleSpec) -> Result<Self, Error> {
        let action_matcher = match &spec {
            RuleSpec::MaxValue { action_pattern, .. } |
            RuleSpec::RequireInput { action_pattern, .. } => {
                Some(Regex::new(action_pattern)?)
            }
            _ => None,
        };

        Ok(Self { spec, action_matcher, pattern_matcher: None })
    }
}

impl ProtocolValidationRule for CompiledTeamRule {
    fn validate_action(&self, ctx: &ActionContext, _: &WorkspaceManifest)
        -> Option<ValidationIssue>
    {
        // Implementation based on self.spec type
        match &self.spec {
            RuleSpec::ForbiddenAction { actions, message, severity, .. } => {
                if actions.contains(&ctx.action_type.to_string()) {
                    return Some(ValidationIssue {
                        severity: *severity,
                        message: Cow::Owned(message.clone()),
                        // ...
                    });
                }
            }
            // ... other rule types
        }
        None
    }
}
```

**Discovery & Loading:**

```rust
// Search for rules in:
// 1. .txtx/rules.yml (project-specific)
// 2. txtx.yml (in validation section)
// 3. ~/.txtx/rules.yml (user global)

impl Linter {
    fn load_team_rules(&mut self) -> Result<(), Error> {
        let config = TeamRulesConfig::discover_and_load()?;

        for spec in config.rules {
            let compiled = CompiledTeamRule::compile(spec)?;
            self.team_rules.push(Box::new(compiled));
        }

        Ok(())
    }
}
```

**Success Criteria:**

- Teams can define 4+ rule types via YAML
- Rules compile once at linter initialization
- Clear error messages for invalid configurations
- Documentation with examples
- Rule precedence: team rules override protocol defaults

---

### Phase 4: Advanced Features (Future)

**Potential Extensions:**

1. **Scripted Rules** (sandboxed execution)

   ```yaml
   - type: custom_script
     language: rhai  # or lua, wasm
     script: |
       if action.value > 1_000_000 && !action.has_approval {
         return error("Large transfers require approval");
       }
   ```

2. **Rule Composition**

   ```yaml
   - type: all_of
     rules:
       - type: require_input
         input_name: "gas_limit"
       - type: max_value
         input_name: "gas_limit"
         max_value: "1000000"
   ```

3. **Contextual Rules** (cross-action validation)

   ```yaml
   - type: approval_required
     condition: "total_value > 10_000"
     approvers: ["alice.eth", "bob.eth"]
     threshold: 2
   ```

4. **External Validators** (HTTP callbacks)

   ```yaml
   - type: external_validator
     url: "https://compliance.company.com/validate"
     timeout_ms: 1000
   ```

---

## Migration Path

### Milestone 1 → Milestone 2

- Add `get_validation_rules()` to `Addon` trait (with default impl)
- Existing addons continue to work (return empty vec)
- New EVM rules ship with EVM addon
- Linter loads both input rules (static) + protocol rules (dynamic)

### Milestone 2 → Milestone 3

- Team rules are optional (discovered, not required)
- If no `.txtx/rules.yml` exists, only protocol rules run
- Team rules compile to same `ProtocolValidationRule` trait
- No breaking changes to addon API

---

## Implementation Checklist

### Milestone 1: Current Implementation (Ready for PR) ✅

- [x] Refactor input validation to function pointers
- [x] Implement 4 core input rules
- [x] Support multiple output formats
- [x] Workspace analysis & manifest loading
- [x] LSP integration hooks
- [x] Test coverage (25+ tests passing)
- [x] Documentation (README.md)
- [ ] PR review & merge

### Milestone 2: Protocol Validation (8-10 weeks)

- [ ] Define `ProtocolValidationRule` trait
- [ ] Update `Addon` trait with `get_validation_rules()`
- [ ] Implement EVM validation rules (3-5 rules)
  - [ ] Gas limit warnings
  - [ ] Chain ID validation
  - [ ] Address format checking
  - [ ] Value limit warnings
- [ ] Update validator to collect & run addon rules
- [ ] Filter rules by active addons
- [ ] Add action-level context extraction
- [ ] Benchmark performance
- [ ] Documentation for addon developers
- [ ] Example: Solana validation rules

### Milestone 3: Team Rules Configuration (6-8 weeks)

- [ ] Define YAML schema for team rules
- [ ] Implement rule discovery (.txtx/rules.yml, etc.)
- [ ] Create `RuleSpec` deserialization
- [ ] Compile team rules to `ProtocolValidationRule`
- [ ] Cache compiled regex patterns
- [ ] Support 4+ rule types
- [ ] Clear error messages for invalid configs
- [ ] Documentation with examples
- [ ] Validation for rule files themselves

---

## Design Rationale

### Why Two Validation Levels?

**Input Validation** (static functions):

- Validates *references* to inputs (`input.api_key`)
- Fixed set of rules (naming, sensitivity, overrides)
- Pure functions, zero allocations
- Fast enough to run on every LSP keystroke

**Action Validation** (trait objects):

- Validates *action instances* with inputs
- Dynamic set from addons + teams
- Needs trait objects for extensibility
- Runs on save or explicit lint command

### Why Traits for Protocol Rules?

Function pointers work for static rules but break down for:

1. **Dynamic loading**: Addons loaded at runtime
2. **State**: Some rules need compiled regex, configuration
3. **Polymorphism**: Different addons, same interface
4. **Testing**: Can mock trait implementations

The small overhead of trait objects is acceptable because:

- Protocol rules run less frequently than input rules
- Addons already use trait objects (`Box<dyn Addon>`)
- Validation isn't in the hot path for execution

### Why YAML for Team Rules?

Configuration files (vs. code) because:

1. **Non-developers** can review and approve rules
2. **Version control** tracks policy changes
3. **Declarative** makes it clear what's enforced
4. **Tooling** can validate, lint, and suggest rules
5. **Portability** works across languages/editors

---

## Performance Considerations

### Current Performance

- Linter validates ~100 inputs in <10ms
- LSP can run on every keystroke
- No noticeable lag in editor

### Phase 2 Impact

- Protocol rules filtered by addon (cheap)
- `applies_to_action()` is O(1) string check
- Expect <5ms overhead per 100 actions
- Still fast enough for LSP

### Phase 3 Impact

- Regex compilation done once at startup
- Pattern matching is O(n) in action type
- YAML parsing ~5-10ms for typical config
- Cache compiled rules across validations

### Future Optimizations

- Parallel validation with rayon
- Incremental re-validation (only changed actions)
- Rule indexing (by protocol, by action type)
- WASM compilation for scripted rules

---

## Security Considerations

### Sandboxing (Phase 4)

- Scripted rules must run in sandbox
- Options: Rhai (safe Rust scripting), Wasmtime
- No file system access
- CPU/memory limits
- Timeout enforcement

### Team Rules Validation

- Schema validation on load
- Regex DoS protection (complexity limits)
- No arbitrary code execution
- Clear error messages (avoid info leaks)

### External Validators

- HTTPS only
- Timeout enforcement (1-5s)
- No sensitive data in requests
- Optional (teams must opt-in)

---

## Testing Strategy

### Milestone 1 (Current)

- ✅ Unit tests for each rule
- ✅ Integration tests (workspace analysis)
- ✅ LSP integration tests
- ✅ Format output tests

### Milestone 2

- Unit tests for each protocol rule
- Mock `ActionContext` for testing
- Test rule filtering by addon
- Performance benchmarks
- EVM addon integration tests

### Milestone 3

- YAML parsing tests (valid & invalid)
- Rule compilation tests
- Regex pattern tests
- Config discovery tests
- End-to-end team rule enforcement

---

## Documentation Plan

### User Documentation

- [ ] Linter CLI usage guide
- [ ] Available rules reference
- [ ] Output format guide
- [ ] LSP integration guide
- [ ] Team rules configuration guide (Phase 3)

### Developer Documentation

- [ ] Adding validation rules to addons
- [ ] `ProtocolValidationRule` trait guide
- [ ] Testing validation rules
- [ ] Performance best practices
- [ ] Rule architecture overview

---

## Success Metrics

### Milestone 1

- All existing linter tests pass
- Zero performance regression
- Documentation coverage >80%
- PR approved by 2+ reviewers

### Milestone 2

- 3+ addons implement custom rules
- <10ms validation overhead
- Developer docs published
- 2+ external contributors add rules

### Milestone 3

- 10+ teams using custom rules
- <5% performance regression
- Rule examples in docs
- Config validation catches 90%+ of errors

---

## Open Questions for Review

1. **Rule Severity Levels**: Should we support `info`, `warning`, `error`? Or just warning/error?

2. **Rule Configuration**: Should rules be configurable per-environment (stricter in prod)?

3. **Rule Precedence**: If both protocol and team rules fire, which takes priority?

4. **Breaking Changes**: When should we consider breaking the `Addon` trait?

5. **External Plugins**: Should we support loading external .so/.dylib rule plugins?

6. **Rule Discovery**: Should `.txtx/rules.yml` be convention, or configurable?

---

## Conclusion

This proposal establishes a clear path from our current stable implementation to a fully extensible, multi-chain validation system. By shipping Milestone 1 now, we provide immediate value while laying the groundwork for protocol-specific and team-defined rules.

The architecture balances:

- **Simplicity** (function pointers for static rules)
- **Extensibility** (traits for dynamic rules)
- **Performance** (filtering, caching, zero-copy where possible)
- **Developer Experience** (clear APIs, good docs, easy testing)

**Recommendation**: Approve Milestone 1 for immediate PR, begin design discussions for Milestone 2.
