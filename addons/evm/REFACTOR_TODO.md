# EVM Addon Refactoring TODO

## Overview
This document tracks the ongoing refactoring work for the EVM addon, combining error-stack migration, test improvements, and code quality enhancements. Update this document with each commit to track progress.

## Current Status (December 2024)
- ✅ **Error-Stack Migration**: 100% COMPLETE
- ⚠️ **Test Coverage**: ~70-75% (needs improvement)
- ⚠️ **Test Documentation**: 60% of tests lack proper specs
- 🔄 **Branch Cleanup**: 113 commits need rebasing

---

## Phase 1: Immediate Priorities ⚡

### 1.1 Branch Cleanup (TODAY)
- [ ] Create backup branch: `git branch feat/evm-error-stack-backup`
- [ ] Interactive rebase to squash 113 commits into ~15-20 logical commits
- [ ] Follow REBASE_PLAN.md Option 1 for commit organization
- [ ] Test compilation after each major squash
- [ ] Force push and update PR description

### 1.2 Critical Test Coverage Gaps (THIS WEEK)
These modules have 0% test coverage and handle critical functionality:

#### RPC Module (`/src/rpc/mod.rs`)
- [ ] Test retry logic with exponential backoff
- [ ] Test connection pooling
- [ ] Test network failover scenarios
- [ ] Test rate limiting handling
- [ ] Test WebSocket subscriptions
- [ ] Add mock RPC server for testing

#### Signers Module (`/src/signers/`)
- [ ] Test secret key signing
- [ ] Test web wallet integration
- [ ] Test hardware wallet support (if applicable)
- [ ] Test key derivation paths
- [ ] Test mnemonic handling
- [ ] Test signature verification

#### Contract Verification (`/src/codec/verify/`)
- [ ] Test Sourcify integration
- [ ] Test Etherscan verification
- [ ] Test multi-file verification
- [ ] Test library linking verification

---

## Phase 2: Test Documentation 📝

### Priority 1: Critical Tests Without Specs
Update these test files with proper specifications using the template below:

- [ ] `anvil_harness.rs` - Add Anvil setup requirements and cleanup specs
- [ ] `txtx_commands_tests.rs` - Document command I/O specifications
- [ ] `transaction_management_tests.rs` - Add nonce management and retry specs

### Priority 2: Integration Tests Needing Clarity
- [ ] `codec_integration_tests.rs` - Specify codec behaviors and formats
- [ ] `check_confirmations_tests.rs` - Document confirmation and reorg handling
- [ ] `contract_interaction_tests.rs` - Add gas estimation and event specs
- [ ] `create2_deployment_tests.rs` - Document salt generation and address prediction

### Priority 3: Migrated Tests
- [ ] `migrated_abi_tests.rs` - Update docs for error-stack patterns
- [ ] `migrated_deployment_tests.rs` - Document new deployment flow
- [ ] `migrated_transaction_tests.rs` - Specify transaction types and gas pricing

### Test Documentation Template
```rust
//! Test Module: [Module Name]
//! 
//! ## Purpose
//! [Clear statement of what this test module validates]
//! 
//! ## Requirements
//! - REQ-001: [Specific, measurable requirement]
//! - REQ-002: [Another requirement]
//! 
//! ## Test Scenarios
//! ### Scenario 1: [Name]
//! **Given**: [Initial conditions]
//! **When**: [Action taken]
//! **Then**: [Expected outcome]
//! 
//! ## Edge Cases
//! - [Edge case and how it's handled]
//! 
//! ## Performance Criteria
//! - [Execution time/resource limits]
```

---

## Phase 3: Code Quality Improvements 🔧

### 3.1 Module Organization
- [ ] Review module structure for logical grouping
- [ ] Ensure consistent naming conventions
- [ ] Remove any remaining deprecated code
- [ ] Update module documentation

### 3.2 Error Handling Enhancements
- [x] ~~Migrate all functions to use EvmResult<T>~~
- [x] ~~Remove all String error returns~~
- [x] ~~Add contextual error attachments~~
- [ ] Add error recovery suggestions where applicable
- [ ] Implement structured error codes for programmatic handling

### 3.3 Performance Optimizations
- [ ] Profile gas estimation functions
- [ ] Optimize ABI encoding/decoding
- [ ] Implement caching for repeated RPC calls
- [ ] Add benchmarks for critical paths

---

## Phase 4: Integration Testing 🧪

### 4.1 End-to-End Scenarios
- [ ] Multi-step deployment and interaction flow
- [ ] Upgrade proxy contract scenario
- [ ] Multi-sig wallet interaction
- [ ] DEX interaction scenario
- [ ] NFT minting and transfer

### 4.2 Stress Testing
- [ ] Concurrent transaction handling (100+ txs)
- [ ] Large batch operations
- [ ] Network interruption recovery
- [ ] Gas price spike handling

### 4.3 Edge Cases
- [ ] Blockchain reorg during confirmation
- [ ] Nonce gap handling
- [ ] Invalid chain ID scenarios
- [ ] Insufficient funds with pending transactions

---

## Phase 5: Documentation 📚

### 5.1 User Documentation
- [ ] Update README with error-stack patterns
- [ ] Create troubleshooting guide
- [ ] Document common error scenarios and solutions
- [ ] Add example runbooks for common tasks

### 5.2 Developer Documentation
- [x] ~~ERROR_STACK_MIGRATION.md - Complete~~
- [x] ~~TEST_COVERAGE_REPORT.md - Complete~~
- [x] ~~TESTS_NEEDING_SPECS.md - Complete~~
- [ ] API documentation with examples
- [ ] Architecture decision records (ADRs)

### 5.3 Code Comments
- [ ] Add rustdoc comments to all public functions
- [ ] Document complex algorithms
- [ ] Add examples in doc comments
- [ ] Generate and review rustdoc output

---

## Tracking Metrics 📊

### Test Coverage Progress
```
Module                | Current | Target | Status
---------------------|---------|--------|--------
ABI Encoding/Decode  | 90%     | 95%    | ✅
Transaction Building | 85%     | 90%    | ✅
Contract Deployment  | 80%     | 90%    | ⚠️
Contract Interaction | 75%     | 85%    | ⚠️
Error Handling       | 60%     | 80%    | ⚠️
Gas Estimation       | 50%     | 80%    | ❌
Transaction Signing  | 40%     | 80%    | ❌
RPC Operations       | 0%      | 80%    | ❌
Signers             | 0%      | 80%    | ❌
Verification        | 0%      | 70%    | ❌
```

### Documentation Progress
```
Category              | Complete | Total | Status
----------------------|----------|-------|--------
Test Specifications   | 3        | 32    | ❌ 9%
Module Documentation  | 5        | 12    | ⚠️ 42%
Public API Docs       | 20       | 50    | ⚠️ 40%
Integration Examples  | 5        | 15    | ⚠️ 33%
```

---

## Commit Checklist ✓

For each commit, update this document:

1. **Mark completed items** with ~~strikethrough~~
2. **Update metrics** if test coverage or docs change
3. **Add new findings** to appropriate sections
4. **Note blockers** or dependencies
5. **Update status** percentages

### Next Commit Should Focus On:
1. Branch cleanup via interactive rebase
2. RPC module test implementation
3. Test documentation for Priority 1 files

---

## Blockers & Dependencies 🚧

### Current Blockers
- None

### Dependencies
- Anvil must be installed for integration tests
- Foundry required for contract compilation tests

---

## Success Criteria ✅

### Definition of Done
- [ ] 100% error-stack migration (COMPLETE)
- [ ] 80%+ test coverage for all modules
- [ ] All tests have proper specifications
- [ ] Clean git history (15-20 commits)
- [ ] All public APIs documented
- [ ] Performance benchmarks established
- [ ] Zero string errors in codebase
- [ ] Integration test suite covers all major scenarios

### Review Checklist
- [ ] Code compiles without warnings
- [ ] All tests pass
- [ ] Documentation is current
- [ ] No TODO comments remain
- [ ] Error messages are helpful
- [ ] PR description summarizes changes

---

## Notes & Observations 📝

### Lessons Learned
- Error-stack provides much better debugging context
- Fixture-based testing improves maintainability
- Test harness abstraction enables better test coverage
- Enum-based error matching is more reliable than string contains

### Technical Debt Identified
- Some test files are too large (500+ lines)
- Integration tests could benefit from more helper functions
- Mock implementations could reduce test complexity
- Some error messages could be more actionable

### Future Improvements
- Consider property-based testing for codec functions
- Add fuzzing for transaction building
- Implement snapshot testing for complex outputs
- Create error documentation generator

---

*Last Updated: December 2024*
*Update this timestamp with each modification*