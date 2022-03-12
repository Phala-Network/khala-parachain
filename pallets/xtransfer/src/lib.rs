#![cfg_attr(not(feature = "std"), no_std)]

mod traits;
mod helpers;

// Re-export
pub mod xcm;
pub use xcm as pallet_xcm_transfer;

pub mod chainbridge;
pub use chainbridge as pallet_chainbridge;

pub use traits::*;
pub use helpers::*;

pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
    use codec::{Decode, Encode};
	use frame_support::{
		dispatch::DispatchResult, pallet_prelude::*, traits::StorageVersion, transactional, weights::Weight,
	};
	use frame_system::pallet_prelude::*;
	use scale_info::TypeInfo;
	use sp_runtime::traits::StaticLookup;
	use sp_std::{boxed::Box, convert::From, vec, vec::Vec};
	use xcm::latest::{prelude::*, MultiAsset, MultiLocation};

	const LOG_TARGET: &str = "runtime::xtransfer";
	const STORAGE_VERSION: StorageVersion = StorageVersion::new(0);

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	#[pallet::storage_version(STORAGE_VERSION)]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(_);

    #[pallet::config]
	pub trait Config: frame_system::Config {
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
        /// Configured bridges.
        type Bridge: BridgeTransact;
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
		}

        #[pallet::weight(195_000_000)]
		#[transactional]
        pub fn transfer_generic(origin: OriginFor<T>, data: Box<Vec<u8>>, dest: Box<MultiLocation>, dest_weight: Option<Weight>) -> DispatchResult {
            let sender = ensure_signed(origin)?;
			Self::do_transfer_generic(sender, &data, *dest, dest_weight)?;
        }
    }

    impl<T: Config> Pallet<T>
	where
		T::AccountId: Into<[u8; 32]> + From<[u8; 32]>,
	{
        fn do_transfer(sender: T::AccountId, what: MultiAsset, dest: MultiLocation, dest_weight: Option<Weight>) -> DispatchResult {
            ensure!(T::Bridge::can_deposit_asset(what, who), Error::<T>::UnsupportedDest);
            match (what.fun, what.id) {
                // Fungible assets
                (Fungible(_), Concrete(_)) => {
                    T::Bridge::transfer_fungible(sender.into(), what, dest, dest_weight)?;
                },
                // NonFungible assets
                (NonFungible(_), Concrete(_)) => {
                    T::Bridge::transfer_nonfungible(sender.into(), what, dest, dest_weight)?;
                },
                _ => return Error::<T>::UnknownAsset,
            }
        }

        fn do_transfer_generic(sender: T::AccountId, data: Vec<U8>, dest: MultiLocation, dest_weight: Option<Weight>) -> DispatchResult {
            ensure!(T::Bridge::can_send_data(&data, dest), Error::<T>::UnsupportedDest);
            T::Bridge::transfer_generic(sender.into(), &data, dest, dest_weight)?;
        }
    }

    pub trait OnWithdrawn {
		fn on_withdrawn(what: MultiAsset, who: MultiLocation, memo: Vec<u8>) -> DispatchResult;
	}
	impl<T: Config> XcmOnWithdrawn for Pallet<T> {
		fn on_withdrawn(what: MultiAsset, who: MultiLocation, memo: Vec<u8>) -> DispatchResult {
			Self::deposit_event(Event::Withdrawn { what, who, memo});
			Ok(())
		}
	}

    pub trait OnDeposited {
		fn on_deposited(what: MultiAsset, who: MultiLocation, memo: Vec<u8>) -> DispatchResult;
	}
	impl<T: Config> OnDeposited for Pallet<T> {
		fn on_deposited(what: MultiAsset, who: MultiLocation, memo: Vec<u8>) -> DispatchResult {
			Self::deposit_event(Event::Deposited { what, who, memo});
			Ok(())
		}
	}

    pub trait OnForwarded {
		fn on_forwarded(what: MultiAsset, who: MultiLocation, memo: Vec<u8>) -> DispatchResult;
	}
	impl<T: Config> OnForwarded for Pallet<T> {
		fn on_forwarded(what: MultiAsset, who: MultiLocation, memo: Vec<u8>) -> DispatchResult {
			Self::deposit_event(Event::Forwarded { what, who, memo});
			Ok(())
		}
	}
}
