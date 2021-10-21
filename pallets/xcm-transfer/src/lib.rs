#![cfg_attr(not(feature = "std"), no_std)]

pub mod assets;
pub mod xcm_transfer;
pub mod xcm_helper;

// Alias
pub use assets as pallet_xtransfer_assets;
pub use xcm_transfer as pallet_xcm_transfer;

#[cfg(test)]
mod mock;
