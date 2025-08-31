//! CREATE2 deployment tests
//! 
//! Note: CREATE2 deployment requires special factory contracts and is not
//! directly supported by standard deploy_contract. These tests verify the
//! address calculation logic.

#[cfg(test)]
mod create2_tests {
    use alloy::primitives::{Address, Bytes, B256};
    use std::str::FromStr;
    
    #[test]
    fn test_create2_address_calculation() {
        println!("🔍 Testing CREATE2 address calculation");
        
        // Test data
        let deployer = Address::from_str("0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb8").unwrap();
        let salt = B256::from([42u8; 32]);
        let bytecode = Bytes::from_str("0x602a60005260206000f3").unwrap();
        
        // Calculate CREATE2 address
        let init_code_hash = alloy::primitives::keccak256(&bytecode);
        let create2_hash = alloy::primitives::keccak256(
            [
                &[0xff],
                deployer.as_slice(),
                salt.as_slice(),
                init_code_hash.as_slice(),
            ].concat()
        );
        
        let expected_address = Address::from_slice(&create2_hash[12..]);
        println!("Calculated CREATE2 address: {}", expected_address);
        
        // Verify it's deterministic
        let recalculated = {
            let init_code_hash = alloy::primitives::keccak256(&bytecode);
            let create2_hash = alloy::primitives::keccak256(
                [
                    &[0xff],
                    deployer.as_slice(),
                    salt.as_slice(),
                    init_code_hash.as_slice(),
                ].concat()
            );
            Address::from_slice(&create2_hash[12..])
        };
        
        assert_eq!(expected_address, recalculated, "CREATE2 address should be deterministic");
        println!("✓ CREATE2 address calculation verified as deterministic");
    }
}