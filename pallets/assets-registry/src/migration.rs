use super::*;

mod assets_registry_v3_migration_common {
	use super::*;
	use frame_support::{ensure, traits::StorageVersion};

	pub const EXPECTED_STORAGE_VERSION: StorageVersion = StorageVersion::new(1);
	pub const FINAL_STORAGE_VERSION: StorageVersion = StorageVersion::new(2);
	pub const KSM_PRICE: u128 = 111;
	pub const KAR_PRICE: u128 = 111;
	pub const BNC_PRICE: u128 = 111;
	pub const ZLK_PRICE: u128 = 111;
	pub const AUSD_PRICE: u128 = 111;
	pub const BSX_PRICE: u128 = 111;

	pub fn migrate_asset_price<T: Config>(
		assets: &[(<T as pallet_assets::Config>::AssetId, u128)],
	) -> frame_support::weights::Weight {
		for (asset_id, execution_price) in assets.iter() {
			let mut reg_info = RegistryInfoByIds::<T>::get(asset_id).unwrap();

			reg_info.execution_price = Some(*execution_price);
			RegistryInfoByIds::<T>::insert(asset_id, &reg_info);
		}

		assets.len() as u64
	}

	pub fn pre_check_price<T: Config>(
		ids: &[<T as pallet_assets::Config>::AssetId],
	) -> Result<(), &'static str> {
		for asset_id in ids.iter() {
			let reg_info =
				RegistryInfoByIds::<T>::get(asset_id).ok_or(Error::<T>::AssetNotRegistered)?;
			ensure!(reg_info.execution_price == None, "Incorrect registry info");
		}
		Ok(())
	}

	pub fn post_check_price<T: Config>(
		assets: &[(<T as pallet_assets::Config>::AssetId, u128)],
	) -> Result<(), &'static str> {
		for (asset_id, price) in assets {
			let reg_info =
				RegistryInfoByIds::<T>::get(asset_id).ok_or(Error::<T>::AssetNotRegistered)?;
			ensure!(
				reg_info.execution_price == Some(*price),
				"Incorrect registry info of KSM"
			);
		}
		Ok(())
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

		let roc_id: <T as pallet_assets::Config>::AssetId = 0u32.into();
		let kar_id: <T as pallet_assets::Config>::AssetId = 1u32.into();
		let bnc_id: <T as pallet_assets::Config>::AssetId = 2u32.into();
		let zlk_id: <T as pallet_assets::Config>::AssetId = 3u32.into();
		let ausd_id: <T as pallet_assets::Config>::AssetId = 4u32.into();
		let bsx_id: <T as pallet_assets::Config>::AssetId = 5u32.into();

		common::pre_check_price::<T>(&[roc_id, kar_id, bnc_id, zlk_id, ausd_id, bsx_id])?;

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

		let roc_id: <T as pallet_assets::Config>::AssetId = 0u32.into();
		let kar_id: <T as pallet_assets::Config>::AssetId = 1u32.into();
		let bnc_id: <T as pallet_assets::Config>::AssetId = 2u32.into();
		let zlk_id: <T as pallet_assets::Config>::AssetId = 3u32.into();
		let ausd_id: <T as pallet_assets::Config>::AssetId = 4u32.into();
		let bsx_id: <T as pallet_assets::Config>::AssetId = 5u32.into();

		common::post_check_price::<T>(&[
			(roc_id, common::KSM_PRICE),
			(kar_id, common::KAR_PRICE),
			(bnc_id, common::BNC_PRICE),
			(zlk_id, common::ZLK_PRICE),
			(ausd_id, common::AUSD_PRICE),
			(bsx_id, common::BSX_PRICE),
		])?;

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
		let ksm_id: <T as pallet_assets::Config>::AssetId = 0u32.into();
		let kar_id: <T as pallet_assets::Config>::AssetId = 1u32.into();
		let bnc_id: <T as pallet_assets::Config>::AssetId = 2u32.into();
		let zlk_id: <T as pallet_assets::Config>::AssetId = 3u32.into();
		let ausd_id: <T as pallet_assets::Config>::AssetId = 4u32.into();

		common::pre_check_price::<T>(&[ksm_id, kar_id, bnc_id, zlk_id, ausd_id])?;

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

			// Update execution prices
			write_count += common::migrate_asset_price::<T>(&[
				(ksm_id, common::KSM_PRICE),
				(kar_id, common::KAR_PRICE),
				(bnc_id, common::BNC_PRICE),
				(zlk_id, common::ZLK_PRICE),
				(ausd_id, common::AUSD_PRICE),
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
		let ksm_id: <T as pallet_assets::Config>::AssetId = 0u32.into();
		let kar_id: <T as pallet_assets::Config>::AssetId = 1u32.into();
		let bnc_id: <T as pallet_assets::Config>::AssetId = 2u32.into();
		let zlk_id: <T as pallet_assets::Config>::AssetId = 3u32.into();
		let ausd_id: <T as pallet_assets::Config>::AssetId = 4u32.into();

		ensure!(
			StorageVersion::get::<Pallet<T>>() == common::FINAL_STORAGE_VERSION,
			"Incorrect AssetRegistry storage version in post migrate"
		);

		common::post_check_price::<T>(&[
			(ksm_id, common::KSM_PRICE),
			(kar_id, common::KAR_PRICE),
			(bnc_id, common::BNC_PRICE),
			(zlk_id, common::ZLK_PRICE),
			(ausd_id, common::AUSD_PRICE),
		])?;

		log::info!("Assets registry post migration check passed👏");

		Ok(())
	}
}
