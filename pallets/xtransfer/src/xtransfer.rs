pub use self::pallet::*;

#[frame_support::pallet]
pub mod pallet {
	use crate::traits::*;
	use assets_registry::AccountId32Conversion;
	use frame_support::{
		dispatch::DispatchResult, pallet_prelude::*, traits::StorageVersion, transactional,
		weights::Weight,
	};
	use frame_system::pallet_prelude::*;
	use sp_std::{boxed::Box, convert::From, vec::Vec};
	use xcm::latest::{prelude::*, MultiAsset, MultiLocation};

	const STORAGE_VERSION: StorageVersion = StorageVersion::new(0);

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	#[pallet::storage_version(STORAGE_VERSION)]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: frame_system::Config {
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
		type XcmBridge: BridgeTransact;
		type ChainBridge: BridgeTransact;
		// TODO: CelerBridge
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// Assets being withdrawn from somewhere.
		Withdrawn {
			what: MultiAsset,
			who: MultiLocation,
			memo: Vec<u8>,
		},
		/// Assets being deposited to somewhere.
		Deposited {
			what: MultiAsset,
			who: MultiLocation,
			memo: Vec<u8>,
		},
		/// Assets being forwarded to somewhere.
		Forwarded {
			what: MultiAsset,
			who: MultiLocation,
			memo: Vec<u8>,
		},
	}

	#[pallet::error]
	pub enum Error<T> {
		UnknownError,
		UnknownAsset,
		UnsupportedDest,
		UnhandledTransfer,
	}

	#[pallet::call]
	impl<T: Config> Pallet<T>
	where
		T::AccountId: Into<[u8; 32]> + From<[u8; 32]>,
	{
		#[pallet::weight(195_000_000)]
		#[transactional]
		pub fn transfer(
			origin: OriginFor<T>,
			asset: Box<MultiAsset>,
			dest: Box<MultiLocation>,
			dest_weight: Option<Weight>,
		) -> DispatchResult {
			let sender = ensure_signed(origin)?;
			Self::do_transfer(sender, *asset, *dest, dest_weight)?;
			Ok(())
		}

		#[pallet::weight(195_000_000)]
		#[transactional]
		pub fn transfer_generic(
			origin: OriginFor<T>,
			data: Box<Vec<u8>>,
			dest: Box<MultiLocation>,
			dest_weight: Option<Weight>,
		) -> DispatchResult {
			let sender = ensure_signed(origin)?;
			Self::do_transfer_generic(sender, data.to_vec(), *dest, dest_weight)?;
			Ok(())
		}
	}

	impl<T: Config> Pallet<T>
	where
		T::AccountId: Into<[u8; 32]> + From<[u8; 32]>,
	{
		fn do_transfer(
			sender: T::AccountId,
			what: MultiAsset,
			dest: MultiLocation,
			dest_weight: Option<Weight>,
		) -> DispatchResult {
			Self::do_xcm_transfer(sender.clone(), what.clone(), dest.clone(), dest_weight)
				.or_else(|_| Self::do_chainbridge_transfer(sender, what, dest, dest_weight))?;
			Ok(())
		}

		fn do_transfer_generic(
			sender: T::AccountId,
			data: Vec<u8>,
			dest: MultiLocation,
			dest_weight: Option<Weight>,
		) -> DispatchResult {
			Self::do_xcm_transfer_generic(sender.clone(), data.clone(), dest.clone(), dest_weight)
				.or_else(|_| {
					Self::do_chainbridge_transfer_generic(sender, data, dest, dest_weight)
				})?;
			Ok(())
		}

		fn do_xcm_transfer(
			sender: T::AccountId,
			what: MultiAsset,
			dest: MultiLocation,
			dest_weight: Option<Weight>,
		) -> DispatchResult {
			match (&what.fun, &what.id) {
				// Fungible assets
				(Fungible(_), Concrete(_)) => {
					T::XcmBridge::transfer_fungible(sender.into(), what, dest, dest_weight)
						.map_err(|_| Error::<T>::UnknownError)?;
				}
				// NonFungible assets
				(NonFungible(_), Concrete(_)) => {
					T::XcmBridge::transfer_nonfungible(sender.into(), what, dest, dest_weight)
						.map_err(|_| Error::<T>::UnknownError)?;
				}
				_ => return Err(Error::<T>::UnknownAsset.into()),
			}
			Ok(())
		}

		fn do_chainbridge_transfer(
			sender: T::AccountId,
			what: MultiAsset,
			dest: MultiLocation,
			dest_weight: Option<Weight>,
		) -> DispatchResult {
			match (&what.fun, &what.id) {
				// Fungible assets
				(Fungible(_), Concrete(_)) => {
					T::ChainBridge::transfer_fungible(sender.into(), what, dest, dest_weight)
						.map_err(|_| Error::<T>::UnknownError)?;
				}
				// NonFungible assets
				(NonFungible(_), Concrete(_)) => {
					T::ChainBridge::transfer_nonfungible(sender.into(), what, dest, dest_weight)
						.map_err(|_| Error::<T>::UnknownError)?;
				}
				_ => return Err(Error::<T>::UnknownAsset.into()),
			}
			Ok(())
		}

		fn do_xcm_transfer_generic(
			sender: T::AccountId,
			data: Vec<u8>,
			dest: MultiLocation,
			dest_weight: Option<Weight>,
		) -> DispatchResult {
			T::XcmBridge::transfer_generic(sender.into(), &data, dest, dest_weight)?;
			Ok(())
		}

		fn do_chainbridge_transfer_generic(
			sender: T::AccountId,
			data: Vec<u8>,
			dest: MultiLocation,
			dest_weight: Option<Weight>,
		) -> DispatchResult {
			T::ChainBridge::transfer_generic(sender.into(), &data, dest, dest_weight)?;
			Ok(())
		}
	}

	impl<T: Config> OnWithdrawn for Pallet<T> {
		fn on_withdrawn(what: MultiAsset, who: MultiLocation, memo: Vec<u8>) -> DispatchResult {
			Self::deposit_event(Event::Withdrawn { what, who, memo });
			Ok(())
		}
	}

	impl<T: Config> OnDeposited for Pallet<T> {
		fn on_deposited(what: MultiAsset, who: MultiLocation, memo: Vec<u8>) -> DispatchResult {
			Self::deposit_event(Event::Deposited { what, who, memo });
			Ok(())
		}
	}

	impl<T: Config> OnForwarded for Pallet<T>
	where
		<T as frame_system::Config>::AccountId: From<[u8; 32]> + Into<[u8; 32]>,
	{
		fn on_forwarded(what: MultiAsset, who: MultiLocation, memo: Vec<u8>) -> DispatchResult {
			// Every forwarded transfer will deposit asset into temporary account in advance, so here we
			// use it as sender, asset will be withdrawn from this account.
			let temporary_account =
				MultiLocation::new(0, X1(GeneralKey(b"bridge_transfer".to_vec()))).into_account();
			Self::do_transfer(
				temporary_account.into(),
				what.clone(),
				who.clone(),
				6_000_000_000u64.into(),
			)?;
			Self::deposit_event(Event::Forwarded { what, who, memo });
			// TODO: Should we support forward generic message in the future?
			Ok(())
		}
	}
}
