# Compiler-Aware Architecture for txtx

## Executive Summary

This proposal outlines a transformation of txtx from a runtime-interpreted system to a properly compiled language with a multi-phase compilation pipeline. This architecture will enable static analysis, early error detection, advanced IDE support, and optimization opportunities while maintaining the flexibility of the current addon system.

## Current State

### Existing Architecture

- **Direct Interpretation**: HCL parser → Block structures → Runtime evaluation
- **Late Binding**: Addon capabilities discovered at runtime
- **Limited Static Analysis**: Most validation happens during execution
- **Ad-hoc Validation**: Scattered across different modules without clear phases

### Areas for Improvement

1. **Error Detection Timing**: Many errors only surface during execution
2. **IDE Support**: Limited autocomplete and type information opportunities
3. **Optimization Potential**: Direct interpretation limits optimization possibilities
4. **Type Safety**: Custom addon types validated at runtime rather than compile-time
5. **Cross-File Validation**: Not currently supported in the architecture

## Proposed Architecture

### Compilation Pipeline

```text
Source (.tx files)
    ↓
[1] Lexing/Parsing (HCL)
    ↓
[2] AST → HIR (High-level IR)
    ↓
[3] Semantic Analysis
    ↓
[4] HIR → MIR (Mid-level IR)
    ↓
[5] Optimization
    ↓
[6] Execution Planning
    ↓
Runtime Execution
```

### Phase Descriptions

#### Phase 1: Lexing/Parsing

- Leverage existing HCL parser
- Produce AST with location information
- Syntax error reporting

#### Phase 2: HIR Construction

- Convert HCL AST to txtx-specific high-level representation
- Resolve imports and modules
- Build initial symbol table
- Maintain source mapping for diagnostics

#### Phase 3: Semantic Analysis

- **Type Checking**: Validate all type constraints
- **Reference Resolution**: Resolve all variable, action, and output references
- **Dependency Analysis**: Build and validate dependency graph
- **Addon Validation**: Check addon-specific constraints
- **Cross-File Analysis**: Validate references across files

#### Phase 4: MIR Construction

- Lower HIR to execution-oriented representation
- Resolve all symbolic references
- Inline constants
- Prepare for optimization

#### Phase 5: Optimization

- Dead code elimination
- Constant folding
- Parallel execution analysis
- Addon-specific optimizations

#### Phase 6: Execution Planning

- Generate execution graph
- Identify parallelization opportunities
- Prepare runtime structures

## Addon Integration

### Current Addon System

```rust
trait Addon {
    fn get_namespace(&self) -> &str;
    fn get_functions(&self) -> Vec<FunctionSpecification>;
    fn get_actions(&self) -> Vec<PreCommandSpecification>;
    fn get_signers(&self) -> Vec<SignerSpecification>;
}
```

### Proposed Addon System

#### 1. Addon Manifest (compile-time)

```rust
// addon_manifest.rs
pub struct AddonManifest {
    pub namespace: String,
    pub version: String,
    pub types: Vec<TypeDefinition>,
    pub functions: Vec<FunctionSpecification>,
    pub actions: Vec<ActionSpecification>,
    pub signers: Vec<SignerSpecification>,
    pub validators: Vec<Box<dyn AddonValidator>>,
}

// Each addon provides:
impl BitcoinAddon {
    pub const MANIFEST: AddonManifest = AddonManifest {
        namespace: "bitcoin",
        types: vec![
            TypeDefinition::Custom("btc::Opcode", ...),
            TypeDefinition::Custom("btc::Address", ...),
        ],
        // ... specifications
    };
}
```

#### 2. Two-Phase Loading

```rust
// Phase 1: Compile-time (static specifications)
pub trait AddonStatic {
    fn manifest() -> &'static AddonManifest;
}

// Phase 2: Runtime (implementations)
pub trait AddonRuntime {
    fn get_implementation(&self, action: &str) -> Box<dyn CommandImpl>;
}
```

#### 3. Addon-Aware Type System

```rust
pub struct TypeRegistry {
    builtin_types: HashMap<String, Type>,
    addon_types: HashMap<String, HashMap<String, Type>>, // namespace -> type_name -> Type
}

pub enum Type {
    // Existing types
    String,
    Integer,
    Array(Box<Type>),
    // New addon-aware types
    Addon { namespace: String, name: String },
    Generic { constraints: Vec<TypeConstraint> },
}
```

#### 4. Static Validation API

```rust
pub trait AddonValidator {
    fn validate_action(
        &self,
        action: &str,
        inputs: &TypedValueMap,
        context: &SemanticContext,
    ) -> Result<TypeMap, ValidationError>;
    
    fn validate_function_call(
        &self,
        function: &str,
        args: &[TypedValue],
        context: &SemanticContext,
    ) -> Result<Type, ValidationError>;
}
```

## Implementation Plan

### Phase 1: Foundation (4-6 weeks)

#### Week 1: Design & Architecture

- [ ] Finalize compilation phase interfaces
- [ ] Design HIR and MIR structures
- [ ] Create addon manifest schema
- [ ] Plan backward compatibility strategy

#### Weeks 2-4: Core Infrastructure

- [ ] Implement compilation pipeline framework
- [ ] Build HIR from HCL AST
- [ ] Create type registry system
- [ ] Implement basic semantic analysis

#### Weeks 5-6: Addon Manifest System

- [ ] Define manifest format and schema
- [ ] Build manifest loader and validator
- [ ] Create manifest generation tooling
- [ ] Implement static addon registry

### Phase 2: Semantic Analysis (6-8 weeks)

#### Weeks 1-4: Type System & Symbol Table

- [ ] Build addon-aware symbol table
- [ ] Implement type inference engine
- [ ] Add cross-addon type compatibility
- [ ] Create reference resolution system

#### Weeks 5-6: Validation Framework

- [ ] Migrate existing validators to semantic phase
- [ ] Implement addon-specific validators
- [ ] Add circular dependency detection
- [ ] Enable cross-file validation

#### Weeks 7-8: Error Reporting

- [ ] Enhanced diagnostics with addon context
- [ ] Implement error recovery
- [ ] Add fix suggestions
- [ ] Create diagnostic rendering

### Phase 3: Addon Migration (4-6 weeks)

#### Weeks 1-3: Core Addons

- [ ] Migrate Bitcoin addon
  - [ ] Create manifest file
  - [ ] Define custom types
  - [ ] Add validators
- [ ] Migrate EVM addon
  - [ ] Complex type definitions
  - [ ] Contract interaction validation
- [ ] Migrate Stacks addon
  - [ ] Clarity contract validation
  - [ ] Custom signer types

#### Weeks 4-6: Secondary Addons

- [ ] Migrate SVM addon
- [ ] Migrate SP1 addon
- [ ] Migrate OVM addon
- [ ] Test cross-addon scenarios
- [ ] Performance optimization

### Phase 4: LSP Integration (3-4 weeks)

#### Weeks 1-2: Static Analysis APIs

- [ ] Hook LSP into compilation phases
- [ ] Implement incremental compilation
- [ ] Add semantic token provider
- [ ] Create completion providers

#### Weeks 3-4: IDE Features

- [ ] Go-to-definition for addon types
- [ ] Hover documentation from manifests
- [ ] Implement rename refactoring
- [ ] Add code actions

### Phase 5: Testing & Documentation (2-3 weeks)

#### Week 1-2: Testing

- [ ] Update test infrastructure for new pipeline
- [ ] Create migration test suite
- [ ] Performance benchmarks
- [ ] Cross-addon integration tests

#### Week 3: Documentation

- [ ] Architecture documentation
- [ ] Addon development guide
- [ ] Migration guide for existing addons
- [ ] API documentation

## Migration Strategy

### Backward Compatibility

1. **Dual Mode Operation**: Support both old and new pipelines initially
2. **Automatic Migration**: Tool to convert existing addons
3. **Deprecation Period**: 6-month transition period
4. **Feature Flags**: Enable new pipeline per addon

### Migration Path for Existing Code

```bash
# Phase 1: Analyze existing addon
txtx addon analyze bitcoin/

# Phase 2: Generate manifest
txtx addon generate-manifest bitcoin/ > bitcoin_manifest.toml

# Phase 3: Validate migration
txtx addon validate --new-pipeline bitcoin/

# Phase 4: Update addon code
txtx addon migrate bitcoin/
```

## Benefits

### Immediate (Phase 1-2)

- **Early Error Detection**: Catch type errors during parsing
- **Better Error Messages**: Context-aware diagnostics
- **Basic IDE Support**: Syntax highlighting and basic completions

### Medium-term (Phase 3-4)

- **Full Static Analysis**: Complete type checking before execution
- **Advanced IDE Features**: Go-to-definition, refactoring
- **Cross-Addon Validation**: Ensure type compatibility

### Long-term (Phase 5+)

- **Optimization**: Parallel execution, constant folding
- **Advanced Tooling**: Debugger, profiler, visual tools
- **Extensibility**: Easy to add new analysis passes

## Risk Mitigation

### Technical Risks

1. **Performance Overhead**
   - Mitigation: Incremental compilation, caching
   - Benchmark against current system

2. **Compatibility Issues**
   - Mitigation: Extensive testing, gradual rollout
   - Maintain backward compatibility mode

3. **Complexity**
   - Mitigation: Incremental development, clear phases
   - Regular architecture reviews

### Process Risks

1. **Team Ramp-up**
   - Mitigation: Knowledge sharing sessions
   - Pair programming on critical components

2. **Timeline Slippage**
   - Mitigation: MVP approach, feature flags
   - Regular milestone reviews

## Success Metrics

### Phase 1 Success Criteria

- [ ] Basic compilation pipeline working
- [ ] One addon (EVM) fully migrated
- [ ] 50% reduction in runtime errors

### Phase 2 Success Criteria

- [ ] All type errors caught at compile time
- [ ] LSP provides useful completions
- [ ] Cross-file validation working

### Phase 3 Success Criteria

- [ ] All addons migrated
- [ ] No performance regression
- [ ] Developer satisfaction improved

## Resource Requirements

### Team Composition

- **Technical Lead**: Architecture and design decisions
- **Core Team**: 3-4 senior engineers
- **Addon Team**: 2-3 engineers per addon
- **QA/Testing**: 1-2 engineers

### Timeline

- **Total Duration**: 19-27 weeks (5-7 months)
- **MVP (Phase 1)**: 6-8 weeks
- **Full Implementation**: Additional 13-19 weeks

### Dependencies

- No external dependencies
- Builds on existing HCL parser
- Leverages current type system

## Conclusion

This transformation represents a significant investment in txtx's future. By implementing a proper compilation pipeline with static analysis, we'll deliver:

1. **Better Developer Experience**: Earlier error detection, superior IDE support
2. **Improved Performance**: Optimization opportunities, parallel execution
3. **Enhanced Reliability**: Type safety, comprehensive validation
4. **Future Extensibility**: Foundation for advanced tooling

The phased approach allows us to deliver value incrementally while managing risk. The MVP phase provides immediate benefits with just 6-8 weeks of effort, making this a practical and achievable transformation.

## Appendix: Example Code

### Before (Current System)

```hcl
addon "bitcoin" "btc" {}

// Error only caught at runtime
action "script" "btc::encode_script" {
    instructions = [
        btc::op_dup(),
        btc::op_hash160(),
        btc::invalid_function(), // Runtime error!
    ]
}
```

### After (Compiler-Aware System)

```hcl
addon "bitcoin" "btc" {}

action "script" "btc::encode_script" {
    instructions = [
        btc::op_dup(),
        btc::op_hash160(),
        btc::invalid_function(), // Compile-time error with suggestion!
        //    ^^^^^^^^^^^^^^^^
        // Error: Unknown function 'btc::invalid_function'
        // Did you mean 'btc::op_invalidopcode'?
    ]
}
```

### IDE Experience

- Typing `btc::` shows all available Bitcoin functions
- Hover over `op_dup()` shows documentation
- Ctrl+click on `encode_script` jumps to action definition
- Refactoring updates all references across files
