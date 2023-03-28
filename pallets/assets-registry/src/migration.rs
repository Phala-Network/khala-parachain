#[allow(unused_imports)]
use super::*;

use codec::{Decode, Encode};
use frame_support::{
	ensure,
	traits::{Get, OnRuntimeUpgrade, StorageVersion},
};
use log;
use scale_info::TypeInfo;
use sp_std::vec::Vec;
use xcm::latest::MultiLocation;
use xcm::v2::MultiLocation as OldMultiLocation;

#[derive(Clone, Decode, Encode, Eq, PartialEq, Ord, PartialOrd, Debug, TypeInfo)]
pub struct OldAssetRegistryInfo {
	pub location: OldMultiLocation,
	pub reserve_location: Option<OldMultiLocation>,
	pub enabled_bridges: Vec<XBridge>,
	pub properties: AssetProperties,
	pub execution_price: Option<u128>,
}

const EXPECTED_STORAGE_VERSION: StorageVersion = StorageVersion::new(2);
const FINAL_STORAGE_VERSION: StorageVersion = StorageVersion::new(3);

fn migrate_asset_registery<T: Config>() -> Result<u64, &'static str> {
	let mut asset_count = 0;
	let _ = IdByLocations::<T>::clear(15, None);

	RegistryInfoByIds::<T>::translate(
		|id: T::AssetId, old_registery_info: OldAssetRegistryInfo| {
			let new_location: MultiLocation = old_registery_info.location.try_into().unwrap();

			// Insert back with new location
			IdByLocations::<T>::insert(new_location, id);

			asset_count += 1;
			Some(AssetRegistryInfo {
				location: new_location,
				reserve_location: old_registery_info
					.reserve_location
					.map(|old_reserve_location| old_reserve_location.try_into().unwrap()),
				enabled_bridges: old_registery_info.enabled_bridges,
				properties: old_registery_info.properties,
				execution_price: old_registery_info.execution_price,
			})
		},
	);

	// Two insert ops & clear
	Ok(asset_count * 2 + 1)
}

pub struct AssetsRegistryToV3Location<T>(sp_std::marker::PhantomData<T>);

impl<T: Config> OnRuntimeUpgrade for AssetsRegistryToV3Location<T> {
	fn on_runtime_upgrade() -> frame_support::weights::Weight {
		if StorageVersion::get::<Pallet<T>>() == EXPECTED_STORAGE_VERSION {
			log::info!("Start assets-registry migration");
			let mut weight = 0;

			weight += migrate_asset_registery::<T>().unwrap();

			// Set new storage version
			StorageVersion::new(3).put::<Pallet<T>>();

			log::info!("Assets registry migration done👏");

			T::DbWeight::get().writes(weight)
		} else {
			T::DbWeight::get().reads(1)
		}
	}

	#[cfg(feature = "try-runtime")]
	fn pre_upgrade() -> Result<Vec<u8>, &'static str> {
		ensure!(
            StorageVersion::get::<Pallet<T>>() == EXPECTED_STORAGE_VERSION,
            "Incorrect AssetRegistry storage version in pre migrate"
        );

		log::info!("Assets registry pre migration check passed👏");

		Ok(Vec::new())
	}

	#[cfg(feature = "try-runtime")]
	fn post_upgrade(_state: Vec<u8>) -> Result<(), &'static str> {
		ensure!(
            StorageVersion::get::<Pallet<T>>() == FINAL_STORAGE_VERSION,
            "Incorrect AssetRegistry storage version in post migrate"
        );

		let mut asset_count = 0;
		for _key in IdByLocations::<T>::iter_keys() {
			asset_count += 1;
		}
		ensure!(
            asset_count == 3 || asset_count == 12 || asset_count == 15,
            "Incorrect asset count"
        );

		log::info!("Assets registry post migration check passed👏");

		Ok(())
	}
}
