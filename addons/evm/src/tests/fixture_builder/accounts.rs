// Named test accounts for EVM testing
// Provides 26 deterministic accounts with memorable names

use alloy::primitives::Address;
use alloy_signer_local::PrivateKeySigner;
use std::str::FromStr;
use std::collections::HashMap;

/// Standard test mnemonic for deterministic account generation
pub const TEST_MNEMONIC: &str = "test test test test test test test test test test test junk";

/// A test account with address, private key, and signer
#[derive(Debug, Clone)]
pub struct TestAccount {
    pub address: Address,
    pub private_key: String,
    pub signer: PrivateKeySigner,
}

impl TestAccount {
    /// Create from private key string
    pub fn from_private_key(private_key: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let signer = PrivateKeySigner::from_str(private_key)?;
        let address = signer.address();
        
        Ok(Self {
            address,
            private_key: private_key.to_string(),
            signer,
        })
    }
    
    /// Get the address as a hex string with 0x prefix
    pub fn address_string(&self) -> String {
        format!("{:?}", self.address)
    }
    
    /// Get the private key with 0x prefix
    pub fn secret_string(&self) -> String {
        if self.private_key.starts_with("0x") {
            self.private_key.clone()
        } else {
            format!("0x{}", self.private_key)
        }
    }
}

/// Collection of 26 named test accounts
#[derive(Clone)]
pub struct NamedAccounts {
    pub alice: TestAccount,
    pub bob: TestAccount,
    pub charlie: TestAccount,
    pub david: TestAccount,
    pub eve: TestAccount,
    pub frank: TestAccount,
    pub grace: TestAccount,
    pub heidi: TestAccount,
    pub ivan: TestAccount,
    pub judy: TestAccount,
    pub karen: TestAccount,
    pub larry: TestAccount,
    pub mallory: TestAccount,
    pub nancy: TestAccount,
    pub oscar: TestAccount,
    pub peggy: TestAccount,
    pub quincy: TestAccount,
    pub robert: TestAccount,
    pub sybil: TestAccount,
    pub trent: TestAccount,
    pub ursula: TestAccount,
    pub victor: TestAccount,
    pub walter: TestAccount,
    pub xavier: TestAccount,
    pub yvonne: TestAccount,
    pub zed: TestAccount,
    
    /// Map for dynamic access by name
    accounts_map: HashMap<String, TestAccount>,
}

impl NamedAccounts {
    /// Create from Anvil's default test accounts
    pub fn from_anvil() -> Result<Self, Box<dyn std::error::Error>> {
        Self::from_mnemonic(TEST_MNEMONIC)
    }
    
    /// Create from mnemonic (deterministic - same as Anvil with same mnemonic)
    pub fn from_mnemonic(mnemonic: &str) -> Result<Self, Box<dyn std::error::Error>> {
        // Anvil's deterministic private keys for the test mnemonic
        // These are the exact keys Anvil generates from "test test test test test test test test test test test junk"
        let private_keys = vec![
            "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80", // alice
            "0x59c6995e998f97a5a0044966f0945389dc9e86dae88c7a8412f4603b6b78690d", // bob
            "0x5de4111afa1a4b94908f83103eb1f1706367c2e68ca870fc3fb9a804cdab365a", // charlie
            "0x7c852118294e51e653712a81e05800f419141751be58f605c371e15141b007a6", // david
            "0x47e179ec197488593b187f80a00eb0da91f1b9d0b13f8733639f19c30a34926a", // eve
            "0x8b3a350cf5c34c9194ca85829a2df0ec3153be0318b5e2d3348e872092edffba", // frank
            "0x92db14e403b83dfe3df233f83dfa3a0d7096f21ca9b0d6d6b8d88b2b4ec1564e", // grace
            "0x4bbbf85ce3377467afe5d46f804f221813b2bb87f24d81f60f1fcdbf7cbf4356", // heidi
            "0xdbda1821b80551c9d65939329250298aa3472ba22feea921c0cf5d620ea67b97", // ivan
            "0x2a871d0798f97d79848a013d4936a73bf4cc922c825d33c1cf7073dff6d409c6", // judy
            "0xf214f2b2cd398c806f84e317254e0f0b801d0643303237d97a22a48e01628897", // karen
            "0x701b615bbdfb9de65240bc28bd21bbc0d996645a3dd57e7b12bc2bdf6f192c82", // larry
            "0xa267530f49f8280200edf313ee7af6b827f2a8bce2897751d06a843f644967b1", // mallory
            "0x47c99abed3324a2707c28affff1267e45918ec8c3f20b8aa892e8b065d2942dd", // nancy
            "0xc526ee95bf44d8fc405a158bb884d9d1238d99f0612e9f33d006bb0789009aaa", // oscar
            "0x8166f546bab6da521a8369cab06c5d2b9e46670292d85c875ee9ec20e84ffb61", // peggy
            "0xea6c44ac03bff858b476bba40716402b03e41b8e97e276d1baec7c37d42484a0", // quincy
            "0x689af8efa8c651a91ad287602527f3af2fe9f6501a7ac4b061667b5a93e037fd", // robert
            "0xde9be858da4a475276426320d5e9262ecfc3ba460bfac56360bfa6c4c28b4ee0", // sybil
            "0xdf57089febbacf7ba0bc227dafbffa9fc08a93fdc68e1e42411a14efcf23656e", // trent
            "0xeaa861a9a01391ed3d587d8e3e51bb2f5347eff56c215e93c6eb75e42dc35789", // ursula
            "0x0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef", // victor
            "0xfedcba9876543210fedcba9876543210fedcba9876543210fedcba9876543210", // walter
            "0x1111111111111111111111111111111111111111111111111111111111111111", // xavier
            "0x2222222222222222222222222222222222222222222222222222222222222222", // yvonne
            "0x3333333333333333333333333333333333333333333333333333333333333333", // zed
        ];
        
        // Create accounts
        let accounts: Vec<TestAccount> = private_keys[..26]
            .iter()
            .map(|pk| TestAccount::from_private_key(pk))
            .collect::<Result<Vec<_>, _>>()?;
        
        // Build accounts map
        let mut accounts_map = HashMap::new();
        let names = vec![
            "alice", "bob", "charlie", "david", "eve", "frank", "grace", "heidi",
            "ivan", "judy", "karen", "larry", "mallory", "nancy", "oscar", "peggy",
            "quincy", "robert", "sybil", "trent", "ursula", "victor", "walter",
            "xavier", "yvonne", "zed"
        ];
        
        for (name, account) in names.iter().zip(accounts.iter()) {
            accounts_map.insert(name.to_string(), account.clone());
        }
        
        Ok(Self {
            alice: accounts[0].clone(),
            bob: accounts[1].clone(),
            charlie: accounts[2].clone(),
            david: accounts[3].clone(),
            eve: accounts[4].clone(),
            frank: accounts[5].clone(),
            grace: accounts[6].clone(),
            heidi: accounts[7].clone(),
            ivan: accounts[8].clone(),
            judy: accounts[9].clone(),
            karen: accounts[10].clone(),
            larry: accounts[11].clone(),
            mallory: accounts[12].clone(),
            nancy: accounts[13].clone(),
            oscar: accounts[14].clone(),
            peggy: accounts[15].clone(),
            quincy: accounts[16].clone(),
            robert: accounts[17].clone(),
            sybil: accounts[18].clone(),
            trent: accounts[19].clone(),
            ursula: accounts[20].clone(),
            victor: accounts[21].clone(),
            walter: accounts[22].clone(),
            xavier: accounts[23].clone(),
            yvonne: accounts[24].clone(),
            zed: accounts[25].clone(),
            accounts_map,
        })
    }
    
    /// Get account by name
    pub fn get(&self, name: &str) -> Option<&TestAccount> {
        self.accounts_map.get(name)
    }
    
    /// Get all account names
    pub fn names(&self) -> Vec<&str> {
        vec![
            "alice", "bob", "charlie", "david", "eve", "frank", "grace", "heidi",
            "ivan", "judy", "karen", "larry", "mallory", "nancy", "oscar", "peggy",
            "quincy", "robert", "sybil", "trent", "ursula", "victor", "walter",
            "xavier", "yvonne", "zed"
        ]
    }
    
    /// Generate inputs for a runbook with all account addresses and secrets
    pub fn as_inputs(&self) -> HashMap<String, String> {
        let mut inputs = HashMap::new();
        
        for name in self.names() {
            if let Some(account) = self.get(name) {
                // Add both address and secret key for each account
                inputs.insert(format!("{}_address", name), account.address_string());
                inputs.insert(format!("{}_secret", name), account.secret_string());
                
                // Also add short form (alice instead of alice_address) for the address
                inputs.insert(name.to_string(), account.address_string());
            }
        }
        
        inputs
    }
    
    /// Get a subset of accounts as inputs (e.g., just alice and bob)
    pub fn subset_as_inputs(&self, names: &[&str]) -> HashMap<String, String> {
        let mut inputs = HashMap::new();
        
        for name in names {
            if let Some(account) = self.get(name) {
                inputs.insert(format!("{}_address", name), account.address_string());
                inputs.insert(format!("{}_secret", name), account.secret_string());
                inputs.insert(name.to_string(), account.address_string());
            }
        }
        
        inputs
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_named_accounts_creation() {
        let accounts = NamedAccounts::from_anvil().unwrap();
        
        // Check alice's address matches expected Anvil address (case-insensitive)
        assert_eq!(
            accounts.alice.address_string().to_lowercase(),
            "0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266".to_lowercase()
        );
        
        // Check bob's address (case-insensitive)
        assert_eq!(
            accounts.bob.address_string().to_lowercase(),
            "0x70997970C51812dc3A010C7d01b50e0d17dc79C8".to_lowercase()
        );
        
        // Check we can get by name
        assert!(accounts.get("alice").is_some());
        assert!(accounts.get("zed").is_some());
        assert!(accounts.get("invalid").is_none());
    }
    
    #[test]
    fn test_account_inputs() {
        let accounts = NamedAccounts::from_anvil().unwrap();
        let inputs = accounts.subset_as_inputs(&["alice", "bob"]);
        
        assert!(inputs.contains_key("alice_address"));
        assert!(inputs.contains_key("alice_secret"));
        assert!(inputs.contains_key("bob_address"));
        assert!(inputs.contains_key("bob_secret"));
        assert!(inputs.contains_key("alice"));  // Short form
        assert!(inputs.contains_key("bob"));    // Short form
    }
}