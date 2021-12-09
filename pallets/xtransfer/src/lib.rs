#![cfg_attr(not(feature = "std"), no_std)]

// Re-export
pub use crate::xcm::{xcm_helper, xcm_transfer as pallet_xcm_transfer};
mod xcm;

pub mod assets_wrapper;

// Alias
pub use assets_wrapper as pallet_assets_wrapper;
