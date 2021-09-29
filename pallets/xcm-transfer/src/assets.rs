pub use self::pallet::*;

#[allow(unused_variables)]
#[frame_support::pallet]
pub mod pallet {
	use codec::{Decode, Encode};
	use frame_support::{
		fail,
		pallet_prelude::*,
		traits::{
			Contains, Currency, ExistenceRequirement, OnUnbalanced, StorageVersion, WithdrawReasons,
		},
	};
	use frame_system::pallet_prelude::*;
	use sp_runtime::traits::CheckedConversion;
	use sp_std::{convert::TryFrom, vec::Vec};

	use xcm::{
		v1::prelude::*, Version as XcmVersion, VersionedMultiAssets, VersionedMultiLocation,
		VersionedXcm,
	};

	const LOG_TARGET: &str = "xcm-transfer:assets";
	const STORAGE_VERSION: StorageVersion = StorageVersion::new(1);

	pub type PhalaAssetId = [u8; 32];

	type BalanceOf<T> =
		<<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

	type NegativeImbalanceOf<T> = <<T as Config>::Currency as Currency<
		<T as frame_system::Config>::AccountId,
	>>::NegativeImbalance;

	#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug)]
	pub struct AssetInfo {
		pub dest_id: MultiLocation,
		pub asset_identity: Vec<u8>,
		pub is_reserve: bool,
	}

	#[pallet::config]
	pub trait Config: frame_system::Config {
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
		type Currency: Currency<Self::AccountId>;
		/// Origin used to administer the pallet
		type XTransferCommitteeOrigin: EnsureOrigin<Self::Origin>;
	}

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	#[pallet::storage_version(STORAGE_VERSION)]
	pub struct Pallet<T>(_);

	#[pallet::event]
	#[pallet::metadata(BalanceOf<T> = "Balance")]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// [chainId, asset_identity, assetId]
		AssetRegistered(MultiLocation, Vec<u8>, PhalaAssetId),
	}

	#[pallet::error]
	pub enum Error<T> {
		InsufficientBalance,
		AssetIdInUsed,
		AssetNotRegistered,
	}

	#[pallet::storage]
	#[pallet::getter(fn xtransfer_assets)]
	pub type XTransferAssets<T: Config> = StorageMap<_, Blake2_256, PhalaAssetId, AssetInfo>;

	#[pallet::storage]
	#[pallet::getter(fn xtransfer_reserve_assets)]
	pub type XTransferReserveAssets<T: Config> =
		StorageMap<_, Blake2_256, MultiLocation, AssetInfo>;

	#[pallet::storage]
	#[pallet::getter(fn xtransfer_balances)]
	pub type XTransferBalances<T: Config> =
		StorageDoubleMap<_, Blake2_256, PhalaAssetId, Blake2_256, T::AccountId, BalanceOf<T>>;

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Register an asset.
		#[pallet::weight(195_000_000)]
		pub fn register_reserve_asset(
			origin: OriginFor<T>,
			asset_identity: Vec<u8>,
			dest_id: MultiLocation,
		) -> DispatchResult {
			T::XTransferCommitteeOrigin::ensure_origin(origin)?;
			// TODO. Properly way to generate an asset id.
			let asset_id = [0; 32];
			ensure!(
				!XTransferReserveAssets::<T>::contains_key(&dest_id),
				Error::<T>::AssetIdInUsed
			);
			XTransferReserveAssets::<T>::insert(
				&dest_id,
				AssetInfo {
					dest_id: dest_id.clone(),
					asset_identity: asset_identity.clone(),
					is_reserve: true,
				},
			);
			Self::deposit_event(Event::AssetRegistered(dest_id, asset_identity, asset_id));
			Ok(())
		}
	}

	impl<T: Config> Contains<MultiLocation> for Pallet<T> {
		fn contains(a: &MultiLocation) -> bool {
			log::error!(
				target: LOG_TARGET,
				"xtransfer_assets check location {:?}.",
				a.clone(),
            );
            if *a == MultiLocation::here() {
                true
            } else {
                XTransferReserveAssets::<T>::contains_key(&a)
            }
		}
	}
}
