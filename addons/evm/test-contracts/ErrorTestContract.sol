// SPDX-License-Identifier: MIT
pragma solidity ^0.8.19;

/**
 * @title ErrorTestContract
 * @dev Contract to test various error scenarios for better error messages
 */
contract ErrorTestContract {
    
    // Different revert scenarios
    function revertWithReason() public pure {
        revert("This is a custom revert reason");
    }
    
    function revertWithoutReason() public pure {
        revert();
    }
    
    function requireFail() public pure {
        require(false, "Require condition failed");
    }
    
    function assertFail() public pure {
        assert(false);
    }
    
    // Out of gas scenarios
    function infiniteLoop() public pure {
        while(true) {
            // This will run out of gas
        }
    }
    
    function expensiveOperation() public pure returns (uint256) {
        uint256 result = 0;
        for(uint i = 0; i < 1000000; i++) {
            result = result + i * i;
        }
        return result;
    }
    
    // Division by zero
    function divideByZero(uint256 numerator) public pure returns (uint256) {
        uint256 denominator = 0;
        return numerator / denominator;
    }
    
    // Array out of bounds
    function arrayOutOfBounds() public pure returns (uint256) {
        uint256[3] memory arr = [uint256(1), 2, 3];
        return arr[10]; // Out of bounds access
    }
    
    // Stack too deep
    function stackTooDeep() public pure returns (uint256) {
        uint256 a1 = 1;
        uint256 a2 = 2;
        uint256 a3 = 3;
        uint256 a4 = 4;
        uint256 a5 = 5;
        uint256 a6 = 6;
        uint256 a7 = 7;
        uint256 a8 = 8;
        uint256 a9 = 9;
        uint256 a10 = 10;
        uint256 a11 = 11;
        uint256 a12 = 12;
        uint256 a13 = 13;
        uint256 a14 = 14;
        uint256 a15 = 15;
        uint256 a16 = 16;
        
        return a1 + a2 + a3 + a4 + a5 + a6 + a7 + a8 + 
               a9 + a10 + a11 + a12 + a13 + a14 + a15 + a16;
    }
    
    // Invalid opcode
    function invalidOpcode() public pure {
        assembly {
            invalid()
        }
    }
    
    // Different error types with data
    error InsufficientFunds(uint256 requested, uint256 available);
    error UnauthorizedAccess(address caller, address required);
    error InvalidParameter(string paramName, string reason);
    
    function testInsufficientFunds(uint256 amount) public pure {
        uint256 balance = 100;
        if (amount > balance) {
            revert InsufficientFunds(amount, balance);
        }
    }
    
    function testUnauthorizedAccess(address required) public view {
        if (msg.sender != required) {
            revert UnauthorizedAccess(msg.sender, required);
        }
    }
    
    function testInvalidParameter(uint256 value) public pure {
        if (value == 0) {
            revert InvalidParameter("value", "Must be non-zero");
        }
        if (value > 1000) {
            revert InvalidParameter("value", "Exceeds maximum of 1000");
        }
    }
}
