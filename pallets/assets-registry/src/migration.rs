#[allow(unused_imports)]
use super::*;

use codec::{Decode, Encode};
use frame_support::{
	ensure,
	traits::{Get, OnRuntimeUpgrade, StorageVersion},
};
use log;
use scale_info::TypeInfo;
use sp_std::vec;
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

fn migrate_asset_registery<T: Config>() -> Result<u64, &'static str> {
	let mut asset_count = 0;
	let _ = IdByLocations::<T>::clear(12, None);

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

impl<T: Config> OnRuntimeUpgrade for AssetsRegistryToV3Location<T>
where
	<T as pallet_assets::Config>::AssetId: From<u32>,
{
	fn on_runtime_upgrade() -> frame_support::weights::Weight {
		log::info!("Start assets-registry migration");
		let mut weight = 0;

		weight += migrate_asset_registery::<T>().unwrap();

		let asset_ids: Vec<u32> = vec![2u32.into(), 3, 4, 5, 8, 10];
		for id in asset_ids {
			log::info!(
				"asset{:?} - info: {:?}",
				id,
				RegistryInfoByIds::<T>::get::<<T as pallet_assets::Config>::AssetId>(id.into())
			);
		}

		// Set new storage version, same as Khala
		StorageVersion::new(3).put::<Pallet<T>>();

		log::info!("Assets registry migration done👏");

		T::DbWeight::get().writes(weight)
	}
}
