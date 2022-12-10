use super::*;

mod phala_world_migration_common {
	use super::*;
	use codec::{Decode, Encode};
	use frame_support::pallet_prelude::*;
	use frame_support::{
		log,
		traits::StorageVersion,
		BoundedVec,
	};
	use pallet_rmrk_core::Collections;
	use rmrk_traits::{primitives::CollectionId, AccountIdOrCollectionNftTuple, RoyaltyInfo};
	use scale_info::TypeInfo;
	#[cfg(feature = "std")]
	use serde::Serialize;
	use sp_runtime::Permill;

	#[cfg_attr(feature = "std", derive(PartialEq, Eq, Serialize))]
	#[derive(Encode, Decode, Debug, TypeInfo, MaxEncodedLen)]
	pub struct OldNftInfo<AccountId, RoyaltyAmount, BoundedString, CollectionId, NftId> {
		/// The owner of the NFT, can be either an Account or a tuple (CollectionId, NftId)
		pub owner: AccountIdOrCollectionNftTuple<AccountId, CollectionId, NftId>,
		/// Royalty (optional)
		pub royalty: Option<RoyaltyInfo<AccountId, RoyaltyAmount>>,
		/// Arbitrary data about an instance, e.g. IPFS hash
		pub metadata: BoundedString,
		/// Equipped state
		pub equipped: bool,
		/// Pending state (if sent to NFT)
		pub pending: bool,
		/// transferability ( non-transferable is "souldbound" )
		pub transferable: bool,
	}

	pub type OldInstanceInfoOf<T> = OldNftInfo<
		<T as frame_system::Config>::AccountId,
		Permill,
		BoundedVec<u8, <T as pallet_uniques::Config>::StringLimit>,
		<T as pallet_uniques::Config>::CollectionId,
		<T as pallet_uniques::Config>::ItemId,
	>;

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

	pub fn get_next_collection_id<T>() -> (u32, u64)
	where
		T: pallet_pw_nft_sale::Config
			+ pallet_rmrk_core::Config
			+ pallet_uniques::Config<CollectionId = u32>,
	{
		let mut reads: u64 = 0;
		let next_collection_id: u32 = match frame_support::migration::take_storage_value::<
			CollectionId,
		>(b"RmrkCore", b"CollectionIndex", &[])
		{
			Some(next_collection_id) => {
				reads += 1;
				log::info!("NextCollectionId == {}", next_collection_id);
				next_collection_id
			}
			None => {
				log::info!("NextCollectionId NOT detected");
				0
			}
		};

		(next_collection_id, reads)
	}

	pub fn get_old_nft_count_in_collection<T>(next_collection_id: CollectionId) -> u32
	where
		T: pallet_pw_nft_sale::Config
			+ pallet_rmrk_core::Config
			+ pallet_uniques::Config<CollectionId = u32>,
	{
		let mut count = 0;
		let mut collection_id = 0;
		while collection_id < next_collection_id {
			let collection_nft_count = match Collections::<T>::get(collection_id) {
				Some(collection_info_of) => collection_info_of.nfts_count,
				None => 0,
			};
			count += collection_nft_count;
			collection_id += 1;
		}
		count
	}
}

pub mod phala_world_migration_rhala {
	use super::*;
	use common::{get_next_collection_id, get_old_nft_count_in_collection, OldInstanceInfoOf};
	use frame_support::traits::StorageVersion;
	use frame_support::{ensure, log, traits::Get};
	use pallet_rmrk_core::Nfts;
	use phala_world_migration_common as common;
	use rmrk_traits::{
		primitives::{CollectionId, NftId},
		NftInfo,
	};

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

			let (next_collection_id, reads) = get_next_collection_id::<T>();
			let expected_writes = get_old_nft_count_in_collection::<T>(next_collection_id);
			log::info!("Expected writes: {}", expected_writes);
			log::info!("Start translate of Nfts StorageDoubleMap");
			Nfts::<T>::translate(
				|_collection_id: CollectionId,
				 _nft_id: NftId,
				 old_instance_info_of: OldInstanceInfoOf<T>| {
					Some(NftInfo {
						owner: old_instance_info_of.owner,
						royalty: old_instance_info_of.royalty,
						metadata: old_instance_info_of.metadata,
						equipped: None,
						pending: old_instance_info_of.pending,
						transferable: old_instance_info_of.transferable,
					})
				},
			);

			log::info!("Insert {} into NextCollectionId", next_collection_id);
			pallet_pw_nft_sale::NextCollectionId::<T>::put(next_collection_id);
			// Set new storage version
			StorageVersion::new(2).put::<pallet_pw_nft_sale::Pallet<T>>();
			StorageVersion::new(2).put::<pallet_rmrk_core::Pallet<T>>();
			StorageVersion::new(2).put::<pallet_uniques::Pallet<T>>();

			log::info!("PhalaWorld migration done👏");
			T::DbWeight::get().reads_writes(reads, expected_writes.into())
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
	use common::{get_next_collection_id, get_old_nft_count_in_collection, OldInstanceInfoOf};
	use frame_support::traits::StorageVersion;
	use frame_support::{ensure, log, traits::Get};
	use pallet_rmrk_core::Nfts;
	use phala_world_migration_common as common;
	use rmrk_traits::{
		primitives::{CollectionId, NftId},
		NftInfo,
	};

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

			let (next_collection_id, reads) = get_next_collection_id::<T>();
			let expected_writes = get_old_nft_count_in_collection::<T>(next_collection_id);
			log::info!("Expected writes: {}", expected_writes);
			log::info!("Start translate of Nfts StorageDoubleMap");
			Nfts::<T>::translate(
				|_collection_id: CollectionId,
				 _nft_id: NftId,
				 old_instance_info_of: OldInstanceInfoOf<T>| {
					Some(NftInfo {
						owner: old_instance_info_of.owner,
						royalty: old_instance_info_of.royalty,
						metadata: old_instance_info_of.metadata,
						equipped: None,
						pending: old_instance_info_of.pending,
						transferable: old_instance_info_of.transferable,
					})
				},
			);

			log::info!("Insert {} into NextCollectionId", next_collection_id);
			pallet_pw_nft_sale::NextCollectionId::<T>::put(next_collection_id);
			// Set new storage version
			StorageVersion::new(2).put::<pallet_pw_nft_sale::Pallet<T>>();
			StorageVersion::new(2).put::<pallet_rmrk_core::Pallet<T>>();
			StorageVersion::new(2).put::<pallet_uniques::Pallet<T>>();

			log::info!("PhalaWorld migration done👏");
			T::DbWeight::get().reads_writes(reads, expected_writes.into())
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
