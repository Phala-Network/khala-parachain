//! Primitives used by the Phala parachain

#![cfg_attr(not(feature = "std"), no_std)]

use sp_runtime::{
    generic,
    traits::{IdentifyAccount, Verify},
    MultiSignature,
};

pub use sp_runtime::OpaqueExtrinsic as UncheckedExtrinsic;

/// Hash algorithm.
pub type Hasher = sp_runtime::traits::BlakeTwo256;
/// Aura consensus authority.
pub type AuraId = sp_consensus_aura::sr25519::AuthorityId;
/// Opaque block header type.
pub type Header = generic::Header<BlockNumber, Hasher>;
/// An index to a block.
pub type BlockNumber = u32;
/// Alias to 512-bit hash when used in the context of a transaction signature on the chain.
pub type Signature = MultiSignature;
/// Some way of identifying an account on the chain. We intentionally make it equivalent
/// to the public key of our transaction signing scheme.
pub type AccountId = <<Signature as Verify>::Signer as IdentifyAccount>::AccountId;
/// Balance of an account.
pub type Balance = u128;
/// Index of a transaction in the chain.
pub type Index = u32;
/// A hash of some data used by the chain.
pub type Hash = sp_core::H256;
/// Digest item type.
pub type DigestItem = generic::DigestItem<Hash>;
