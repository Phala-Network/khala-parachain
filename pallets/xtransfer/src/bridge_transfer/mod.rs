#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

pub use self::pallet::*;
#[frame_support::pallet]
pub mod pallet {
	use crate::pallet_assets_wrapper::{XTransferAsset, XTransferAssetInfo};
	use frame_support::{
		pallet_prelude::*,
		traits::{
			tokens::{fungibles::Mutate as FungibleMutate, BalanceConversion, WithdrawReasons},
			Currency, ExistenceRequirement, OnUnbalanced, StorageVersion,
		},
		transactional,
	};

	pub use crate::bridge;
	use frame_system::pallet_prelude::*;
	use sp_arithmetic::traits::SaturatedConversion;
	use sp_core::U256;
	use sp_std::prelude::*;

	type ResourceId = bridge::ResourceId;

	type BalanceOf<T> =
		<<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;
	type NegativeImbalanceOf<T> = <<T as Config>::Currency as Currency<
		<T as frame_system::Config>::AccountId,
	>>::NegativeImbalance;

	const STORAGE_VERSION: StorageVersion = StorageVersion::new(1);

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	#[pallet::storage_version(STORAGE_VERSION)]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: frame_system::Config + bridge::Config + pallet_assets::Config {
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

		/// Assets register wrapper
		type AssetsWrapper: XTransferAssetInfo<<Self as pallet_assets::Config>::AssetId>;

		/// Convert Balance of Currency to AssetId of pallet_assets
		type BalanceConverter: BalanceConversion<
			BalanceOf<Self>,
			<Self as pallet_assets::Config>::AssetId,
			<Self as pallet_assets::Config>::Balance,
		>;

		/// Specifies the origin check provided by the bridge for calls that can only be called by the bridge pallet
		type BridgeOrigin: EnsureOrigin<Self::Origin, Success = Self::AccountId>;

		/// Currency impl
		type Currency: Currency<Self::AccountId>;

		/// PHA resource id
		#[pallet::constant]
		type NativeTokenResourceId: Get<ResourceId>;

		/// The handler to absorb the fee.
		type OnFeePay: OnUnbalanced<NegativeImbalanceOf<Self>>;
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// [chainId, min_fee, fee_scale]
		FeeUpdated(bridge::BridgeChainId, BalanceOf<T>, u32),
	}

	#[pallet::error]
	pub enum Error<T> {
		AssetConversionFailed,
		AssetNotRegistered,
		FeeOptionsMissing,
		InvalidDestination,
		InvalidFeeOption,
		InsufficientBalance,
		BalanceConversionFailed,
		FailedToTransactAsset,
	}

	#[pallet::storage]
	#[pallet::getter(fn bridge_fee)]
	pub type BridgeFee<T: Config> =
		StorageMap<_, Twox64Concat, bridge::BridgeChainId, (BalanceOf<T>, u32), ValueQuery>;

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Change extra bridge transfer fee that user should pay
		#[pallet::weight(195_000_000)]
		pub fn change_fee(
			origin: OriginFor<T>,
			min_fee: BalanceOf<T>,
			fee_scale: u32,
			dest_id: bridge::BridgeChainId,
		) -> DispatchResult {
			T::BridgeCommitteeOrigin::ensure_origin(origin)?;
			ensure!(fee_scale <= 1000u32, Error::<T>::InvalidFeeOption);
			BridgeFee::<T>::insert(dest_id, (min_fee, fee_scale));
			Self::deposit_event(Event::FeeUpdated(dest_id, min_fee, fee_scale));
			Ok(())
		}

		/// Transfer some amount of specific asset to some recipient on a (whitelisted) distination chain.
		#[pallet::weight(195_000_000)]
		#[transactional]
		pub fn transfer_assets(
			origin: OriginFor<T>,
			asset: XTransferAsset,
			dest_id: bridge::BridgeChainId,
			recipient: Vec<u8>,
			amount: BalanceOf<T>,
		) -> DispatchResult {
			let source = ensure_signed(origin)?;
			ensure!(
				<bridge::Pallet<T>>::chain_whitelisted(dest_id),
				Error::<T>::InvalidDestination
			);
			ensure!(
				BridgeFee::<T>::contains_key(&dest_id),
				Error::<T>::FeeOptionsMissing
			);
			let asset_id: <T as pallet_assets::Config>::AssetId =
				T::AssetsWrapper::id(&asset).ok_or(Error::<T>::AssetNotRegistered)?;
			let asset_amount = T::BalanceConverter::to_asset_balance(amount, asset_id)
				.map_err(|_| Error::<T>::BalanceConversionFailed)?;
			ensure!(
				<pallet_assets::pallet::Pallet<T>>::balance(asset_id.into(), &source)
					>= asset_amount,
				Error::<T>::InsufficientBalance
			);

			let rid: bridge::ResourceId = asset
				.try_into()
				.map_err(|_| Error::<T>::AssetConversionFailed)?;
			let fee = Self::calculate_fee(dest_id, amount);
			// check native balance to cover fee
			let native_free_balance = <T as Config>::Currency::free_balance(&source);
			ensure!(native_free_balance >= fee, Error::<T>::InsufficientBalance);

			// pay fee to treasury
			let imbalance = <T as Config>::Currency::withdraw(
				&source,
				fee,
				WithdrawReasons::FEE,
				ExistenceRequirement::AllowDeath,
			)?;
			T::OnFeePay::on_unbalanced(imbalance);

			let asset_amount = T::BalanceConverter::to_asset_balance(amount, asset_id)
				.map_err(|_| Error::<T>::BalanceConversionFailed)?;
			// burn asset from sender
			<pallet_assets::pallet::Pallet<T> as FungibleMutate<T::AccountId>>::burn_from(
				asset_id,
				&source,
				asset_amount,
			)
			.map_err(|e| Error::<T>::FailedToTransactAsset)?;

			<bridge::Pallet<T>>::transfer_fungible(
				dest_id,
				rid,
				recipient,
				U256::from(amount.saturated_into::<u128>()),
			)
		}

		/// Transfers some amount of the native token to some recipient on a (whitelisted) destination chain.
		#[pallet::weight(195_000_000)]
		#[transactional]
		pub fn transfer_native(
			origin: OriginFor<T>,
			amount: BalanceOf<T>,
			recipient: Vec<u8>,
			dest_id: bridge::BridgeChainId,
		) -> DispatchResult {
			let source = ensure_signed(origin)?;
			ensure!(
				<bridge::Pallet<T>>::chain_whitelisted(dest_id),
				Error::<T>::InvalidDestination
			);
			let bridge_id = <bridge::Pallet<T>>::account_id();
			ensure!(
				BridgeFee::<T>::contains_key(&dest_id),
				Error::<T>::FeeOptionsMissing
			);
			let fee = Self::calculate_fee(dest_id, amount);
			let free_balance = <T as Config>::Currency::free_balance(&source);
			ensure!(
				free_balance >= (amount + fee),
				Error::<T>::InsufficientBalance
			);

			let imbalance = <T as Config>::Currency::withdraw(
				&source,
				fee,
				WithdrawReasons::FEE,
				ExistenceRequirement::AllowDeath,
			)?;
			T::OnFeePay::on_unbalanced(imbalance);
			<T as Config>::Currency::transfer(
				&source,
				&bridge_id,
				amount,
				ExistenceRequirement::AllowDeath,
			)?;

			<bridge::Pallet<T>>::transfer_fungible(
				dest_id,
				T::NativeTokenResourceId::get(),
				recipient,
				U256::from(amount.saturated_into::<u128>()),
			)
		}

		//
		// Executable calls. These can be triggered by a bridge transfer initiated on another chain
		//

		/// Executes a simple currency transfer using the bridge account as the source
		#[pallet::weight(195_000_000)]
		pub fn transfer(
			origin: OriginFor<T>,
			to: T::AccountId,
			amount: BalanceOf<T>,
			rid: ResourceId,
		) -> DispatchResult {
			let source = T::BridgeOrigin::ensure_origin(origin)?;

			if rid == T::NativeTokenResourceId::get() {
				// ERC20 PHA transfer
				<T as Config>::Currency::transfer(
					&source,
					&to,
					amount,
					ExistenceRequirement::AllowDeath,
				)?;
			} else {
				let xtransfer_asset: XTransferAsset = rid
					.clone()
					.try_into()
					.map_err(|_| Error::<T>::AssetConversionFailed)?;
				let asset_id: <T as pallet_assets::Config>::AssetId =
					T::AssetsWrapper::id(&xtransfer_asset).ok_or(Error::<T>::AssetNotRegistered)?;
				let asset_amount = T::BalanceConverter::to_asset_balance(amount, asset_id)
					.map_err(|_| Error::<T>::BalanceConversionFailed)?;

				// mint asset into recipient
				<pallet_assets::pallet::Pallet<T> as FungibleMutate<T::AccountId>>::mint_into(
					asset_id,
					&to,
					asset_amount,
				)
				.map_err(|e| Error::<T>::FailedToTransactAsset)?;
			}

			Ok(())
		}
	}

	impl<T: Config> Pallet<T> {
		// TODO.wf: A more proper way to estimate fee
		pub fn calculate_fee(dest_id: bridge::BridgeChainId, amount: BalanceOf<T>) -> BalanceOf<T> {
			let (min_fee, fee_scale) = Self::bridge_fee(dest_id);
			let fee_estimated = amount * fee_scale.into() / 1000u32.into();
			if fee_estimated > min_fee {
				fee_estimated
			} else {
				min_fee
			}
		}
	}
}
