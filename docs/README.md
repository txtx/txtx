# Txtx Documentation

Welcome to the txtx documentation. This guide covers everything from user guides to architecture details.

## 📚 User Guides

Start here if you're using txtx to validate runbooks and write blockchain automation.

- [**Linter Guide**](user/linter-guide.md) - Validate runbooks with `txtx lint`
- [**Linter Configuration**](user/linter-configuration.md) - Command-line options and output formats

## 🛠 Developer Documentation

For contributors and maintainers working on txtx itself.

- [**Developer Guide**](developer/DEVELOPER.md) - Development setup, workflows, and contributing
- [**Testing Guide**](developer/TESTING_GUIDE.md) - Testing strategies, utilities, and conventions
- [**Validation Architecture**](developer/VALIDATION_ARCHITECTURE.md) - Deep dive into the validation system
- [**API Documentation**](https://docs.rs/txtx) - Generated Rust documentation (or run `cargo doc --open --no-deps`)

## 🏗️ Architecture

Understand the txtx architecture, design decisions, and performance characteristics.

### Component Documentation

- [**Architecture Overview**](architecture/README.md) - Hybrid documentation approach and C4 models
- [**Linter Architecture**](architecture/linter/architecture.md) - Multi-layer validation pipeline
- [**Feature Behavior**](architecture/features.md) - Linter feature scoping

### Historical Reports

- [**Performance Improvements**](architecture/performance-improvements.md) - August 2024 async refactoring achievements

### Architecture Decision Records

Understand why key architectural decisions were made:

- [ADR 001: Parallel Runbook Validation](adr/001-pr-architectural-premise.md)
- [ADR 003: Capture Everything Pattern](adr/003-capture-everything-filter-later-pattern.md)
- [ADR 004: Visitor Strategy Pattern](adr/004-visitor-strategy-pattern-with-readonly-iterators.md)

## 📋 Internal Documents

Planning and future features.

- [**Linter Plugin System**](internal/linter-plugin-system.md) - Future extensible validation system (Phases 2-4)

## 📖 Examples

- [**Validation Errors**](examples/validation-errors.md) - Common validation errors with fixes

## 🎯 Quick Links

- [Project README](../README.md) - Getting started with txtx
- [Test Utils README](../crates/txtx-test-utils/README.md) - Testing utilities
- [VSCode Extension](../vscode-extension/README.md) - Editor extension

## Getting Help

- **Issues**: [GitHub Issues](https://github.com/txtx/txtx/issues)
- **Discussions**: [GitHub Discussions](https://github.com/txtx/txtx/discussions)
