// SPDX-License-Identifier: MIT
pragma solidity ^0.8.19;

/**
 * @title FactoryContract
 * @dev Contract for testing CREATE and CREATE2 deployment patterns
 */
contract FactoryContract {
    event ContractDeployed(address indexed deployedAddress, bytes32 salt);
    
    // Minimal contract bytecode for testing
    // This is the bytecode for a contract that just returns 42
    bytes constant MINIMAL_BYTECODE = hex"602a60005260206000f3";
    
    // Deploy using CREATE
    function deployWithCreate(bytes memory bytecode) public returns (address) {
        address deployed;
        assembly {
            deployed := create(0, add(bytecode, 0x20), mload(bytecode))
        }
        require(deployed != address(0), "CREATE deployment failed");
        emit ContractDeployed(deployed, bytes32(0));
        return deployed;
    }
    
    // Deploy using CREATE2
    function deployWithCreate2(
        bytes memory bytecode,
        bytes32 salt
    ) public returns (address) {
        address deployed;
        assembly {
            deployed := create2(0, add(bytecode, 0x20), mload(bytecode), salt)
        }
        require(deployed != address(0), "CREATE2 deployment failed");
        emit ContractDeployed(deployed, salt);
        return deployed;
    }
    
    // Compute CREATE2 address
    function computeCreate2Address(
        bytes memory bytecode,
        bytes32 salt
    ) public view returns (address) {
        bytes32 hash = keccak256(
            abi.encodePacked(
                bytes1(0xff),
                address(this),
                salt,
                keccak256(bytecode)
            )
        );
        return address(uint160(uint256(hash)));
    }
    
    // Deploy with constructor arguments
    function deployWithConstructor(
        bytes memory bytecode,
        bytes memory constructorArgs
    ) public returns (address) {
        bytes memory deploymentBytecode = abi.encodePacked(bytecode, constructorArgs);
        return deployWithCreate(deploymentBytecode);
    }
    
    // Test double deployment (should fail)
    function testDoubleDeployment(bytes32 salt) public {
        // First deployment should succeed
        deployWithCreate2(MINIMAL_BYTECODE, salt);
        
        // Second deployment with same salt should fail
        deployWithCreate2(MINIMAL_BYTECODE, salt); // This will revert
    }
    
    // Get deployed code
    function getDeployedCode(address deployed) public view returns (bytes memory) {
        return deployed.code;
    }
    
    // Check if address has code
    function isContract(address addr) public view returns (bool) {
        return addr.code.length > 0;
    }
}

// Simple test contract to be deployed
contract TestDeployable {
    uint256 public value;
    address public owner;
    
    constructor(uint256 _value) {
        value = _value;
        owner = msg.sender;
    }
    
    function getValue() public view returns (uint256) {
        return value;
    }
    
    function setValue(uint256 _value) public {
        require(msg.sender == owner, "Only owner");
        value = _value;
    }
}
