use super::*;

mod phala_world_migration_common {
	use super::*;
	use frame_support::traits::StorageVersion;

	pub type Versions = (
		// Version for NFT Sale pallet
		StorageVersion,
		// Version for RMRK pallet
		StorageVersion,
		// Version for Uniques pallet
		StorageVersion,
	);

	pub const EXPECTED_KHALA_STORAGE_VERSION: Versions = (
		StorageVersion::new(0),
		StorageVersion::new(0),
		StorageVersion::new(0),
	);

	pub const FINAL_STORAGE_VERSION: Versions = (
		StorageVersion::new(1),
		StorageVersion::new(1),
		StorageVersion::new(1),
	);

	pub fn get_versions<T>() -> Versions
	where
		T: pallet_pw_nft_sale::Config
			+ pallet_rmrk_core::Config
			+ pallet_uniques::Config<CollectionId = u32>,
	{
		(
			StorageVersion::get::<pallet_pw_nft_sale::Pallet<T>>(),
			StorageVersion::get::<pallet_rmrk_core::Pallet<T>>(),
			StorageVersion::get::<pallet_uniques::Pallet<T>>(),
		)
	}
}

pub mod phala_world_migration {
	use super::*;
	use frame_support::traits::StorageVersion;
	use frame_support::{ensure, log, traits::Get};
	use pallet_rmrk_core::Collections;
	use phala_world_migration_common as common;
	pub const SPIRIT_COLLECTION_ID: u32 = 0;

	pub fn get_collection_count<T>(
		collection_id: <T as pallet_uniques::Config>::CollectionId,
	) -> Option<u32>
	where
		T: pallet_pw_nft_sale::Config
			+ pallet_rmrk_core::Config
			+ pallet_uniques::Config<CollectionId = u32>,
	{
		let collection_info = Collections::<T>::get(collection_id);
		collection_info.map(|info| info.nfts_count)
	}

	pub fn pre_migrate<T>() -> Result<(), &'static str>
	where
		T: pallet_pw_nft_sale::Config
			+ pallet_rmrk_core::Config
			+ pallet_uniques::Config<CollectionId = u32>,
	{
		ensure!(
			common::get_versions::<T>() == common::EXPECTED_KHALA_STORAGE_VERSION,
			"Incorrect PhalaWorld storage version in pre migrate"
		);
		log::info!("PhalaWorld pre migration check passed👏");
		Ok(())
	}

	pub fn migrate<T>() -> frame_support::weights::Weight
	where
		T: pallet_pw_nft_sale::Config
			+ pallet_rmrk_core::Config
			+ pallet_uniques::Config<CollectionId = u32>,
	{
		if common::get_versions::<T>() == common::EXPECTED_KHALA_STORAGE_VERSION {
			log::info!("Start PhalaWorld migration");

			if let Some(next_nft_id) = get_collection_count::<T>(SPIRIT_COLLECTION_ID) {
				pallet_pw_nft_sale::NextNftId::<T>::insert(SPIRIT_COLLECTION_ID, next_nft_id);
				// Set new storage version
				StorageVersion::new(1).put::<pallet_pw_nft_sale::Pallet<T>>();
				StorageVersion::new(1).put::<pallet_rmrk_core::Pallet<T>>();
				StorageVersion::new(1).put::<pallet_uniques::Pallet<T>>();

				log::info!("PhalaWorld migration done👏");
				T::DbWeight::get().reads_writes(1, 5)
			} else {
				log::error!("NFT count could not be determined");
				T::DbWeight::get().reads(1)
			}
		} else {
			T::DbWeight::get().reads(1)
		}
	}

	pub fn post_migrate<T>() -> Result<(), &'static str>
	where
		T: pallet_pw_nft_sale::Config
			+ pallet_rmrk_core::Config
			+ pallet_uniques::Config<CollectionId = u32>,
	{
		ensure!(
			common::get_versions::<T>() == common::FINAL_STORAGE_VERSION,
			"Incorrect PhalaWorld storage version in post migrate"
		);
		log::info!("PhalaWorld post migration check passed👏");

		Ok(())
	}
}
