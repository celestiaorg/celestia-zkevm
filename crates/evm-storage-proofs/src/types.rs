use alloy_primitives::{
    Address, Bytes, FixedBytes, Uint,
    hex::{FromHex, ToHexExt},
};
use alloy_rpc_types::EIP1186AccountProofResponse;
use alloy_trie::{Nibbles, TrieAccount, proof::verify_proof};
use serde::{Deserialize, Serialize};

use crate::digest_keccak;

// These are the fixed keys for the Hyperlane Tree branch nodes.
// They index into the MerkleTreeHook contract's storage slots for range 151-182 (inclusive).
// 183 stores the count.
pub const HYPERLANE_MERKLE_TREE_KEYS: [&str; 32] = [
    "0x0000000000000000000000000000000000000000000000000000000000000097",
    "0x0000000000000000000000000000000000000000000000000000000000000098",
    "0x0000000000000000000000000000000000000000000000000000000000000099",
    "0x000000000000000000000000000000000000000000000000000000000000009a",
    "0x000000000000000000000000000000000000000000000000000000000000009b",
    "0x000000000000000000000000000000000000000000000000000000000000009c",
    "0x000000000000000000000000000000000000000000000000000000000000009d",
    "0x000000000000000000000000000000000000000000000000000000000000009e",
    "0x000000000000000000000000000000000000000000000000000000000000009f",
    "0x00000000000000000000000000000000000000000000000000000000000000a0",
    "0x00000000000000000000000000000000000000000000000000000000000000a1",
    "0x00000000000000000000000000000000000000000000000000000000000000a2",
    "0x00000000000000000000000000000000000000000000000000000000000000a3",
    "0x00000000000000000000000000000000000000000000000000000000000000a4",
    "0x00000000000000000000000000000000000000000000000000000000000000a5",
    "0x00000000000000000000000000000000000000000000000000000000000000a6",
    "0x00000000000000000000000000000000000000000000000000000000000000a7",
    "0x00000000000000000000000000000000000000000000000000000000000000a8",
    "0x00000000000000000000000000000000000000000000000000000000000000a9",
    "0x00000000000000000000000000000000000000000000000000000000000000aa",
    "0x00000000000000000000000000000000000000000000000000000000000000ab",
    "0x00000000000000000000000000000000000000000000000000000000000000ac",
    "0x00000000000000000000000000000000000000000000000000000000000000ad",
    "0x00000000000000000000000000000000000000000000000000000000000000ae",
    "0x00000000000000000000000000000000000000000000000000000000000000af",
    "0x00000000000000000000000000000000000000000000000000000000000000b0",
    "0x00000000000000000000000000000000000000000000000000000000000000b1",
    "0x00000000000000000000000000000000000000000000000000000000000000b2",
    "0x00000000000000000000000000000000000000000000000000000000000000b3",
    "0x00000000000000000000000000000000000000000000000000000000000000b4",
    "0x00000000000000000000000000000000000000000000000000000000000000b5",
    "0x00000000000000000000000000000000000000000000000000000000000000b6",
];

/// HyperlaneBranchProof contains a Patricia Trie merkle proof (storage proof) for Hyperlane Tree branch nodes.
#[derive(Serialize, Deserialize)]
pub struct HyperlaneBranchProof {
    pub proof: EIP1186AccountProofResponse,
}

/// Verify a Hyperlane Tree branch node's storage proof against the execution state root.
impl HyperlaneBranchProof {
    pub fn new(proof: EIP1186AccountProofResponse) -> Self {
        Self { proof }
    }

    pub fn get_branch_node(&self, index: usize) -> String {
        self.proof
            .storage_proof
            .get(index)
            .unwrap()
            .value
            .to_be_bytes::<32>()
            .encode_hex()
    }

    pub fn get_stored_account(&self) -> Vec<u8> {
        let leaf_node: Vec<Bytes> = alloy_rlp::decode_exact(self.proof.account_proof.last().unwrap()).unwrap();
        leaf_node.last().unwrap().to_vec()
    }

    pub fn get_state_root(&self) -> FixedBytes<32> {
        let account: TrieAccount = alloy_rlp::decode_exact(self.get_stored_account()).unwrap();
        account.storage_root
    }

    pub fn verify(&self, keys: &[&str], contract: Address, root: String) -> bool {
        // verify the account proof against the execution state root
        match verify_proof(
            FixedBytes::from_hex(root).unwrap(),
            Nibbles::unpack(digest_keccak(&contract.0.0)),
            Some(self.get_stored_account()),
            &self.proof.account_proof,
        ) {
            Ok(_) => {}
            Err(_) => return false,
        }
        let storage_root = self.get_state_root();
        for (key, proof) in keys.iter().zip(self.proof.storage_proof.iter()) {
            // Skip empty branch nodes as those don't have storage proofs
            if proof.value == Uint::from(0) {
                continue;
            }
            match verify_proof(
                storage_root,
                Nibbles::unpack(digest_keccak(&alloy_primitives::hex::decode(key).unwrap())),
                Some(encode(proof.value)),
                &proof.proof,
            ) {
                Ok(_) => {}
                Err(_) => {
                    println!("Failed to verify proof for key: {key}");
                    return false;
                }
            }
        }
        true
    }

    pub fn verify_single(&self, key: &str, contract: Address, root: String) -> bool {
        // verify the account proof against the execution state root
        match verify_proof(
            FixedBytes::from_hex(root).unwrap(),
            Nibbles::unpack(digest_keccak(&contract.0.0)),
            Some(self.get_stored_account()),
            &self.proof.account_proof,
        ) {
            Ok(_) => {}
            Err(_) => return false,
        }
        let account: TrieAccount = alloy_rlp::decode_exact(self.get_stored_account()).unwrap();
        // verify the storage proof against the account root
        match verify_proof(
            account.storage_root,
            Nibbles::unpack(digest_keccak(&alloy_primitives::hex::decode(key).unwrap())),
            Some(encode(self.proof.storage_proof.first().unwrap().value)),
            &self.proof.storage_proof.first().unwrap().proof,
        ) {
            Ok(_) => {}
            Err(_) => return false,
        }
        true
    }
}

fn encode(value: Uint<256, 4>) -> Vec<u8> {
    let raw32 = value.to_be_bytes::<32>();
    let encoded: Vec<u8> = alloy_rlp::encode(raw32.as_slice());
    encoded
}
