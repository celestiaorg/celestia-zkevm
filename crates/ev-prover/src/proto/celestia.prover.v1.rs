// @generated
/// BlockProof represents a zero-knowledge proof for a single Celestia block.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct BlockProof {
    /// celestia_height is the Celestia block height this proof corresponds to.
    #[prost(uint64, tag="1")]
    pub celestia_height: u64,
    /// proof_data contains the serialized zero-knowledge proof bytes.
    #[prost(bytes="vec", tag="2")]
    pub proof_data: ::prost::alloc::vec::Vec<u8>,
    /// public_values contains the public inputs used for proof verification.
    #[prost(bytes="vec", tag="3")]
    pub public_values: ::prost::alloc::vec::Vec<u8>,
    /// created_at is the Unix timestamp (in seconds) when this proof was generated.
    #[prost(uint64, tag="4")]
    pub created_at: u64,
}
/// RangeProof represents an aggregated zero-knowledge proof covering multiple
/// Celestia blocks within a height range.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct RangeProof {
    /// start_height is the starting Celestia block height (inclusive).
    #[prost(uint64, tag="1")]
    pub start_height: u64,
    /// end_height is the ending Celestia block height (inclusive).
    #[prost(uint64, tag="2")]
    pub end_height: u64,
    /// proof_data contains the serialized aggregated zero-knowledge proof bytes.
    #[prost(bytes="vec", tag="3")]
    pub proof_data: ::prost::alloc::vec::Vec<u8>,
    /// public_values contains the public inputs used for proof verification.
    #[prost(bytes="vec", tag="4")]
    pub public_values: ::prost::alloc::vec::Vec<u8>,
    /// created_at is the Unix timestamp (in seconds) when this proof was generated.
    #[prost(uint64, tag="5")]
    pub created_at: u64,
}
/// MembershipProof represents a zero-knowledge proof for state membership
/// verification.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct MembershipProof {
    /// proof_data contains the serialized zero-knowledge proof bytes.
    #[prost(bytes="vec", tag="1")]
    pub proof_data: ::prost::alloc::vec::Vec<u8>,
    /// public_values contains the public inputs used for proof verification.
    #[prost(bytes="vec", tag="2")]
    pub public_values: ::prost::alloc::vec::Vec<u8>,
    /// created_at is the Unix timestamp (in seconds) when this proof was generated.
    #[prost(uint64, tag="3")]
    pub created_at: u64,
}
/// GetBlockProofRequest is the request type for GetBlockProof.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetBlockProofRequest {
    /// celestia_height is the Celestia block height for which to retrieve the proof.
    #[prost(uint64, tag="1")]
    pub celestia_height: u64,
}
/// GetBlockProofResponse is the response type for GetBlockProof.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetBlockProofResponse {
    /// proof is the block proof for the requested height.
    #[prost(message, optional, tag="1")]
    pub proof: ::core::option::Option<BlockProof>,
}
/// GetBlockProofsInRangeRequest is the request type for GetBlockProofsInRange.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetBlockProofsInRangeRequest {
    /// start_height is the starting Celestia block height (inclusive).
    #[prost(uint64, tag="1")]
    pub start_height: u64,
    /// end_height is the ending Celestia block height (inclusive).
    #[prost(uint64, tag="2")]
    pub end_height: u64,
}
/// GetBlockProofsInRangeResponse is the response type for GetBlockProofsInRange.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetBlockProofsInRangeResponse {
    /// proofs is the list of block proofs within the requested range.
    #[prost(message, repeated, tag="1")]
    pub proofs: ::prost::alloc::vec::Vec<BlockProof>,
}
/// GetLatestBlockProofRequest is the request type for GetLatestBlockProof.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetLatestBlockProofRequest {
}
/// GetLatestBlockProofResponse is the response type for GetLatestBlockProof.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetLatestBlockProofResponse {
    /// proof is the most recently generated block proof.
    #[prost(message, optional, tag="1")]
    pub proof: ::core::option::Option<BlockProof>,
}
/// GetMembershipProofRequest is the request type for GetMembershipProof.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetMembershipProofRequest {
    /// height is the block height for which to retrieve the membership proof.
    #[prost(uint64, tag="1")]
    pub height: u64,
}
/// GetMembershipProofResponse is the response type for GetMembershipProof.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetMembershipProofResponse {
    /// proof is the membership proof for the requested height.
    #[prost(message, optional, tag="1")]
    pub proof: ::core::option::Option<MembershipProof>,
}
/// GetLatestMembershipProofRequest is the request type for
/// GetLatestMembershipProof.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetLatestMembershipProofRequest {
}
/// GetLatestMembershipProofResponse is the response type for
/// GetLatestMembershipProof.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetLatestMembershipProofResponse {
    /// proof is the most recently generated membership proof.
    #[prost(message, optional, tag="1")]
    pub proof: ::core::option::Option<MembershipProof>,
}
/// GetRangeProofsRequest is the request type for GetRangeProofs.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetRangeProofsRequest {
    /// start_height is the starting Celestia block height (inclusive).
    #[prost(uint64, tag="1")]
    pub start_height: u64,
    /// end_height is the ending Celestia block height (inclusive).
    #[prost(uint64, tag="2")]
    pub end_height: u64,
}
/// GetRangeProofsResponse is the response type for GetRangeProofs.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetRangeProofsResponse {
    /// proofs is the list of aggregated range proofs covering the requested range.
    #[prost(message, repeated, tag="1")]
    pub proofs: ::prost::alloc::vec::Vec<RangeProof>,
}
include!("celestia.prover.v1.tonic.rs");
// @@protoc_insertion_point(module)