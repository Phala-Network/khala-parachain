use super::*;

pub mod phala_world_migration {
	use super::*;
	use frame_support::weights::Weight;
	use frame_support::{log, traits::Get};
	use pallet_rmrk_core::Collections;

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

	pub fn migrate<T>() -> frame_support::weights::Weight
	where
		T: pallet_pw_nft_sale::Config
			+ pallet_rmrk_core::Config
			+ pallet_uniques::Config<CollectionId = u32>,
	{
		let mut weight: Weight = 0;
		if let Some(next_nft_id) = get_collection_count::<T>(SPIRIT_COLLECTION_ID) {
			pallet_pw_nft_sale::NextNftId::<T>::insert(SPIRIT_COLLECTION_ID, next_nft_id);
			weight += T::DbWeight::get().reads_writes(1, 1);
			weight
		} else {
			log::error!("NFT count could not be determined");
			T::DbWeight::get().reads(1)
		}
	}
}
