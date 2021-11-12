pub use self::pallet::*;

#[allow(unused_variables)]
#[frame_support::pallet]
pub mod pallet {
	use crate::xcm_helper::{ConcrateAsset, NativeAssetFilter};
	use codec::{Decode, Encode};
	use cumulus_primitives_core::ParaId;
	use frame_support::{
		pallet_prelude::*,
		traits::{
			Contains, Currency, ExistenceRequirement::AllowDeath, StorageVersion, WithdrawReasons,
		},
	};
	use frame_system::pallet_prelude::OriginFor;
	use sp_runtime::traits::{SaturatedConversion, Saturating};
	use sp_std::{convert::TryInto, result, vec::Vec};
	use scale_info::TypeInfo;
	use xcm::v1::{
		prelude::*, AssetId::Concrete, Error as XcmError, Fungibility::Fungible, MultiAsset,
		MultiLocation, Result as XcmResult,
	};
	use xcm_executor::{
		traits::{Convert, MatchesFungible, TransactAsset},
		Assets,
	};

	const LOG_TARGET: &str = "xcm-transfer:assets";
	const STORAGE_VERSION: StorageVersion = StorageVersion::new(1);

	pub type XTransferAssetId = [u8; 32];

	type BalanceOf<T> =
		<<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

	type NegativeImbalanceOf<T> = <<T as Config>::Currency as Currency<
		<T as frame_system::Config>::AccountId,
	>>::NegativeImbalance;

	#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, TypeInfo)]
	pub struct AssetInfo {
		pub asset_location: MultiLocation,
		pub asset_identity: Vec<u8>,
		pub asset_id: XTransferAssetId,
	}

	impl TryInto<MultiLocation> for AssetInfo {
		type Error = ();
		fn try_into(self) -> Result<MultiLocation, Self::Error> {
			// TODO: return error if asset comes from a solo chain(e.g. bridge assets)
			Ok(self.asset_location)
		}
	}

	#[pallet::config]
	pub trait Config: frame_system::Config {
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
		type Currency: Currency<Self::AccountId>;
		/// Origin used to administer the pallet
		type XTransferCommitteeOrigin: EnsureOrigin<Self::Origin>;
		type FungibleMatcher: MatchesFungible<BalanceOf<Self>>;
		type AccountIdConverter: Convert<MultiLocation, Self::AccountId>;
		/// ParachainID
		#[pallet::constant]
		type ParachainInfo: Get<ParaId>;
	}

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	#[pallet::storage_version(STORAGE_VERSION)]
	pub struct Pallet<T>(_);

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// [chainId, asset_identity, assetId]
		AssetRegistered(MultiLocation, Vec<u8>, XTransferAssetId),
		/// [who, amount]
		NativeAssetDeposited(T::AccountId, BalanceOf<T>),
		/// [location, who, amount]
		AssetDeposited(MultiLocation, T::AccountId, BalanceOf<T>),
		/// [who, amount]
		NativeAssetWithdrawn(T::AccountId, BalanceOf<T>),
		/// [location, who, amount]
		AssetWithdrawn(MultiLocation, T::AccountId, BalanceOf<T>),
	}

	#[pallet::error]
	pub enum Error<T> {
		/// AssetId already in used
		AssetIdInUsed,
		/// Asset has not registered
		AssetNotRegistered,
		/// Asset not found.
		AssetNotFound,
		/// `MultiLocation` to `AccountId` conversion failed.
		AccountIdConversionFailed,
		/// `u128` amount to currency `Balance` conversion failed.
		AmountToBalanceConversionFailed,
		/// Insufficient  balance
		InsufficientBalance,
	}

	impl<T: Config> From<Error<T>> for XcmError {
		fn from(e: Error<T>) -> Self {
			use XcmError::FailedToTransactAsset;
			match e {
				Error::<T>::AssetIdInUsed => FailedToTransactAsset("AssetIdInUsed"),
				Error::<T>::AssetNotRegistered => XcmError::AssetNotFound,
				Error::<T>::AssetNotFound => XcmError::AssetNotFound,
				Error::<T>::AccountIdConversionFailed => {
					FailedToTransactAsset("AccountIdConversionFailed")
				}
				Error::<T>::AmountToBalanceConversionFailed => {
					FailedToTransactAsset("AmountToBalanceConversionFailed")
				}
				Error::<T>::InsufficientBalance => FailedToTransactAsset("InsufficientBalance"),
				_ => FailedToTransactAsset("Unknown"),
			}
		}
	}

	#[pallet::storage]
	#[pallet::getter(fn xtransfer_assets)]
	pub type AssetLocationToInfo<T: Config> = StorageMap<_, Twox64Concat, MultiLocation, AssetInfo>;

	#[pallet::storage]
	#[pallet::getter(fn assetidentity_to_into)]
	pub type AssetsIdentityToInfo<T: Config> = StorageMap<_, Twox64Concat, Vec<u8>, AssetInfo>;

	#[pallet::storage]
	#[pallet::getter(fn assetid_to_info)]
	pub type AssetIdToInfo<T: Config> = StorageMap<_, Twox64Concat, XTransferAssetId, AssetInfo>;

	#[pallet::storage]
	#[pallet::getter(fn xtransfer_balances)]
	pub type XTransferBalances<T: Config> =
		StorageDoubleMap<_, Twox64Concat, MultiLocation, Twox64Concat, T::AccountId, BalanceOf<T>>;

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Register an asset.
		#[pallet::weight(195_000_000)]
		pub fn register_asset(
			origin: OriginFor<T>,
			asset_identity: Vec<u8>,
			asset_location: MultiLocation,
		) -> DispatchResult {
			T::XTransferCommitteeOrigin::ensure_origin(origin)?;
			// TODO. Proper way to generate an asset id.
			let asset_id = [0; 32];
			ensure!(
				!AssetLocationToInfo::<T>::contains_key(&asset_location),
				Error::<T>::AssetIdInUsed
			);
			ensure!(
				!AssetsIdentityToInfo::<T>::contains_key(&asset_identity),
				Error::<T>::AssetIdInUsed
			);

			let asset_info = AssetInfo {
				asset_location: asset_location.clone(),
				asset_identity: asset_identity.clone(),
				asset_id: asset_id.clone(),
			};

			AssetLocationToInfo::<T>::insert(&asset_location, &asset_info);
			AssetsIdentityToInfo::<T>::insert(&asset_identity, &asset_info);
			AssetIdToInfo::<T>::insert(&asset_id, &asset_info);

			Self::deposit_event(Event::AssetRegistered(
				asset_location,
				asset_identity,
				asset_id,
			));
			Ok(())
		}
	}

	impl<T: Config> Pallet<T> {
		pub fn free_balance(asset: &MultiLocation, who: &T::AccountId) -> BalanceOf<T> {
			XTransferBalances::<T>::get(asset, who).unwrap_or_default()
		}

		/// Deposit specific amount assets into recipient account.
		///
		/// DO NOT guarantee asset was registered
		pub fn do_asset_deposit(
			asset: &MultiLocation,
			recipient: &T::AccountId,
			amount: BalanceOf<T>,
		) -> Result<(), Error<T>> {
			let resolve_balance = XTransferBalances::<T>::get(asset, recipient).unwrap_or_default();
			XTransferBalances::<T>::insert(
				asset,
				recipient,
				resolve_balance.saturating_add(amount),
			);

			Ok(())
		}

		/// Withdraw specific amount assets from sender.
		///
		/// Assets would be withdrawn from the sender.
		///
		/// DO NOT guarantee asset was registered
		/// DO NOT grarantee sender account has enough balance
		pub fn do_asset_withdraw(
			asset: &MultiLocation,
			sender: &T::AccountId,
			amount: BalanceOf<T>,
		) -> Result<(), Error<T>> {
			let resolve_balance = XTransferBalances::<T>::get(asset, sender).unwrap_or_default();
			ensure!(resolve_balance >= amount, Error::<T>::InsufficientBalance);
			XTransferBalances::<T>::insert(asset, sender, resolve_balance.saturating_sub(amount));
			Ok(())
		}
	}

	impl<T: Config> TransactAsset for Pallet<T> {
		fn deposit_asset(what: &MultiAsset, who: &MultiLocation) -> XcmResult {
			log::trace!(
				target: LOG_TARGET,
				"deposit_asset, what: {:?}, who: {:?}.",
				what.clone(),
				who.clone(),
			);
			// Check if we handle this asset.
			let amount: u128 = T::FungibleMatcher::matches_fungible(&what)
				.ok_or(Error::<T>::AssetNotFound)?
				.saturated_into();
			let who = T::AccountIdConverter::convert_ref(who)
				.map_err(|()| Error::<T>::AccountIdConversionFailed)?;
			let balance_amount = amount
				.try_into()
				.map_err(|_| Error::<T>::AmountToBalanceConversionFailed)?;

			if NativeAssetFilter::<T::ParachainInfo>::is_native_asset(&what) {
				let _imbalance = T::Currency::deposit_creating(&who, balance_amount);
				Self::deposit_event(Event::NativeAssetDeposited(who, balance_amount));
			} else {
				Self::do_asset_deposit(
					&ConcrateAsset::id(&what).ok_or(Error::<T>::AssetNotFound)?,
					&who,
					balance_amount,
				)
				.map_err(|e| XcmError::FailedToTransactAsset(e.into()))?;
				Self::deposit_event(Event::AssetDeposited(
					ConcrateAsset::id(&what).unwrap(),
					who,
					balance_amount,
				));
			}
			Ok(())
		}

		fn withdraw_asset(
			what: &MultiAsset,
			who: &MultiLocation,
		) -> result::Result<Assets, XcmError> {
			log::trace!(
				target: LOG_TARGET,
				"withdraw_asset, what: {:?}, who: {:?}.",
				what.clone(),
				who.clone(),
			);
			// Check we handle this asset.
			let amount: u128 = T::FungibleMatcher::matches_fungible(what)
				.ok_or(Error::<T>::AssetNotFound)?
				.saturated_into();
			let who = T::AccountIdConverter::convert_ref(who)
				.map_err(|()| Error::<T>::AccountIdConversionFailed)?;
			let balance_amount = amount
				.try_into()
				.map_err(|_| Error::<T>::AmountToBalanceConversionFailed)?;

			if NativeAssetFilter::<T::ParachainInfo>::is_native_asset(&what) {
				let _imbalance = T::Currency::withdraw(
					&who,
					balance_amount,
					WithdrawReasons::TRANSFER,
					AllowDeath,
				)
				.map_err(|e| XcmError::FailedToTransactAsset(e.into()))?;
				Self::deposit_event(Event::NativeAssetWithdrawn(who, balance_amount));
			} else {
				Self::do_asset_withdraw(
					&ConcrateAsset::id(&what).ok_or(Error::<T>::AssetNotFound)?,
					&who,
					balance_amount,
				)
				.map_err(|e| XcmError::FailedToTransactAsset(e.into()))?;
				Self::deposit_event(Event::AssetWithdrawn(
					ConcrateAsset::id(&what).unwrap(),
					who,
					balance_amount,
				));
			}

			Ok(what.clone().into())
		}
	}

	impl<T: Config> Contains<MultiLocation> for Pallet<T> {
		fn contains(a: &MultiLocation) -> bool {
			log::trace!(
				target: LOG_TARGET,
				"xtransfer_assets check location {:?}.",
				a.clone(),
			);
			if NativeAssetFilter::<T::ParachainInfo>::is_native_asset_id(a) {
				true
			} else {
				AssetLocationToInfo::<T>::contains_key(&a)
			}
		}
	}
}

#[cfg(test)]
mod test {
	use assert_matches::assert_matches;
	use cumulus_primitives_core::ParaId;
	use frame_support::{assert_err, assert_noop, assert_ok, traits::Currency};
	use polkadot_parachain::primitives::{AccountIdConversion, Sibling};
	use sp_runtime::AccountId32;
	use xcm::v1::{
		AssetId::Concrete, Error as XcmError, Fungibility::Fungible, MultiAsset, MultiLocation,
		Result as XcmResult,
	};
	use xcm_simulator::TestExt;

	use super::*;
	use crate::mock::{
		para::Event as ParaEvent, para::Origin, para::Runtime as Test, para_ext, para_take_events,
		XTransferAssets,
	};

	// TODO. Add more test cases.
	#[test]
	fn test_register_asset() {
		para_ext(2004).execute_with(|| {
			assert_ok!(XTransferAssets::register_asset(
				Origin::root(),
				b"PHA-2004".to_vec(),
				MultiLocation::here(),
			));

			let ev = para_take_events();
			let expected_ev: Vec<ParaEvent> =
				[
					Event::AssetRegistered(MultiLocation::here(), b"PHA-2004".to_vec(), [0; 32])
						.into(),
				]
				.to_vec();
			assert_matches!(ev, expected_ev);

			assert_noop!(
				XTransferAssets::register_asset(
					Origin::root(),
					b"PHA-2004".to_vec(),
					MultiLocation::parent(),
				),
				Error::<Test>::AssetIdInUsed
			);

			assert_noop!(
				XTransferAssets::register_asset(
					Origin::root(),
					b"PHA-2005".to_vec(),
					MultiLocation::here(),
				),
				Error::<Test>::AssetIdInUsed
			);
		});
	}
}
