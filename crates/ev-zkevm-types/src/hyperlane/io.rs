use std::str::FromStr;

use alloy_primitives::{Address, U256};
use serde::{Deserialize, Serialize};

use super::{
    merkle::{MerkleTree, TREE_DEPTH},
    proof::{HyperlaneBranchProofInputs, hyperlane_merkle_tree_state_keys},
};

#[derive(Serialize, Deserialize, Debug, Clone)]
/// Inputs for the hyperlane merkle tree circuit.
pub struct HyperlaneTreeInputs {
    pub state_root: String,
    pub contract: String,
    /// Storage proof of the 32 branch nodes followed by the count, see `hyperlane_merkle_tree_state_keys`.
    pub tree_proof: HyperlaneBranchProofInputs,
}

impl HyperlaneTreeInputs {
    pub fn new(state_root: String, contract: String, tree_proof: HyperlaneBranchProofInputs) -> Self {
        Self {
            state_root,
            contract,
            tree_proof,
        }
    }

    /// Verify the tree state against the state root and return the proven tree.
    pub fn verify(&self) -> MerkleTree {
        let verified = self
            .tree_proof
            .verify(
                &hyperlane_merkle_tree_state_keys(),
                Address::from_str(&self.contract).unwrap(),
                &self.state_root,
            )
            .expect("Failed to verify tree proof");
        assert!(verified, "Tree proof does not verify against the state root");

        let branch = std::array::from_fn(|idx| self.tree_proof.get_branch_node(idx));
        let count = U256::from_be_slice(&self.tree_proof.storage_values[TREE_DEPTH]);
        assert!(count <= U256::from(u32::MAX), "Tree count exceeds u32");

        MerkleTree {
            branch,
            count: count.to::<u128>(),
        }
    }
}

/// Version of the `HyperlaneTreeOutputs` layout. Bump it whenever the committed fields change so that
/// celestia-app x/zkism can reject public values produced by any other program version.
pub const HYPERLANE_TREE_OUTPUTS_VERSION: u8 = 1;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
/// Outputs of the hyperlane merkle tree circuit. Bincode encodes this as a fixed 101 bytes:
/// `version (u8) || state_root (32) || merkle_tree_address (32) || tree_root (32) || count (u32 LE)`.
pub struct HyperlaneTreeOutputs {
    /// Always `HYPERLANE_TREE_OUTPUTS_VERSION`.
    pub version: u8,
    pub state_root: [u8; 32],
    pub merkle_tree_address: [u8; 32],
    pub tree_root: [u8; 32],
    pub count: u32,
}

#[cfg(test)]
mod tests {
    use super::{HYPERLANE_TREE_OUTPUTS_VERSION, HyperlaneTreeOutputs};

    #[test]
    fn test_hyperlane_tree_outputs_encoding() {
        let output = HyperlaneTreeOutputs {
            version: HYPERLANE_TREE_OUTPUTS_VERSION,
            state_root: [1u8; 32],
            merkle_tree_address: [2u8; 32],
            tree_root: [3u8; 32],
            count: 0x0102,
        };

        // celestia-app x/zkism decodes exactly this fixed 101 byte layout. The length can never collide with the
        // legacy message id layout (72 + 32 * n bytes) and the version byte rejects any other program version.
        let serialized = bincode::serialize(&output).unwrap();
        assert_eq!(serialized.len(), 101);
        assert_eq!(serialized[0], 1);
        assert_eq!(&serialized[1..33], &[1u8; 32]);
        assert_eq!(&serialized[33..65], &[2u8; 32]);
        assert_eq!(&serialized[65..97], &[3u8; 32]);
        assert_eq!(&serialized[97..], &0x0102u32.to_le_bytes());

        let deserialized: HyperlaneTreeOutputs = bincode::deserialize(&serialized).unwrap();
        assert_eq!(output, deserialized);
    }

    #[test]
    fn test_branch_proof_input_conversion_preserves_storage_values() {
        use alloy_primitives::{Address, U256};
        use alloy_rpc_types::{EIP1186AccountProofResponse, EIP1186StorageProof};

        use crate::hyperlane::proof::HyperlaneBranchProof;

        let proof = HyperlaneBranchProof::new(EIP1186AccountProofResponse {
            address: Address::ZERO,
            account_proof: vec![vec![0xc0].into()],
            balance: U256::ZERO,
            code_hash: Default::default(),
            nonce: 0,
            storage_hash: Default::default(),
            storage_proof: vec![EIP1186StorageProof {
                key: Default::default(),
                value: U256::from(7),
                proof: vec![],
            }],
        });

        let inputs = super::HyperlaneBranchProofInputs::from(proof);

        assert_eq!(inputs.storage_values.len(), 1);
        assert_eq!(inputs.storage_values[0], U256::from(7).to_be_bytes::<32>().to_vec());
    }
}
