// SPDX-License-Identifier: MIT
pragma solidity ^0.8.19;

/**
 * @title TypeTestContract
 * @dev Comprehensive contract for testing all Solidity type conversions and ABI encoding
 */
contract TypeTestContract {
    // Events for testing log decoding
    event Transfer(address indexed from, address indexed to, uint256 value);
    event ComplexEvent(
        uint256 indexed id,
        address indexed user,
        string message,
        bytes data
    );
    
    // Custom errors for testing error decoding
    error InsufficientBalance(uint256 required, uint256 available);
    error InvalidAddress(address provided);
    error CustomError(string reason);
    
    // Struct definitions for testing
    struct SimpleStruct {
        address owner;
        uint256 value;
    }
    
    struct ComplexStruct {
        address maker;
        address taker;
        uint256 amount;
        uint256 expiry;
        bytes signature;
        SimpleStruct nested;
    }
    
    // Test all primitive types
    function testPrimitiveTypes(
        address addr,
        uint256 u256,
        uint128 u128,
        uint64 u64,
        uint32 u32,
        uint16 u16,
        uint8 u8,
        int256 i256,
        int128 i128,
        bool b,
        bytes32 b32,
        string memory str
    ) public pure returns (bytes memory) {
        return abi.encode(addr, u256, u128, u64, u32, u16, u8, i256, i128, b, b32, str);
    }
    
    // Test dynamic types
    function testDynamicTypes(
        bytes memory dynBytes,
        uint256[] memory uintArray,
        address[] memory addrArray,
        string[] memory strArray
    ) public pure returns (bytes memory) {
        return abi.encode(dynBytes, uintArray, addrArray, strArray);
    }
    
    // Test fixed arrays
    function testFixedArrays(
        uint256[3] memory fixedUints,
        address[2] memory fixedAddrs,
        bytes32[4] memory fixedBytes
    ) public pure returns (bytes memory) {
        return abi.encode(fixedUints, fixedAddrs, fixedBytes);
    }
    
    // Test structs
    function testSimpleStruct(
        SimpleStruct memory simple
    ) public pure returns (address, uint256) {
        return (simple.owner, simple.value);
    }
    
    function testComplexStruct(
        ComplexStruct memory complex
    ) public pure returns (bytes32) {
        return keccak256(abi.encode(complex));
    }
    
    // Test nested arrays
    function testNestedArrays(
        uint256[][] memory nestedUints,
        address[][] memory nestedAddrs
    ) public pure returns (bytes memory) {
        return abi.encode(nestedUints, nestedAddrs);
    }
    
    // Test tuple returns
    function testTupleReturn() public pure returns (
        address owner,
        uint256 balance,
        bool active,
        string memory name
    ) {
        return (
            address(0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb8),
            1000000000000000000,
            true,
            "Test"
        );
    }
    
    // Error triggering functions
    function triggerInsufficientBalance(uint256 required) public pure {
        uint256 available = 100;
        if (required > available) {
            revert InsufficientBalance(required, available);
        }
    }
    
    function triggerInvalidAddress(address addr) public pure {
        if (addr == address(0)) {
            revert InvalidAddress(addr);
        }
    }
    
    function triggerCustomError(string memory reason) public pure {
        revert CustomError(reason);
    }
    
    // Functions to test overflow/underflow
    function testUint8Overflow(uint8 value) public pure returns (uint8) {
        return value + 1;
    }
    
    function testIntUnderflow(int8 value) public pure returns (int8) {
        return value - 1;
    }
    
    // Test address validation
    function requireValidAddress(address addr) public pure returns (bool) {
        require(addr != address(0), "Zero address not allowed");
        require(uint160(addr) > 1000, "Address too small");
        return true;
    }
    
    // Test bytes encoding
    function testBytesConversion(
        bytes1 b1,
        bytes4 b4,
        bytes8 b8,
        bytes16 b16,
        bytes32 b32
    ) public pure returns (bytes memory) {
        return abi.encode(b1, b4, b8, b16, b32);
    }
    
    // Test function overloading
    function transfer(address to, uint256 amount) public pure returns (bool) {
        require(to != address(0), "Invalid recipient");
        require(amount > 0, "Amount must be positive");
        return true;
    }
    
    function transfer(address to, uint256 amount, bytes memory data) public pure returns (bool) {
        require(to != address(0), "Invalid recipient");
        require(amount > 0, "Amount must be positive");
        require(data.length > 0, "Data required");
        return true;
    }
    
    // Test payable functions
    function deposit() public payable returns (uint256) {
        require(msg.value > 0, "Must send ETH");
        return msg.value;
    }
    
    // Test view functions
    function getConstants() public pure returns (uint256, address, string memory) {
        return (42, address(0xdead), "constant");
    }
}
