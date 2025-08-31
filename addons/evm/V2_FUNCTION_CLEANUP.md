# V2 Function Cleanup Plan

## Current State

We have several functions with both original and `_v2` versions:

### 1. `format_transaction_cost` / `format_transaction_cost_v2`
- **Original**: `pub fn format_transaction_cost(cost: i128) -> Result<String, String>`
- **V2**: `pub fn format_transaction_cost_v2(cost: i128) -> EvmResult<String>`
- **Usage**: Mixed - tests use both versions

### 2. `get_expected_address` / `get_expected_address_v2`
- **Original**: `pub fn get_expected_address(value: &Value) -> Result<Address, String>`
- **V2**: `pub fn get_expected_address_v2(value: &Value) -> EvmResult<Address>`
- **Usage**: Original (13 calls) vs V2 (4 calls)

### 3. `get_common_tx_params_from_args` / `get_common_tx_params_from_args_v2`
- **Original**: Returns `Result<CommonTransactionFields, String>`
- **V2**: Returns `EvmResult<CommonTransactionFields>`
- **Usage**: Needs investigation

## Migration Strategy

### Option 1: Complete Migration (Recommended)
1. Update all callers to use the `_v2` versions
2. Remove the original versions
3. Rename `_v2` functions to original names
4. Benefits: Clean API, consistent error handling

### Option 2: Compatibility Layer
1. Keep both versions
2. Have original versions call `_v2` and convert errors
3. Benefits: No breaking changes for existing code

### Option 3: Gradual Migration
1. Mark original versions as deprecated
2. Migrate callers over time
3. Remove deprecated versions in next major version

## Implementation Steps for Option 1

1. **Update all callers of original functions to handle `EvmResult`**
   - Convert `.map_err(|e| e.to_string())` where needed
   - Use `report_to_diagnostic()` for Diagnostic contexts

2. **Remove original functions**

3. **Rename _v2 functions**
   - `format_transaction_cost_v2` → `format_transaction_cost`
   - `get_expected_address_v2` → `get_expected_address`  
   - `get_common_tx_params_from_args_v2` → `get_common_tx_params_from_args`

4. **Update imports and exports**

5. **Run tests to verify**

## Files to Update

### High Priority (Core Functions)
- `/src/commands/actions/mod.rs` - Contains the function definitions
- `/src/codec/transaction/cost.rs` - Contains format_transaction_cost

### Callers to Update
- `/src/commands/actions/deploy_contract.rs` - Uses get_expected_address
- `/src/commands/actions/send_eth.rs` - Uses these functions
- `/src/commands/actions/call_contract.rs` - Uses these functions
- Various test files

## Risks

- Breaking existing code that depends on String errors
- Test failures if error handling isn't updated properly
- Potential runtime issues if error conversion is missed

## Decision

**Proceed with Option 1** - Complete the migration to have a clean, consistent API using error-stack throughout.