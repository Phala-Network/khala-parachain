use super::*;

mod phala_world_migration_common {
	use super::*;
	use frame_support::pallet_prelude::*;
	use frame_support::{log, traits::StorageVersion, BoundedVec};
	use pallet_rmrk_core::Collections;
	use rmrk_traits::{primitives::CollectionId, CollectionInfo};

	pub const ORIGIN_OF_SHELL_COLLECTION_ID: CollectionId = 1;
	pub const ORIGIN_OF_SHELL_METADATA: &str = "ar://ay3H3zOPsU6VqdVpkVzuAMa_tzaKMRB--swj2Crag4I";
	pub const SHELL_COLLECTION_ID: CollectionId = 2;
	pub const SHELL_METADATA: &str = "ar://rR1LW1TI4C5cVgFYBGb1dzd3KpPrHx2x_Rb4rq4Zgfk";

	// Turns a &str into a BoundedVec
	pub fn str_to_bvec<T>(s: &str) -> BoundedVec<u8, T::StringLimit>
	where
		T: pallet_pw_nft_sale::Config
			+ pallet_rmrk_core::Config
			+ pallet_uniques::Config<CollectionId = u32>,
	{
		s.as_bytes().to_vec().try_into().unwrap()
	}

	pub type Versions = (
		// Version for NFT Sale pallet
		StorageVersion,
		// Version for RMRK pallet
		StorageVersion,
		// Version for Uniques pallet
		StorageVersion,
	);

	pub const EXPECTED_KHALA_STORAGE_VERSION: Versions = (
		StorageVersion::new(2),
		StorageVersion::new(2),
		StorageVersion::new(2),
	);

	pub const FINAL_STORAGE_VERSION: Versions = (
		StorageVersion::new(3),
		StorageVersion::new(3),
		StorageVersion::new(3),
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

	pub fn set_collection_metadata<T>(
		collection_id: CollectionId,
		metadata: BoundedVec<u8, T::StringLimit>,
	) -> (u32, u32)
	where
		T: pallet_pw_nft_sale::Config
			+ pallet_rmrk_core::Config
			+ pallet_uniques::Config<CollectionId = u32>,
	{
		let mut read_count = 0;
		let mut write_count = 0;
		Collections::<T>::mutate(collection_id, |col_info| {
			if let Some(col_info) = col_info {
				log::info!(
					"Updating collection id [{}] old metadata [{:?}] to [{:?}]",
					collection_id,
					col_info.metadata,
					metadata
				);
				col_info.metadata = metadata;
				write_count += 1;
			}
			read_count += 1;
		});

		(read_count, write_count)
	}
}

pub mod phala_world_migration_rhala {
	use super::*;
	use common::{
		set_collection_metadata, str_to_bvec, ORIGIN_OF_SHELL_COLLECTION_ID,
		ORIGIN_OF_SHELL_METADATA, SHELL_COLLECTION_ID, SHELL_METADATA,
	};
	use frame_support::traits::StorageVersion;
	use frame_support::{ensure, log, traits::Get};
	use phala_world_migration_common as common;
	use rmrk_traits::primitives::{CollectionId, NftId};

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
			+ pallet_uniques::Config<CollectionId = u32, ItemId = NftId>,
	{
		if common::get_versions::<T>() == common::EXPECTED_KHALA_STORAGE_VERSION {
			log::info!("Start PhalaWorld migration");
			log::info!("Updating Origin of Shell NFT Collection...");
			let (mut expected_reads, mut expected_writes) = set_collection_metadata::<T>(
				ORIGIN_OF_SHELL_COLLECTION_ID,
				str_to_bvec::<T>(ORIGIN_OF_SHELL_METADATA),
			);
			log::info!("Updating Shell NFT Collection...");
			let (reads, writes) =
				set_collection_metadata::<T>(SHELL_COLLECTION_ID, str_to_bvec::<T>(SHELL_METADATA));
			expected_reads += reads;
			expected_writes += writes;
			log::info!("Expected writes: {}", expected_writes);
			// Set new storage version
			StorageVersion::new(3).put::<pallet_pw_nft_sale::Pallet<T>>();
			StorageVersion::new(3).put::<pallet_rmrk_core::Pallet<T>>();
			StorageVersion::new(3).put::<pallet_uniques::Pallet<T>>();

			log::info!("PhalaWorld migration done👏");
			T::DbWeight::get().reads_writes(expected_reads.into(), expected_writes.into())
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

pub mod phala_world_migration_khala {
	use super::*;
	use crate::migration::phala_world_migration_common::str_to_bvec;
	use common::{
		set_collection_metadata, ORIGIN_OF_SHELL_COLLECTION_ID, ORIGIN_OF_SHELL_METADATA,
		SHELL_COLLECTION_ID, SHELL_METADATA,
	};
	use frame_support::traits::StorageVersion;
	use frame_support::{ensure, log, traits::Get};
	use phala_world_migration_common as common;
	use rmrk_traits::primitives::{CollectionId, NftId};

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
			+ pallet_uniques::Config<CollectionId = u32, ItemId = NftId>,
	{
		if common::get_versions::<T>() == common::EXPECTED_KHALA_STORAGE_VERSION {
			log::info!("Start PhalaWorld migration");
			log::info!("Updating Origin of Shell NFT Collection...");
			let (mut expected_reads, mut expected_writes) = set_collection_metadata::<T>(
				ORIGIN_OF_SHELL_COLLECTION_ID,
				str_to_bvec::<T>(ORIGIN_OF_SHELL_METADATA),
			);
			log::info!("Updating Shell NFT Collection...");
			let (reads, writes) =
				set_collection_metadata::<T>(SHELL_COLLECTION_ID, str_to_bvec::<T>(SHELL_METADATA));
			expected_reads += reads;
			expected_writes += writes;
			log::info!("Expected writes: {}", expected_writes);
			// Set new storage version
			StorageVersion::new(3).put::<pallet_pw_nft_sale::Pallet<T>>();
			StorageVersion::new(3).put::<pallet_rmrk_core::Pallet<T>>();
			StorageVersion::new(3).put::<pallet_uniques::Pallet<T>>();

			log::info!("PhalaWorld migration done👏");
			T::DbWeight::get().reads_writes(expected_reads.into(), expected_writes.into())
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
