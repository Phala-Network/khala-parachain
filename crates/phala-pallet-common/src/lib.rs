#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::pallet_prelude::*;
use sp_runtime::{BoundedVec, WeakBoundedVec};
use sp_std::convert::Into;
use xcm::latest::prelude::*;

pub struct WrapSlice(pub &'static [u8]);

impl<T: Get<u32>> Into<BoundedVec<u8, T>> for WrapSlice {
	fn into(self) -> BoundedVec<u8, T> {
		self.0
			.to_vec()
			.try_into()
			.expect("less than length limit; qed")
	}
}

impl<T: Get<u32>> Into<WeakBoundedVec<u8, T>> for WrapSlice {
	fn into(self) -> WeakBoundedVec<u8, T> {
		self.0
			.to_vec()
			.try_into()
			.expect("less than length limit; qed")
	}
}

impl WrapSlice {
	pub fn into_generalkey(self) -> Junction {
		let len = self.0.len();
		assert!(len <= 32);
		GeneralKey {
			length: len as u8,
			data: {
				let mut data = [0u8; 32];
				data[..len].copy_from_slice(self.0);
				data
			},
		}
	}
}
