use highway::{HighwayHash, HighwayHasher, Key};
use rand::{thread_rng as rng, Rng};
lazy_static! {
    static ref SEED: [u64; 4] = [rng().gen(), rng().gen(), rng().gen(), rng().gen()];
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct BlockId(String);

impl BlockId {
    pub fn new(data: &[u8]) -> Self {
        // use a seed that is consistent across this run of txtx, but unique across multiple runs
        let key = Key(*SEED);
        let mut hasher = HighwayHasher::new(key);
        hasher.append(data);
        // the result is two 64 bit numbers
        let res = hasher.finalize128();
        // turn each number into a hex string, padded with 0s to 16 chars so we have consistent length
        BlockId(format!("{:016x}-{:016x}", res[0], res[1]))
    }
}

impl core::fmt::Display for BlockId {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[cfg(test)]
mod tests {
    use crate::types::block_id::BlockId;

    #[test]
    fn it_yields_consistent_ids() {
        assert_eq!(BlockId::new("test".as_bytes()), BlockId::new("test".as_bytes()));
        assert_ne!(BlockId::new("test".as_bytes()), BlockId::new("tEsT".as_bytes()));
    }
}
