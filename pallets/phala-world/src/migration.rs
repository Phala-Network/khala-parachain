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
		StorageVersion::new(1),
		StorageVersion::new(1),
		StorageVersion::new(1),
	);

	pub const FINAL_STORAGE_VERSION: Versions = (
		StorageVersion::new(2),
		StorageVersion::new(2),
		StorageVersion::new(2),
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

	pub fn get_next_collection_id<T>() -> (u32, u64)
	where
		T: pallet_pw_nft_sale::Config
			+ pallet_rmrk_core::Config
			+ pallet_uniques::Config<CollectionId = u32>,
	{
		let mut next_collection_id: u32 = 0;
		let mut reads: u64 = 0;
		for collection_id in pallet_rmrk_core::Collections::<T>::iter_keys() {
			if collection_id > next_collection_id {
				next_collection_id = collection_id;
			}
			reads += 1;
		}
		// Add 1 to the next_collection_id
		next_collection_id += 1;
		(next_collection_id, reads)
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

			let (next_collection_id, reads) = get_next_collection_id::<T>();
			log::info!("Insert {} into NextCollectionId", next_collection_id);
			pallet_pw_nft_sale::NextCollectionId::<T>::put(next_collection_id);
			// Set new storage version
			StorageVersion::new(2).put::<pallet_pw_nft_sale::Pallet<T>>();
			StorageVersion::new(2).put::<pallet_rmrk_core::Pallet<T>>();
			StorageVersion::new(2).put::<pallet_uniques::Pallet<T>>();

			log::info!("PhalaWorld migration done👏");
			T::DbWeight::get().reads_writes(reads, 1)
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
