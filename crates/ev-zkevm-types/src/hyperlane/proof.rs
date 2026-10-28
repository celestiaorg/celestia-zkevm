use alloy_primitives::{
    Address, Bytes, FixedBytes, U256, Uint,
    hex::{FromHex, ToHexExt},
};
use alloy_rlp::encode;
use alloy_rpc_types::EIP1186AccountProofResponse;
use alloy_trie::{Nibbles, TrieAccount, proof::verify_proof};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use super::digest_keccak;

pub type MerkleProof = Vec<Vec<u8>>;
pub type StoredValue = Vec<u8>;

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

/// The storage key of the Hyperlane Tree count in the MerkleTreeHook contract (slot 183).
pub const HYPERLANE_MERKLE_TREE_COUNT_KEY: &str = "0x00000000000000000000000000000000000000000000000000000000000000b7";

/// Returns the storage keys of the full Hyperlane Tree state: the 32 branch nodes followed by the count.
pub fn hyperlane_merkle_tree_state_keys() -> Vec<&'static str> {
    let mut keys = HYPERLANE_MERKLE_TREE_KEYS.to_vec();
    keys.push(HYPERLANE_MERKLE_TREE_COUNT_KEY);
    keys
}

/// HyperlaneBranchProof contains a Patricia Trie merkle proof (storage proof) for Hyperlane Tree branch nodes.
#[derive(Serialize, Deserialize)]
pub struct HyperlaneBranchProof {
    pub proof: EIP1186AccountProofResponse,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct HyperlaneBranchProofInputs {
    pub account_proof: MerkleProof,
    pub storage_proofs: Vec<MerkleProof>,
    pub account_value: StoredValue,
    pub storage_values: Vec<StoredValue>,
}

/// Verify a Hyperlane Tree branch node's storage proof against the execution state root.
impl HyperlaneBranchProof {
    pub fn new(proof: EIP1186AccountProofResponse) -> Self {
        Self { proof }
    }

    /// Get the branch node at the given index.
    pub fn get_branch_node(&self, index: usize) -> Result<String> {
        Ok(self
            .proof
            .storage_proof
            .get(index)
            .context("Failed to get branch node")?
            .value
            .to_be_bytes::<32>()
            .encode_hex())
    }

    /// Get the stored account.
    pub fn get_stored_account(&self) -> Result<Vec<u8>> {
        let leaf_node: Vec<Bytes> = alloy_rlp::decode_exact(self.proof.account_proof.last().unwrap())?;
        Ok(leaf_node.last().expect("Failed to get stored account").to_vec())
    }

    /// Get the state root.
    pub fn get_state_root(&self) -> Result<FixedBytes<32>> {
        let account: TrieAccount = alloy_rlp::decode_exact(self.get_stored_account()?).unwrap();
        Ok(account.storage_root)
    }

    /// Verify the branch proof against the execution state root.
    pub fn verify(&self, keys: &[&str], contract: Address, root: &str) -> Result<bool> {
        if verify_proof(
            FixedBytes::from_hex(root).unwrap(),
            Nibbles::unpack(digest_keccak(contract.as_slice())),
            Some(self.get_stored_account()?),
            &self.proof.account_proof,
        )
        .is_err()
        {
            return Ok(false);
        }
        let storage_root = self.get_state_root()?;
        for (key, proof) in keys.iter().zip(self.proof.storage_proof.iter()) {
            let key_bytes = alloy_primitives::hex::decode(key)?;
            let key_nibbles = Nibbles::unpack(digest_keccak(&key_bytes));
            if proof.value == Uint::from(0) {
                if verify_proof(storage_root, key_nibbles, None, &proof.proof).is_err() {
                    println!("Failed to verify exclusion (zero) proof for key: {key}");
                    return Ok(false);
                }
            } else if verify_proof(storage_root, key_nibbles, Some(encode(proof.value)), &proof.proof).is_err() {
                println!("Failed to verify proof for key: {key}");
                return Ok(false);
            }
        }
        Ok(true)
    }

    /// Verify a single branch node against the execution state root.
    pub fn verify_single(&self, key: &str, contract: Address, root: &str) -> Result<bool> {
        if verify_proof(
            FixedBytes::from_hex(root).unwrap(),
            Nibbles::unpack(digest_keccak(contract.as_slice())),
            Some(self.get_stored_account()?),
            &self.proof.account_proof,
        )
        .is_err()
        {
            return Ok(false);
        }
        let account: TrieAccount = alloy_rlp::decode_exact(self.get_stored_account()?)?;
        if verify_proof(
            account.storage_root,
            Nibbles::unpack(digest_keccak(&alloy_primitives::hex::decode(key)?)),
            Some(encode(self.proof.storage_proof.first().unwrap().value)),
            &self.proof.storage_proof.first().unwrap().proof,
        )
        .is_err()
        {
            return Ok(false);
        }
        Ok(true)
    }
}

impl From<HyperlaneBranchProof> for HyperlaneBranchProofInputs {
    fn from(proof: HyperlaneBranchProof) -> Self {
        Self {
            account_proof: proof.proof.account_proof.iter().map(|b| b.to_vec()).collect(),
            account_value: proof.proof.account_proof.last().unwrap().to_vec(),
            storage_proofs: proof
                .proof
                .storage_proof
                .iter()
                .map(|b| b.proof.iter().map(|b| b.to_vec()).collect())
                .collect(),
            storage_values: proof
                .proof
                .storage_proof
                .iter()
                .map(|b| b.value.to_be_bytes::<32>().to_vec())
                .collect(),
        }
    }
}

/// Verify a Hyperlane Tree branch node's storage proof against the execution state root.
impl HyperlaneBranchProofInputs {
    pub fn new(proof: HyperlaneBranchProof) -> Self {
        Self::from(proof)
    }

    /// Get the branch node at the given index.
    pub fn get_branch_node(&self, index: usize) -> String {
        self.storage_values
            .get(index)
            .expect("Failed to get branch node")
            .encode_hex()
    }

    /// Get the stored account.
    pub fn get_stored_account(&self) -> Result<Vec<u8>> {
        let leaf_node: Vec<Bytes> = alloy_rlp::decode_exact(self.account_value.as_slice())?;
        Ok(leaf_node.last().context("Failed to get stored account")?.to_vec())
    }

    /// Get the state root.
    pub fn get_state_root(&self) -> Result<FixedBytes<32>> {
        let account: TrieAccount = alloy_rlp::decode_exact(self.get_stored_account()?).unwrap();
        Ok(account.storage_root)
    }

    /// Verify the branch proof against the execution state root.
    pub fn verify(&self, keys: &[&str], contract: Address, root: &str) -> Result<bool> {
        let proof_vec: Vec<Bytes> = self.account_proof.iter().map(|b| Bytes::from(b.to_vec())).collect();
        if verify_proof(
            FixedBytes::from_hex(root).unwrap(),
            Nibbles::unpack(digest_keccak(contract.as_slice())),
            Some(self.get_stored_account()?),
            &proof_vec,
        )
        .is_err()
        {
            println!("Failed to verify account proof");
            return Ok(false);
        }
        // Every key must have a proof and a value, otherwise zip would silently skip unproven keys.
        if keys.len() != self.storage_proofs.len() || keys.len() != self.storage_values.len() {
            println!("Storage proof count does not match key count");
            return Ok(false);
        }
        let storage_root = self.get_state_root()?;
        for (key, (proof, value)) in keys
            .iter()
            .zip(self.storage_proofs.iter().zip(self.storage_values.iter()))
        {
            let value: [u8; 32] = value.as_slice().try_into().context("storage value must be 32 bytes")?;
            let value = U256::from_be_bytes(value);
            // A zero value is absent from the storage trie and must be proven with an exclusion proof.
            let expected = if value.is_zero() { None } else { Some(encode(value)) };
            if verify_proof(
                storage_root,
                Nibbles::unpack(digest_keccak(&alloy_primitives::hex::decode(key).unwrap())),
                expected,
                &proof.iter().map(|b| Bytes::from(b.to_vec())).collect::<Vec<Bytes>>(),
            )
            .is_err()
            {
                println!("Failed to verify storage proof for key: {key}");
                return Ok(false);
            }
        }
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::HyperlaneBranchProofInputs;

    #[test]
    fn test_get_branch_node_reads_hex_value() {
        let inputs = HyperlaneBranchProofInputs {
            account_proof: vec![],
            storage_proofs: vec![],
            account_value: vec![],
            storage_values: vec![vec![0x12, 0x34, 0xab, 0xcd]],
        };

        assert_eq!(inputs.get_branch_node(0), "1234abcd");
    }
}
