# Unicode Support in EVM Addon

## Overview
The EVM addon fully supports Unicode (UTF-8) strings in smart contract interactions, enabling global applications with international character support.

## Supported Character Sets

### 1. Emoji Characters
- Full emoji support including compound emojis
- Example: `🚀`, `🎉`, `🎊`, `🎈`, `🎆`, `🎇`, `✨`
- Tested with: Person names containing emojis

### 2. International Languages
- **Chinese (中文)**: Full support for simplified and traditional characters
  - Example: `张三`, `这是一个很长的中文字符串`
- **Japanese (日本語)**: Hiragana, Katakana, and Kanji
  - Example: `田中さん`
- **Arabic (العربية)**: Right-to-left text support
  - Example: `مرحبا`
- **Korean (한국어)**: Hangul characters
- **Russian (Русский)**: Cyrillic alphabet
- And many more...

### 3. Special Characters
- Mathematical symbols: `∀x∈ℝ: x²≥0 ∑∏∫√∞`
- Zero-width joiners (ZWJ) for compound emojis
- Directional marks (RTL, LTR)
- Combining characters and diacritics

## Implementation Details

### Smart Contract Storage
The SimpleStorage contract used for testing demonstrates:
```solidity
struct People {
    string name;        // Stores UTF-8 encoded strings
    uint256 favoriteNumber;
}

mapping(string => uint256) public nameToFavoriteNumber;
```

### Test Coverage
Located in `src/tests/integration/unicode_storage_tests.rs`:

1. **Basic Unicode Test** (`test_unicode_storage_and_retrieval`)
   - Stores and retrieves various Unicode strings
   - Verifies data integrity across different character sets
   - Tests mapping lookups with Unicode keys

2. **Edge Cases Test** (`test_unicode_edge_cases`)
   - Empty strings
   - Very long Unicode strings
   - Special Unicode characters (ZWJ, RTL marks)
   - Mathematical and symbolic characters

### Fixture Structure
The test fixture (`fixtures/integration/unicode_storage.tx`) demonstrates:
```hcl
# Store emoji in smart contract
action "store_emoji" "evm::call_contract_function" {
    contract_address = action.deploy_storage.contract_address
    function_signature = "addPerson(string,uint256)"
    function_args = ["Alice 🚀 Rocket", 100]
    signer = signer.deployer
}

# Store Chinese characters
action "store_chinese" "evm::call_contract_function" {
    contract_address = action.deploy_storage.contract_address
    function_signature = "addPerson(string,uint256)"
    function_args = ["张三", 200]
    signer = signer.deployer
}
```

## Usage Examples

### Storing Unicode Data
```hcl
action "store_international" "evm::call_contract_function" {
    contract_address = "0x..."
    function_signature = "setMessage(string)"
    function_args = ["Hello 世界 🌍"]
    signer = signer.user
}
```

### Retrieving Unicode Data
```hcl
action "get_message" "evm::call_contract_function" {
    contract_address = "0x..."
    function_signature = "getMessage()"
}

output "international_message" {
    value = action.get_message.result
}
```

## Technical Considerations

### Encoding
- All strings are UTF-8 encoded before being sent to the blockchain
- The EVM stores strings as dynamic byte arrays
- Proper encoding/decoding is handled automatically by the txtx framework

### Gas Costs
- Unicode strings may use more bytes than ASCII
- Gas costs scale with byte size, not character count
- Example: "A" = 1 byte, "世" = 3 bytes, "🚀" = 4 bytes

### Compatibility
- Full compatibility with Solidity string type
- Works with all EVM-compatible chains
- Transparent handling through standard ABI encoding

## Best Practices

1. **Always test with actual Unicode data** - Don't assume ASCII-only
2. **Consider gas costs** - Unicode can be 2-4x more expensive than ASCII
3. **Validate string lengths** - Count bytes, not characters
4. **Test edge cases** - Empty strings, very long strings, special characters
5. **Use appropriate data types** - `string` for text, `bytes` for binary data

## Testing Your Unicode Support

Run the Unicode tests:
```bash
cargo test --package txtx-addon-network-evm unicode_storage_tests
```

The tests will verify:
- Storage and retrieval integrity
- Mapping key functionality with Unicode
- Edge case handling
- Multiple character set support

## Limitations

1. **String comparison** - Solidity doesn't normalize Unicode for comparison
2. **Character counting** - No built-in way to count Unicode characters (only bytes)
3. **Sorting** - Unicode collation is not natively supported

## Conclusion

The txtx EVM addon provides robust Unicode support, enabling truly global blockchain applications. All string operations transparently handle UTF-8 encoding, making it easy to build international dApps without worrying about character encoding issues.