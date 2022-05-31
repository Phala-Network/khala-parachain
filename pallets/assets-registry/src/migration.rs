use super::*;

mod assets_registry_v3_migration_common {
	use super::*;
	use codec::{Decode, Encode};
	use frame_support::traits::StorageVersion;
	use scale_info::TypeInfo;
	use sp_std::vec::Vec;
	use xcm::latest::MultiLocation;

	#[derive(Clone, Decode, Encode, Eq, PartialEq, Ord, PartialOrd, Debug, TypeInfo)]
	pub struct OldAssetRegistryInfo {
		pub location: MultiLocation,
		pub reserve_location: Option<MultiLocation>,
		pub enabled_bridges: Vec<XBridge>,
		pub properties: AssetProperties,
	}

	pub const EXPECTED_STORAGE_VERSION: StorageVersion = StorageVersion::new(1);
	pub const FINAL_STORAGE_VERSION: StorageVersion = StorageVersion::new(2);

	// Calculated according to https://github.com/Phala-Network/khala-parachain/blob/06a0f84815d0666f72c4db65f7f794a854e5ee20/runtime/khala/src/constants.rs#L73
	pub const PHA_PER_SECOND: u128 = 80_000_000_000_000;
	pub const KSM_PRICE: u128 = PHA_PER_SECOND / 600;
	pub const KAR_PRICE: u128 = PHA_PER_SECOND / 8;
	pub const BNC_PRICE: u128 = PHA_PER_SECOND / 4;
	pub const ZLK_PRICE: u128 = (PHA_PER_SECOND / 4) * 1_000_000; // decimals 18
	pub const AUSD_PRICE: u128 = PHA_PER_SECOND / 8;
	pub const BSX_PRICE: u128 = PHA_PER_SECOND;
	pub const MOVR_PRICE: u128 = (PHA_PER_SECOND / 240) * 1_000_000; // decimals 18
	pub const HKO_PRICE: u128 = PHA_PER_SECOND;

	pub fn lookup_price<T: Config>(
		assets: &[(<T as pallet_assets::Config>::AssetId, u128)],
		asset_id: <T as pallet_assets::Config>::AssetId,
	) -> Option<u128> {
		for (id, price) in assets.iter() {
			if *id == asset_id {
				return Some(*price);
			}
		}
		None
	}

	pub fn migrate_asset_price<T: Config>(
		assets: &[(<T as pallet_assets::Config>::AssetId, u128)],
	) -> frame_support::weights::Weight {
		RegistryInfoByIds::<T>::translate(
			|asset_id: <T as pallet_assets::Config>::AssetId, old_info: OldAssetRegistryInfo| {
				if let Some(execution_price) = lookup_price::<T>(assets, asset_id) {
					Some(AssetRegistryInfo {
						location: old_info.location,
						reserve_location: old_info.reserve_location,
						enabled_bridges: old_info.enabled_bridges,
						properties: old_info.properties,
						execution_price: Some(execution_price),
					})
				} else {
					log::error!(
						"Asset register info not found: ${:?}, would be deleted",
						asset_id
					);
					None
				}
			},
		);

		assets.len() as u64
	}
}

pub mod assets_registry_v3_migration_for_rhala {
	use super::*;
	use assets_registry_v3_migration_common as common;
	use frame_support::{
		ensure,
		traits::{Get, StorageVersion},
	};
	use log;

	pub fn pre_migrate<T>() -> Result<(), &'static str>
	where
		T: Config,
		<T as pallet_assets::Config>::AssetId: From<u32>,
	{
		ensure!(
			StorageVersion::get::<Pallet<T>>() == common::EXPECTED_STORAGE_VERSION,
			"Incorrect AssetRegistry storage version in pre migrate"
		);
		log::info!("Assets registry pre migration check passed👏");

		Ok(())
	}

	pub fn migrate<T>() -> frame_support::weights::Weight
	where
		T: Config,
		<T as pallet_assets::Config>::AssetId: From<u32>,
	{
		if StorageVersion::get::<Pallet<T>>() == common::EXPECTED_STORAGE_VERSION {
			log::info!("Start assets-registry migration");
			let mut write_count = 0;
			let roc_id: <T as pallet_assets::Config>::AssetId = 0u32.into();
			let kar_id: <T as pallet_assets::Config>::AssetId = 1u32.into();
			let bnc_id: <T as pallet_assets::Config>::AssetId = 2u32.into();
			let zlk_id: <T as pallet_assets::Config>::AssetId = 3u32.into();
			let ausd_id: <T as pallet_assets::Config>::AssetId = 4u32.into();
			let bsx_id: <T as pallet_assets::Config>::AssetId = 5u32.into();

			// Update execution prices
			write_count += common::migrate_asset_price::<T>(&[
				(roc_id, common::KSM_PRICE),
				(kar_id, common::KAR_PRICE),
				(bnc_id, common::BNC_PRICE),
				(zlk_id, common::ZLK_PRICE),
				(ausd_id, common::AUSD_PRICE),
				(bsx_id, common::BSX_PRICE),
			]);

			// Set new storage version
			StorageVersion::new(2).put::<Pallet<T>>();

			log::info!("Assets registry migration done👏");

			T::DbWeight::get().writes(write_count + 1)
		} else {
			T::DbWeight::get().reads(1)
		}
	}

	pub fn post_migrate<T>() -> Result<(), &'static str>
	where
		T: Config,
		<T as pallet_assets::Config>::AssetId: From<u32>,
	{
		ensure!(
			StorageVersion::get::<Pallet<T>>() == common::FINAL_STORAGE_VERSION,
			"Incorrect AssetRegistry storage version in post migrate"
		);
		log::info!("Assets registry post migration check passed👏");

		Ok(())
	}
}

pub mod assets_registry_v3_migration_for_khala {
	use super::*;
	use assets_registry_v3_migration_common as common;
	use frame_support::{
		ensure,
		traits::{Get, StorageVersion},
	};
	use log;

	pub fn pre_migrate<T>() -> Result<(), &'static str>
	where
		T: Config,
		<T as pallet_assets::Config>::AssetId: From<u32>,
	{
		ensure!(
			StorageVersion::get::<Pallet<T>>() == common::EXPECTED_STORAGE_VERSION,
			"Incorrect AssetRegistry storage version in pre migrate"
		);
		log::info!("Assets registry pre migration check passed👏");

		Ok(())
	}

	pub fn migrate<T>() -> frame_support::weights::Weight
	where
		T: Config,
		<T as pallet_assets::Config>::AssetId: From<u32>,
	{
		if StorageVersion::get::<Pallet<T>>() == common::EXPECTED_STORAGE_VERSION {
			log::info!("Start assets-registry migration");
			let mut write_count = 0;
			let ksm_id: <T as pallet_assets::Config>::AssetId = 0u32.into();
			let kar_id: <T as pallet_assets::Config>::AssetId = 1u32.into();
			let bnc_id: <T as pallet_assets::Config>::AssetId = 2u32.into();
			let zlk_id: <T as pallet_assets::Config>::AssetId = 3u32.into();
			let ausd_id: <T as pallet_assets::Config>::AssetId = 4u32.into();
			let bsx_id: <T as pallet_assets::Config>::AssetId = 5u32.into();
			let movr_id: <T as pallet_assets::Config>::AssetId = 6u32.into();
			let hko_id: <T as pallet_assets::Config>::AssetId = 7u32.into();

			// Update execution prices
			write_count += common::migrate_asset_price::<T>(&[
				(ksm_id, common::KSM_PRICE),
				(kar_id, common::KAR_PRICE),
				(bnc_id, common::BNC_PRICE),
				(zlk_id, common::ZLK_PRICE),
				(ausd_id, common::AUSD_PRICE),
				(bsx_id, common::BSX_PRICE),
				(movr_id, common::MOVR_PRICE),
				(hko_id, common::HKO_PRICE),
			]);

			// Set new storage version
			StorageVersion::new(2).put::<Pallet<T>>();

			log::info!("Assets registry migration done👏");

			T::DbWeight::get().writes(write_count + 1)
		} else {
			T::DbWeight::get().reads(1)
		}
	}

	pub fn post_migrate<T>() -> Result<(), &'static str>
	where
		T: Config,
		<T as pallet_assets::Config>::AssetId: From<u32>,
	{
		ensure!(
			StorageVersion::get::<Pallet<T>>() == common::FINAL_STORAGE_VERSION,
			"Incorrect AssetRegistry storage version in post migrate"
		);
		log::info!("Assets registry post migration check passed👏");

		Ok(())
	}
}
