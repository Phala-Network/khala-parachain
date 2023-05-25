//! Phala World Marketplace Pallet

pub use crate::pallet_pw_incubation;
pub use crate::pallet_pw_nft_sale;

pub use pallet_rmrk_core::types::*;
pub use pallet_rmrk_core::Nfts;
pub use pallet_rmrk_market;

use crate::nft_sale::BalanceOf;
pub use crate::traits::MarketPlaceStakingListHook;
pub use frame_support::pallet_prelude::*;
pub use frame_system::pallet_prelude::*;
pub use rmrk_traits::{primitives::*, RoyaltyInfo};
pub use sp_runtime::Permill;

pub use self::pallet::*;

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use rmrk_traits::RoyaltyInfo;

	pub type RoyaltyInfoOf<T> = RoyaltyInfo<<T as frame_system::Config>::AccountId, Permill>;

	/// Configure the pallet by specifying the parameters and types on which it depends.
	#[pallet::config]
	pub trait Config:
		frame_system::Config
		+ pallet_rmrk_core::Config
		+ pallet_rmrk_market::Config
		+ pallet_pw_nft_sale::Config
		+ pallet_pw_incubation::Config
	{
		/// Because this pallet emits events, it depends on the runtime's definition of an event.
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
		type StakingListHook: MarketPlaceStakingListHook<Self::CollectionId, Self::ItemId>;
	}

	#[pallet::pallet]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(_);

	// Pallets use events to inform users when important changes are made.
	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// Marketplace owner is set.
		MarketplaceOwnerSet {
			old_marketplace_owner: Option<T::AccountId>,
			new_marketplace_owner: T::AccountId,
		},
		/// RoyaltyInfo updated for a NFT.
		RoyaltyInfoUpdated {
			collection_id: T::CollectionId,
			nft_id: T::ItemId,
			old_royalty_info: Option<RoyaltyInfoOf<T>>,
			new_royalty_info: RoyaltyInfoOf<T>,
		},
	}

	// Errors inform users that something went wrong.
	#[pallet::error]
	pub enum Error<T> {}

	// Dispatchable functions allows users to interact with the pallet and invoke state changes.
	// These functions materialize as "extrinsics", which are often compared to transactions.
	// Dispatchable functions must be annotated with a weight and must return a DispatchResult.
	#[pallet::call]
	impl<T: Config> Pallet<T>
	where
		T: pallet_uniques::Config<CollectionId = CollectionId, ItemId = NftId>,
	{
		/// Privileged function that can be called by the `Overlord` account to set the
		/// `MarketplaceOwner` in the `pallet_rmrk_market` Storage.
		///
		/// Parameters:
		/// - `origin`: Expected to be called by Overlord admin account.
		/// - `new_marketplace_owner`: New `T::AccountId` of the new martketplace owner.
		#[pallet::call_index(0)]
		#[pallet::weight(0)]
		pub fn set_marketplace_owner(
			origin: OriginFor<T>,
			new_marketplace_owner: T::AccountId,
		) -> DispatchResultWithPostInfo {
			// Ensure Overlord account makes call
			let sender = ensure_signed(origin)?;
			pallet_pw_nft_sale::Pallet::<T>::ensure_overlord(&sender)?;
			let mut old_marketplace_owner = None;
			pallet_rmrk_market::MarketplaceOwner::<T>::mutate(|current_marketplace_owner| {
				old_marketplace_owner = current_marketplace_owner.clone();
				*current_marketplace_owner = Some(new_marketplace_owner.clone())
			});
			// Emit event
			Self::deposit_event(Event::MarketplaceOwnerSet {
				old_marketplace_owner,
				new_marketplace_owner,
			});

			Ok(Pays::No.into())
		}

		/// Privileged function to update the RoyaltyInfo for PhalaWorld NFTs that will be sold in
		/// the marketplace.
		///
		/// Parameters:
		/// - `origin`: Expected to be called by Overlord admin account.
		/// - `royalty_info`: `RoyaltyInfo` to set the NFTs to.
		/// - `collection_id`: `T::CollectionId` of the PhalaWorld NFT collection.
		/// - `nft_ids`: `BoundedVec` NFT IDs to update in a collection bounded by T::IterLimit.
		#[pallet::call_index(1)]
		#[pallet::weight(0)]
		pub fn set_nfts_royalty_info(
			origin: OriginFor<T>,
			royalty_info: RoyaltyInfo<T::AccountId, Permill>,
			collection_id: T::CollectionId,
			nft_ids: BoundedVec<T::ItemId, T::IterLimit>,
		) -> DispatchResultWithPostInfo {
			// Ensure Overlord account makes call
			let sender = ensure_signed(origin)?;
			pallet_pw_nft_sale::Pallet::<T>::ensure_overlord(&sender)?;
			// Iterate through NFT IDs and set new RoyaltyInfo
			for nft_id in nft_ids {
				Self::do_set_nft_royalty_info(royalty_info.clone(), collection_id, nft_id);
			}

			Ok(Pays::No.into())
		}
	}
}

impl<T: Config>
	pallet_rmrk_market::types::MarketplaceHooks<BalanceOf<T>, T::CollectionId, T::ItemId> for Pallet<T>
where
	T: pallet_uniques::Config<CollectionId = CollectionId, ItemId = NftId>,
{
	fn calculate_market_fee(amount: BalanceOf<T>, market_fee: Permill) -> Option<BalanceOf<T>> {
		if market_fee.is_zero() {
			None
		} else {
			let calculated_market_fee = market_fee * amount;
			Some(calculated_market_fee)
		}
	}

	fn calculate_royalty_fee(amount: BalanceOf<T>, royalty_fee: Permill) -> Option<BalanceOf<T>> {
		if royalty_fee.is_zero() {
			None
		} else {
			let calculated_royalty_fee = royalty_fee * amount;
			Some(calculated_royalty_fee)
		}
	}

	fn can_sell_in_marketplace(collection_id: T::CollectionId, nft_id: T::ItemId) -> bool {
		pallet_pw_incubation::ShellCollectionId::<T>::get() == Some(collection_id)
			|| T::StakingListHook::can_list(&collection_id, &nft_id)
	}
}

impl<T: Config> Pallet<T>
where
	T: pallet_uniques::Config<CollectionId = CollectionId, ItemId = NftId>,
{
	/// Helper function to update `RoyaltyInfo` for a NFT in a collection
	///
	/// Parameters:
	/// - `royalty_info`: `RoyaltyInfo` to set the NFT to.
	/// - `collection_id`: `T::CollectionId` of the PhalaWorld NFT collection.
	/// - `nft_id`: `T::ItemId` of the PhalaWorld NFT.
	fn do_set_nft_royalty_info(
		royalty_info: RoyaltyInfo<T::AccountId, Permill>,
		collection_id: T::CollectionId,
		nft_id: T::ItemId,
	) {
		let mut old_royalty_info = None;
		Nfts::<T>::mutate_exists(collection_id, nft_id, |nft_info| match nft_info {
			Some(nft_info) => {
				old_royalty_info = nft_info.royalty.clone();
				nft_info.royalty = Some(royalty_info.clone());
			}
			None => (),
		});
		Self::deposit_event(Event::RoyaltyInfoUpdated {
			collection_id,
			nft_id,
			old_royalty_info,
			new_royalty_info: royalty_info,
		});
	}
}
