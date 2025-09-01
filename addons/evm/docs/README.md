# EVM Addon Documentation

## Overview

The txtx EVM addon provides comprehensive support for Ethereum and EVM-compatible blockchains. This documentation is organized to help different audiences find the information they need quickly.

## Documentation Structure

### For Users

Start with the main [README.md](../README.md) which provides:
- Quick start examples
- Basic usage patterns  
- Feature overview
- Installation instructions

Then explore [FEATURES.md](./FEATURES.md) for detailed feature documentation:
- Transaction management
- Smart contract deployment and interaction
- ABI encoding/decoding
- Advanced features like CREATE2 and Unicode support

### For Developers

1. **[ARCHITECTURE.md](./ARCHITECTURE.md)** - System Design
   - Error-stack integration pattern
   - Component architecture
   - Design patterns
   - Performance considerations

2. **[DEVELOPMENT.md](./DEVELOPMENT.md)** - Developer Guide
   - Adding new actions
   - Error handling patterns
   - Code organization
   - Contributing guidelines

3. **[TESTING.md](./TESTING.md)** - Testing Guide
   - FixtureBuilder usage
   - Writing integration tests
   - Test patterns and best practices
   - Debugging tests

### Feature-Specific Documentation

Located in the main addon directory:
- [CREATE2_DEPLOYMENT.md](../CREATE2_DEPLOYMENT.md) - Deterministic contract deployment
- [UNICODE_SUPPORT.md](../UNICODE_SUPPORT.md) - International character support
- [TESTING_GUIDE.md](../TESTING_GUIDE.md) - Comprehensive testing documentation

### Implementation Details

- [ERROR_STACK_ARCHITECTURE.md](../ERROR_STACK_ARCHITECTURE.md) - Error handling design
- [ERROR_STACK_PRESERVATION.md](../ERROR_STACK_PRESERVATION.md) - Context preservation patterns
- [IMPLEMENTATION_SUMMARY.md](../IMPLEMENTATION_SUMMARY.md) - Summary of implementation work

## Quick Links

### Common Tasks

- **Run tests**: See [TESTING.md#running-tests](./TESTING.md#running-tests)
- **Add new action**: See [DEVELOPMENT.md#adding-a-new-action](./DEVELOPMENT.md#adding-a-new-action)
- **Debug failing test**: See [TESTING.md#debugging-tests](./TESTING.md#debugging-tests)
- **Understand error**: See [ARCHITECTURE.md#error-handling](./ARCHITECTURE.md#error-handling-with-error-stack)

### Key Concepts

- **FixtureBuilder**: Test infrastructure system ([TESTING.md](./TESTING.md))
- **error-stack**: Error handling library ([ARCHITECTURE.md](./ARCHITECTURE.md))
- **Anvil singleton**: Test isolation system ([TESTING.md#anvil-management](./TESTING.md#anvil-management))
- **Named accounts**: Deterministic test accounts ([TESTING.md#named-accounts](./TESTING.md#named-accounts))

## Getting Help

1. Check the relevant documentation section
2. Look at example tests in `src/tests/integration/`
3. Review fixture examples in `fixtures/integration/`
4. See the [txtx documentation](https://docs.txtx.sh)

## Documentation Maintenance

This documentation follows a consolidated structure:
- **Active docs**: The 5 core documents in this directory
- **Legacy docs**: Historical documents in `archive/` subdirectory
- **Updates**: Keep documentation in sync with code changes

When making changes:
1. Update relevant documentation
2. Add examples if introducing new features
3. Update this README if adding new documents
4. Archive outdated documents rather than deleting