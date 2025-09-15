//! Implements Hyperlane's incremental merkle tree.
//!
//! This is adapted from the implementation from cw-hyperlane:
//! <https://github.com/many-things/cw-hyperlane/blob/7573576c97fe9ee9a91c3e4557ff5a32bfbcee40/packages/interface/src/types/merkle.rs#L11>

use anyhow::Result;
use sha3::{Digest, Keccak256};

#[allow(unused)]
pub const HASH_LENGTH: usize = 32;
pub const TREE_DEPTH: usize = 32;
pub const MAX_LEAVES: u128 = (2_u128.pow(TREE_DEPTH as u32)) - 1;
pub const ZERO_BYTES: &str = "0000000000000000000000000000000000000000000000000000000000000000";
pub const ZERO_HASHES: [&str; TREE_DEPTH] = [
    "0000000000000000000000000000000000000000000000000000000000000000",
    "ad3228b676f7d3cd4284a5443f17f1962b36e491b30a40b2405849e597ba5fb5",
    "b4c11951957c6f8f642c4af61cd6b24640fec6dc7fc607ee8206a99e92410d30",
    "21ddb9a356815c3fac1026b6dec5df3124afbadb485c9ba5a3e3398a04b7ba85",
    "e58769b32a1beaf1ea27375a44095a0d1fb664ce2dd358e7fcbfb78c26a19344",
    "0eb01ebfc9ed27500cd4dfc979272d1f0913cc9f66540d7e8005811109e1cf2d",
    "887c22bd8750d34016ac3c66b5ff102dacdd73f6b014e710b51e8022af9a1968",
    "ffd70157e48063fc33c97a050f7f640233bf646cc98d9524c6b92bcf3ab56f83",
    "9867cc5f7f196b93bae1e27e6320742445d290f2263827498b54fec539f756af",
    "cefad4e508c098b9a7e1d8feb19955fb02ba9675585078710969d3440f5054e0",
    "f9dc3e7fe016e050eff260334f18a5d4fe391d82092319f5964f2e2eb7c1c3a5",
    "f8b13a49e282f609c317a833fb8d976d11517c571d1221a265d25af778ecf892",
    "3490c6ceeb450aecdc82e28293031d10c7d73bf85e57bf041a97360aa2c5d99c",
    "c1df82d9c4b87413eae2ef048f94b4d3554cea73d92b0f7af96e0271c691e2bb",
    "5c67add7c6caf302256adedf7ab114da0acfe870d449a3a489f781d659e8becc",
    "da7bce9f4e8618b6bd2f4132ce798cdc7a60e7e1460a7299e3c6342a579626d2",
    "2733e50f526ec2fa19a22b31e8ed50f23cd1fdf94c9154ed3a7609a2f1ff981f",
    "e1d3b5c807b281e4683cc6d6315cf95b9ade8641defcb32372f1c126e398ef7a",
    "5a2dce0a8a7f68bb74560f8f71837c2c2ebbcbf7fffb42ae1896f13f7c7479a0",
    "b46a28b6f55540f89444f63de0378e3d121be09e06cc9ded1c20e65876d36aa0",
    "c65e9645644786b620e2dd2ad648ddfcbf4a7e5b1a3a4ecfe7f64667a3f0b7e2",
    "f4418588ed35a2458cffeb39b93d26f18d2ab13bdce6aee58e7b99359ec2dfd9",
    "5a9c16dc00d6ef18b7933a6f8dc65ccb55667138776f7dea101070dc8796e377",
    "4df84f40ae0c8229d0d6069e5c8f39a7c299677a09d367fc7b05e3bc380ee652",
    "cdc72595f74c7b1043d0e1ffbab734648c838dfb0527d971b602bc216c9619ef",
    "0abf5ac974a1ed57f4050aa510dd9c74f508277b39d7973bb2dfccc5eeb0618d",
    "b8cd74046ff337f0a7bf2c8e03e10f642c1886798d71806ab1e888d9e5ee87d0",
    "838c5655cb21c6cb83313b5a631175dff4963772cce9108188b34ac87c81c41e",
    "662ee4dd2dd7b2bc707961b1e646c4047669dcb6584f0d8d770daf5d7e7deb2e",
    "388ab20e2573d171a88108e79d820e98f26c0b84aa8b2f4aa4968dbb818ea322",
    "93237c50ba75ee485f4c22adf2f741400bdf8d6a9cc7df7ecae576221665d735",
    "8448818bb4ae4562849e949e17ac16e0be16688e156b5cf15e098c627c0056a9",
];

/// Hash two strings together using Keccak256
pub fn keccak256_concat(left: &str, right: &str) -> Result<String> {
    let left_bytes = hex::decode(left)?;
    let right_bytes = hex::decode(right)?;
    let mut hasher = Keccak256::new();
    hasher.update(&left_bytes);
    hasher.update(&right_bytes);
    let result = hasher.finalize();
    Ok(hex::encode(result))
}

#[derive(serde::Serialize, serde::Deserialize, Debug, PartialEq, Clone)]
/// Incremental Merkle Tree implementation identical to the implementation from cosmwasm-hyperlane:
/// https://github.com/hyperlane-xyz/cosmwasm/blob/main/packages/interface/src/types/merkle.rs
pub struct MerkleTree {
    pub branch: [String; TREE_DEPTH],
    pub count: u128,
}

impl Default for MerkleTree {
    fn default() -> Self {
        Self {
            branch: std::array::from_fn(|_| ZERO_BYTES.to_string()),
            count: Default::default(),
        }
    }
}

impl MerkleTree {
    /// Insert a new node into the tree.
    //  https://github.com/hyperlane-xyz/cosmwasm/blob/c4485e00c89c2d57315503955946bf7f155e7a47/packages/interface/src/types/merkle.rs#L68
    pub fn insert(&mut self, node: String) -> Result<()> {
        assert!(self.count < MAX_LEAVES, "Tree is full");
        self.count += 1;

        let mut node = node;
        let mut size = self.count;
        for (i, next) in self.branch.iter().enumerate() {
            if (size & 1) == 1 {
                self.branch[i] = node;
                return Ok(());
            }
            node = keccak256_concat(next, &node)?;
            size /= 2;
        }
        panic!("unreachable code")
    }

    /// Get the root of the tree.
    pub fn root_with_ctx(&self, zeroes: &[String; TREE_DEPTH]) -> Result<String> {
        let idx = self.count;
        let mut current = MerkleTree::zero_bytes();

        for (i, zero) in zeroes.iter().enumerate() {
            let ith_bit = (idx >> i) & 1;
            let next = self.branch[i].clone();
            if ith_bit == 1 {
                current = keccak256_concat(&next, &current)?;
            } else {
                current = keccak256_concat(&current, zero)?;
            }
        }

        Ok(current)
    }

    /// Get the root of the tree at a specific index.
    pub fn branch_root(mut item: String, branch: &[String; TREE_DEPTH], idx: u128) -> Result<String> {
        for (i, next) in branch.iter().enumerate() {
            item = match (idx >> i) & 1 {
                1 => keccak256_concat(next, &item)?,
                _ => keccak256_concat(&item, next)?,
            }
        }
        Ok(item)
    }

    /// Get the zero hash.
    pub fn zero_bytes() -> String {
        ZERO_BYTES.to_string()
    }

    /// Get the zero hashes.
    pub fn zero_hashes() -> [String; TREE_DEPTH] {
        ZERO_HASHES.map(|s| s.to_string())
    }
}

#[cfg(test)]
mod tests {
    use alloy_primitives::Keccak256;
    use anyhow::Result;

    use crate::tree::{MerkleTree, ZERO_HASHES};
    #[test]
    fn test_insert() {
        let mut tree = MerkleTree::default();
        let message = [
            3, 0, 0, 0, 0, 0, 0, 4, 210, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 167, 87, 133, 81, 186, 232, 154, 150, 195,
            54, 91, 147, 73, 58, 210, 212, 235, 203, 174, 151, 0, 1, 15, 44, 114, 111, 117, 116, 101, 114, 95, 97, 112,
            112, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            106, 128, 155, 54, 202, 240, 212, 106, 147, 94, 231, 104, 53, 6, 94, 197, 168, 179, 206, 167, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 3, 232,
        ];
        tree.insert(keccak256_hash(&message).unwrap()).unwrap();
        let root = tree.root_with_ctx(&ZERO_HASHES.map(|s| s.to_string())).unwrap();
        assert_eq!(root, "fa252f08612271b1aeff37a319dd0dcee621cd5d52b75b974dbac4062e56a0cc");
        /// Hash bytes using Keccak256
        fn keccak256_hash(data: &[u8]) -> Result<String> {
            let mut hasher = Keccak256::new();
            hasher.update(data);
            let result = hasher.finalize();
            Ok(hex::encode(result))
        }
    }
}
