use super::*;

mod phala_world_migration_common {
	use super::*;
	use crate::nft_sale::Payee;
	use frame_support::pallet_prelude::*;
	use frame_support::{log, traits::StorageVersion};
	use pallet_rmrk_core::{Collections, Nfts};
	use rmrk_traits::primitives::NftId;
	use rmrk_traits::{primitives::CollectionId, RoyaltyInfo};
	use sp_runtime::Permill;

	pub const ROYALTY_FEE: Permill = Permill::from_percent(2);
	pub const ORIGIN_OF_SHELL_COLLECTION_ID: CollectionId = 2;
	pub const ORIGIN_OF_SHELL_PARTS_COLLECTION_ID: CollectionId = 3;

	pub type Versions = (
		// Version for NFT Sale pallet
		StorageVersion,
		// Version for RMRK pallet
		StorageVersion,
		// Version for Uniques pallet
		StorageVersion,
	);

	pub const EXPECTED_KHALA_STORAGE_VERSION: Versions = (
		StorageVersion::new(3),
		StorageVersion::new(3),
		StorageVersion::new(3),
	);

	pub const FINAL_STORAGE_VERSION: Versions = (
		StorageVersion::new(4),
		StorageVersion::new(4),
		StorageVersion::new(4),
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

	pub fn get_nft_count_in_collection<T>(collection_id: CollectionId) -> u32
	where
		T: pallet_pw_nft_sale::Config
			+ pallet_rmrk_core::Config
			+ pallet_uniques::Config<CollectionId = CollectionId>,
	{
		let collection_info_of = Collections::<T>::get(collection_id);
		let nft_collection_count = match collection_info_of {
			Some(collection_info_of) => collection_info_of.nfts_count,
			None => 0,
		};
		nft_collection_count
	}

	pub fn set_royalty_info_for_collection<T>(collection_id: CollectionId) -> (u32, u32)
	where
		T: pallet_pw_nft_sale::Config
			+ pallet_rmrk_core::Config
			+ pallet_uniques::Config<CollectionId = CollectionId, ItemId = NftId>,
	{
		let mut read_count = 0;
		let mut write_count = 0;
		let expected_nft_count = get_nft_count_in_collection::<T>(collection_id);
		if let Some(new_royalty_owner) = Payee::<T>::get() {
			read_count += 1;
			log::info!("Start update of Nfts StorageDoubleMap");
			for nft_id in 0..expected_nft_count - 1 {
				Nfts::<T>::mutate_exists(collection_id, nft_id, |nft_info| match nft_info {
					Some(nft_info) => {
						read_count += 1;
						write_count += 1;
						nft_info.royalty = Some(RoyaltyInfo {
							recipient: new_royalty_owner.clone(),
							amount: ROYALTY_FEE,
						});
						log::info!(
							"Update NftInfo for collection id [{}] nft id [{:?}] for RoyaltyInfo to [{:?}]",
							collection_id,
							nft_id,
							nft_info
						);
					}
					None => {
						read_count += 1;
						log::info!(
							"Missing NftInfo for collection id [{}] nft id [{:?}]",
							collection_id,
							nft_id
						);
					}
				})
			}
			log::info!(
				"Stats: read_count[{}] write_count [{}] expected write count [{}]",
				read_count,
				write_count,
				expected_nft_count
			);
		}

		(read_count, write_count)
	}
}

pub mod phala_world_migration_khala {
	use super::*;
	use crate::migration::phala_world_migration_common::{
		set_royalty_info_for_collection, ORIGIN_OF_SHELL_COLLECTION_ID,
		ORIGIN_OF_SHELL_PARTS_COLLECTION_ID,
	};
	use frame_support::traits::StorageVersion;
	use frame_support::{ensure, log, traits::Get};
	use phala_world_migration_common as common;
	use rmrk_traits::primitives::{CollectionId, NftId};

	pub fn pre_migrate<T>() -> Result<(), &'static str>
	where
		T: pallet_pw_nft_sale::Config
			+ pallet_rmrk_core::Config
			+ pallet_uniques::Config<CollectionId = CollectionId>,
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
			+ pallet_uniques::Config<CollectionId = CollectionId, ItemId = NftId>,
	{
		if common::get_versions::<T>() == common::EXPECTED_KHALA_STORAGE_VERSION {
			log::info!("Start PhalaWorld migration");
			log::info!("Updating Shell NFT Collection...");
			let (mut expected_reads, mut expected_writes) =
				set_royalty_info_for_collection::<T>(ORIGIN_OF_SHELL_COLLECTION_ID);
			log::info!("Shell NFT expected reads: {}", expected_reads);
			log::info!("Shell NFT expected writes: {}", expected_writes);

			log::info!("Updating Shell Parts NFT Collection...");
			let (shell_parts_expected_reads, shell_parts_expected_writes) =
				set_royalty_info_for_collection::<T>(ORIGIN_OF_SHELL_PARTS_COLLECTION_ID);
			log::info!(
				"Shell Parts NFT expected reads: {}",
				shell_parts_expected_reads
			);
			log::info!(
				"Shell Parts NFT expected writes: {}",
				shell_parts_expected_writes
			);

			expected_reads += shell_parts_expected_reads;
			expected_writes += shell_parts_expected_writes;
			log::info!("Total expected reads: {}", expected_reads);
			log::info!("Total expected writes: {}", expected_writes);
			// Set new storage version
			StorageVersion::new(4).put::<pallet_pw_nft_sale::Pallet<T>>();
			StorageVersion::new(4).put::<pallet_rmrk_core::Pallet<T>>();
			StorageVersion::new(4).put::<pallet_uniques::Pallet<T>>();

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
