# EVM Addon Test Migration Documentation Index

## 📚 Overview

This index provides a central reference point for all test migration and fixture system documentation for the EVM addon. The migration effort aims to transform ~83 tests from direct Alloy usage to txtx framework integration using a YAML-based fixture system.

## STATUS 

- [ ] Initial test harness framework. The current tests are a little better than nonsense,
to test the harness workflow
  - [ ] Anvil interaction created; but is naive;
  - [ ] Could explore using a single instance of Anvil with CREATE2, or
  different deployers for testing
  - [x] Foundry contract framework works
  - [ ] Hardhat contract framework tbd
- [ ] Tests are not yet valid. There needs to be a better criteria/spec for them

Tests need to be validated. 

## 📁 Documentation Structure

### 1. **[TEST_MIGRATION_TRACKER.md](./TEST_MIGRATION_TRACKER.md)** - Test Migration Status 📊
**Purpose:** Document-based tracking of all 83 test migrations

**Contents:**
- Test-by-test migration status table
- Organized by source file
- Migration priority queue
- Fixture organization structure
- Summary statistics

**When to use:** 
- Check which tests need migration
- Find status of specific test
- Update migration progress
- See migration priorities

---

### 2. **[TEST_HARNESS_TRACKER.md](./TEST_HARNESS_TRACKER.md)** - Session Progress 📈
**Purpose:** Session logs and overall project status

**Contents:**
- Session accomplishments
- Known issues and blockers
- Next actions
- Quick status dashboard

**When to use:** 
- Review session history
- Check current blockers
- Find next actions
- See overall progress

---

### 3. **[TEST_MIGRATION_GUIDE.md](./TEST_MIGRATION_GUIDE.md)** - How-To Guide 📖
**Purpose:** Practical guide for migrating tests to fixtures

**Contents:**
- Migration patterns and templates
- Step-by-step migration process
- Before/after code examples
- Best practices and anti-patterns
- Troubleshooting common issues
- Utility functions and helpers

**When to use:**
- Converting a legacy test to fixture
- Learning fixture patterns
- Finding example migrations
- Debugging migration issues

---

### 4. **[TEST_ARCHITECTURE.md](./TEST_ARCHITECTURE.md)** - Technical Documentation 🔧
**Purpose:** Technical documentation of the test architecture

**Contents:**
- ProjectTestHarness design
- Test project structure
- Integration with txtx-core
- Test patterns and examples
- Anvil integration
- Known limitations

**When to use:**
- Understanding test system
- Writing new tests
- Debugging test issues
- Technical reference

---

### 5. **[FIXTURE_PROJECTS.md](./FIXTURE_PROJECTS.md)** - Fixture Projects Guide 📦
**Purpose:** Documentation for complete fixture projects with contracts

**Contents:**
- Available fixture projects
- Project structure and contracts
- Adding new fixture projects
- Usage in integration tests

**When to use:**
- Testing with real contracts
- Complex integration scenarios
- Multi-contract deployments
- Full project lifecycle tests

---

## 🗺️ Quick Navigation

### By Task

| I want to... | Go to... |
|-------------|----------|
| See which tests need migration | [TEST_MIGRATION_TRACKER.md](./TEST_MIGRATION_TRACKER.md#migration-status-by-file) |
| Check a specific test's status | [TEST_MIGRATION_TRACKER.md](./TEST_MIGRATION_TRACKER.md) |
| Update migration progress | [TEST_MIGRATION_TRACKER.md](./TEST_MIGRATION_TRACKER.md#how-to-update-this-tracker) |
| Migrate a test | [TEST_MIGRATION_GUIDE.md](./TEST_MIGRATION_GUIDE.md#step-by-step-migration-process) |
| Write a new fixture | [fixtures/README.md](./src/tests/fixtures/README.md#test-definition-format) |
| Understand the architecture | [TEST_ARCHITECTURE.md](./TEST_ARCHITECTURE.md) |
| See what's blocking progress | [TEST_HARNESS_TRACKER.md](./TEST_HARNESS_TRACKER.md#known-issues--blockers) |
| Review session history | [TEST_HARNESS_TRACKER.md](./TEST_HARNESS_TRACKER.md#session-log) |

### By Role

**For Test Writers:**
- Start with [TEST_MIGRATION_GUIDE.md](./TEST_MIGRATION_GUIDE.md)
- Reference [fixtures/README.md](./src/tests/fixtures/README.md)
- Check examples in [fixtures/tests/](./src/tests/fixtures/tests/)

**For Project Managers:**
- Monitor [TEST_HARNESS_TRACKER.md](./TEST_HARNESS_TRACKER.md)
- Review progress dashboard and metrics
- Check blocking issues

**For System Architects:**
- Study [TEST_FIXTURE_DESIGN.md](./TEST_FIXTURE_DESIGN.md)
- Review component architecture
- Understand extension points

**For Contributors:**
- Read all documentation in order
- Start with small test migrations
- Follow patterns in [TEST_MIGRATION_GUIDE.md](./TEST_MIGRATION_GUIDE.md)

---

## 📈 Current Status Summary

**Migration Progress:** 12% (10 of 83 tests using txtx)

**System Status:**
- ✅ Phase 0: Core execution (Complete)
- ✅ Phase 1: Fixture system (Complete)
- 🚧 Phase 2: Action tests (5% - In Progress)
- ⏳ Phase 3: Codec tests (Pending)
- ⏳ Phase 4: Integration tests (Pending)

**Key Blocker:** Signer initialization issue preventing action tests

**Next Priority:** Fix signer issue and migrate remaining 38 action tests

---

## 🚀 Quick Start

### To migrate your first test:

1. **Check current status:**
   ```bash
   cat TEST_HARNESS_TRACKER.md | grep "Overall Progress" -A 5
   ```

2. **Find a test to migrate:**
   ```bash
   grep "🔴 Not Started" TEST_HARNESS_TRACKER.md
   ```

3. **Follow the migration guide:**
   - Open [TEST_MIGRATION_GUIDE.md](./TEST_MIGRATION_GUIDE.md#step-by-step-migration-process)
   - Find similar example pattern
   - Create YAML fixture
   - Test and validate

4. **Track your progress:**
   ```bash
   cargo test --package txtx-addon-network-evm -- --nocapture fixture_cli migrate old_test_name new_fixture.yml
   ```

---

## 🔧 Test Commands
```bash
# Run all EVM tests
cargo test --package txtx-addon-network-evm

# Run specific test file
cargo test --package txtx-addon-network-evm --test <test_name>

# Run integration tests
cargo test --package txtx-addon-network-evm integration::
```

---

## 📝 Contributing

When adding new documentation:
1. Update this index with the new document
2. Add cross-references in related documents
3. Update the tracker with progress

When migrating tests:
1. Follow patterns in [TEST_MIGRATION_GUIDE.md](./TEST_MIGRATION_GUIDE.md)
2. Update [TEST_HARNESS_TRACKER.md](./TEST_HARNESS_TRACKER.md) progress
3. Add fixture to appropriate directory
4. Run validation before committing

---

## 🔗 Related Resources

- **Fixture System Code:** [src/tests/fixture_system/](./src/tests/fixture_system/)
- **Example Fixtures:** [src/tests/fixtures/tests/](./src/tests/fixtures/tests/)
- **Project Test Harness:** [src/tests/project_test_harness.rs](./src/tests/project_test_harness.rs)
- **txtx Core Docs:** [../../crates/txtx-core/README.md](../../crates/txtx-core/README.md)

---


_For questions or updates, refer to [TEST_HARNESS_TRACKER.md](./TEST_HARNESS_TRACKER.md#known-issues--blockers) for current blockers and contact information._
