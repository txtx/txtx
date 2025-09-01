# EVM Addon Documentation

## Core Documentation

### 1. [README.md](../README.md) - Getting Started
The main entry point for users of the EVM addon. Contains:
- Feature overview
- Quick start examples
- Basic usage patterns
- API reference links

### 2. [ARCHITECTURE.md](./ARCHITECTURE.md) - System Design
Technical architecture of the EVM addon:
- Error-stack integration pattern
- Fixture-based testing system
- Contract compilation framework
- RPC and transaction handling

### 3. [TESTING.md](./TESTING.md) - Testing Guide
Comprehensive testing documentation:
- FixtureBuilder usage
- Writing integration tests
- Test patterns and best practices
- Anvil management

### 4. [DEVELOPMENT.md](./DEVELOPMENT.md) - Developer Guide
For contributors and maintainers:
- Adding new actions
- Error handling patterns
- Code organization
- Contributing guidelines

### 5. [FEATURES.md](./FEATURES.md) - Feature Documentation
Detailed documentation of specific features:
- CREATE2 deployments
- Unicode support
- View function detection
- Gas optimization

## Legacy Documentation (To Be Archived)

The following documents were created during development and migration but are no longer actively maintained:

### Migration & Planning Docs
- All `*_MIGRATION_*.md` files - Historical migration tracking
- All `*_TRACKER.md` files - Development progress tracking
- `PLAN_INDEX.md` - Old planning index
- `REFACTOR_TODO.md` - Completed refactoring tasks

### Test Migration Docs
- `TEST_HARNESS_*.md` - Old test harness documentation
- `FIXTURE_*_PLAN.md` - Planning documents
- `TEST_*_SUMMARY.md` - Migration summaries
- `*_CLEANUP.md` - Cleanup tasks

### Implementation Details
- Individual error handling docs (consolidated into ARCHITECTURE.md)
- Test analysis docs (information preserved in TESTING.md)
- Various TODO and tracking documents

## Recommended Action

1. Move legacy docs to `docs/archive/` directory
2. Keep only the consolidated documentation active
3. Update references in code to point to new docs
4. Maintain the five core documents going forward