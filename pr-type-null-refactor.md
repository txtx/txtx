# PR: Split Type::Null into Null and TypedNull variants

## Summary

Refactor the `Type` enum to use two separate variants for null types, enabling Strum Display derive usage and fixing serialization bugs.

## Before

```rust
#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub enum Type {
    Bool,
    Null(Option<Box<Type>>),  // Single variant with Option
    Integer,
    // ...
}

impl Type {
    pub fn to_string(&self) -> String {
        match self {
            Type::Null(inner) => match inner {
                Some(inner_type) => format!("null<{}>", inner_type.to_string()),
                None => "null".to_string(),
            },
            // ... manual implementation for all variants
        }
    }
}
```

**Issues:**
- Manual `to_string()` implementation required
- Nested null serialization bug: `null<null<string>>` became `"null<Null>"`
- Array parsing bug: `"array[string]"` returned `Type::String` instead of `Type::Array(Box::new(Type::String))`
- No validation for empty inner types like `"null<>"` or `"array[]"`

## After

```rust
#[derive(Clone, Debug, Eq, PartialEq, Hash, StrumDisplay)]
pub enum Type {
    #[strum(serialize = "bool")]
    Bool,
    #[strum(serialize = "null")]
    Null,                        // Unit variant for untyped null
    #[strum(to_string = "null<{0}>")]
    TypedNull(Box<Type>),        // Separate variant for typed null
    #[strum(serialize = "integer")]
    Integer,
    // ...
}
```

**Improvements:**
- Strum Display derive handles serialization automatically
- Nested nulls serialize correctly: `Type::typed_null(Type::typed_null(Type::String))` → `"null<null<string>>"`
  - Guarded by: `test_deep_null_nesting`, `test_type_null_serialization_format`
- Array parsing fixed: `"array[string]"` → `Type::Array(Box::new(Type::String))`
  - Guarded by: `test_type_null_parsing`, `test_deep_array_nesting`
- Parser validates empty inner types with contextual error messages
  - Guarded by: `test_empty_inner_type_errors`, `test_invalid_inner_type_errors`
- Serde roundtrip consistency for all type variants
  - Guarded by: `test_type_null_serde_roundtrip`, `test_cross_nesting`

## Behavior Comparison

| Input | Before | After |
|-------|--------|-------|
| `Type::Null` | `"null"` | `"null"` |
| `Type::typed_null(Type::String)` | `"null<string>"` | `"null<string>"` |
| `Type::typed_null(Type::typed_null(Type::String))` | `"null<Null>"` | `"null<null<string>>"` |
| Parse `"array[integer]"` | `Type::Integer` | `Type::Array(Box::new(Type::Integer))` |
| Parse `"null<>"` | `Type::typed_null(???)` | Error: "empty inner type" |
| Parse `"array[]"` | Error (panic) | Error: "empty inner type" |

## A broken walkthrough

The `Type::try_from` string parser is used by serde deserialization when types appear as strings in config/manifest files. ABI/IDL codecs (SVM, EVM) construct types programmatically via `Type::array()`, but those types are serialized to strings for storage and later deserialized back.

**Roundtrip failure example:**

```rust
// ABI produces this type programmatically
let from_abi = Type::array(Type::String);

// Serialize to string (for storage/config)
let serialized = from_abi.to_string();  // "array[string]"

// Later, deserialize from config
let parsed = Type::try_from(serialized)?;

// BUG: parsed == Type::String, not Type::Array(String)
assert_eq!(from_abi, parsed);  // FAILS without fix
```

This affects any workflow where:
1. An action returns an array type derived from an ABI
2. That type is stored in a manifest or passed through config
3. The type is later parsed back for validation or further processing

The `doc/addons/actions.json` file contains type strings like `"typing": "array[buffer]"` that would be affected by the parsing bug.

## Files Changed

| File | Changes |
|------|---------|
| `types/types.rs` | Enum split, Strum derives, parser fixes |
| `types/functions.rs` | Pattern match update |
| `types/type_compatibility.rs` | New module with TypeChecker |
| `types/mod.rs` | Module export |
| `types/tests/mod.rs` | Comprehensive test coverage |

## Test Coverage

- Serialization format tests (7 cases)
- Parsing tests (6 cases)
- Serde roundtrip tests (5 cases)
- Deep nesting tests (6 cases)
- Error handling tests (10 cases)
