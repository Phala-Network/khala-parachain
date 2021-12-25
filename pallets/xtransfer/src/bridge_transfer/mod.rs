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
			tokens::{
				fungibles::{Inspect, Transfer as FungibleTransfer},
				BalanceConversion, WithdrawReasons,
			},
			Currency, ExistenceRequirement, OnUnbalanced, StorageVersion,
		},
		transactional,
	};

	use crate::bridge;
	use crate::bridge::pallet::BridgeTransact;
	use crate::xcm::xcm_transfer::pallet::XcmTransact;
	use frame_system::pallet_prelude::*;
	use sp_arithmetic::traits::SaturatedConversion;
	use sp_core::U256;
	use sp_std::prelude::*;
	use xcm::latest::{prelude::*, MultiAsset, MultiLocation};

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

		/// XCM transactor
		type XcmTransactor: XcmTransact<Self>;

		/// PHA resource id
		#[pallet::constant]
		type NativeTokenResourceId: Get<ResourceId>;

		/// The handler to absorb the fee.
		type OnFeePay: OnUnbalanced<NegativeImbalanceOf<Self>>;
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// [dest_id, min_fee, fee_scale]
		FeeUpdated {
			dest_id: bridge::BridgeChainId,
			min_fee: BalanceOf<T>,
			fee_scale: u32,
		},
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
		DestUnrecognised,
		Unimplemented,
	}

	#[pallet::storage]
	#[pallet::getter(fn bridge_fee)]
	pub type BridgeFee<T: Config> =
		StorageMap<_, Twox64Concat, bridge::BridgeChainId, (BalanceOf<T>, u32), ValueQuery>;

	#[pallet::call]
	impl<T: Config> Pallet<T>
	where
		<T as frame_system::Config>::AccountId: From<[u8; 32]> + Into<[u8; 32]>,
		BalanceOf<T>: Into<u128>,
	{
		/// Change extra bridge transfer fee that user should pay
		#[pallet::weight(195_000_000)]
		pub fn update_fee(
			origin: OriginFor<T>,
			min_fee: BalanceOf<T>,
			fee_scale: u32,
			dest_id: bridge::BridgeChainId,
		) -> DispatchResult {
			T::BridgeCommitteeOrigin::ensure_origin(origin)?;
			ensure!(fee_scale <= 1000u32, Error::<T>::InvalidFeeOption);
			BridgeFee::<T>::insert(dest_id, (min_fee, fee_scale));
			Self::deposit_event(Event::FeeUpdated {
				dest_id,
				min_fee,
				fee_scale,
			});
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
			let sender = ensure_signed(origin)?;
			let reserve_id = <bridge::Pallet<T>>::account_id();

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
			let reducible_balance = <pallet_assets::pallet::Pallet<T>>::reducible_balance(
				asset_id.into(),
				&sender,
				false,
			);
			ensure!(
				reducible_balance >= asset_amount,
				Error::<T>::InsufficientBalance
			);

			let rid: bridge::ResourceId = asset
				.try_into()
				.map_err(|_| Error::<T>::AssetConversionFailed)?;
			let fee = Self::calculate_fee(dest_id, amount);
			// check native balance to cover fee
			let native_free_balance = <T as Config>::Currency::free_balance(&sender);
			ensure!(native_free_balance >= fee, Error::<T>::InsufficientBalance);

			// pay fee to treasury
			let imbalance = <T as Config>::Currency::withdraw(
				&sender,
				fee,
				WithdrawReasons::FEE,
				ExistenceRequirement::AllowDeath,
			)?;
			T::OnFeePay::on_unbalanced(imbalance);

			let asset_amount = T::BalanceConverter::to_asset_balance(amount, asset_id)
				.map_err(|_| Error::<T>::BalanceConversionFailed)?;
			// transfer asset from sender to reserve account
			<pallet_assets::pallet::Pallet<T> as FungibleTransfer<T::AccountId>>::transfer(
				asset_id,
				&sender,
				&reserve_id,
				asset_amount,
				false,
			)
			.map_err(|_| Error::<T>::FailedToTransactAsset)?;

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
			let sender = ensure_signed(origin)?;
			let reserve_id = <bridge::Pallet<T>>::account_id();
			ensure!(
				<bridge::Pallet<T>>::chain_whitelisted(dest_id),
				Error::<T>::InvalidDestination
			);
			ensure!(
				BridgeFee::<T>::contains_key(&dest_id),
				Error::<T>::FeeOptionsMissing
			);
			let fee = Self::calculate_fee(dest_id, amount);
			let free_balance = <T as Config>::Currency::free_balance(&sender);
			ensure!(
				free_balance >= (amount + fee),
				Error::<T>::InsufficientBalance
			);

			let imbalance = <T as Config>::Currency::withdraw(
				&sender,
				fee,
				WithdrawReasons::FEE,
				ExistenceRequirement::AllowDeath,
			)?;
			T::OnFeePay::on_unbalanced(imbalance);
			<T as Config>::Currency::transfer(
				&sender,
				&reserve_id,
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
			dest: Vec<u8>,
			amount: BalanceOf<T>,
			rid: ResourceId,
		) -> DispatchResult {
			let reserve_id = T::BridgeOrigin::ensure_origin(origin.clone())?;
			let dest_location: MultiLocation =
				Decode::decode(&mut dest.as_slice()).map_err(|_| Error::<T>::DestUnrecognised)?;

			match (
				dest_location.clone().parents,
				dest_location.clone().interior,
			) {
				// to local account
				(0, X1(AccountId32 { network: _, id })) => {
					if rid == T::NativeTokenResourceId::get() {
						// ERC20 PHA transfer
						<T as Config>::Currency::transfer(
							&reserve_id,
							&id.into(),
							amount,
							ExistenceRequirement::AllowDeath,
						)?;
					} else {
						let xtransfer_asset: XTransferAsset =
							T::AssetsWrapper::from_resource_id(&rid)
								.ok_or(Error::<T>::AssetConversionFailed)?;
						let asset_id: <T as pallet_assets::Config>::AssetId =
							T::AssetsWrapper::id(&xtransfer_asset)
								.ok_or(Error::<T>::AssetNotRegistered)?;
						let asset_amount = T::BalanceConverter::to_asset_balance(amount, asset_id)
							.map_err(|_| Error::<T>::BalanceConversionFailed)?;

						// transfer asset from reserve account to recipient
						<pallet_assets::pallet::Pallet<T> as FungibleTransfer<T::AccountId>>::transfer(
                            asset_id,
							&reserve_id,
                            &id.into(),
                            asset_amount,
							false,
                        )
                        .map_err(|_| Error::<T>::FailedToTransactAsset)?;
					}
				}
				// to relaychain or other parachain, forward it by xcm
				(1, X1(AccountId32 { network: _, id: _ }))
				| (1, X2(Parachain(_), AccountId32 { network: _, id: _ })) => {
					let multi_asset: MultiAsset = if rid == T::NativeTokenResourceId::get() {
						(MultiLocation::here(), amount.into()).into()
					} else {
						let xtransfer_asset: XTransferAsset =
							T::AssetsWrapper::from_resource_id(&rid)
								.ok_or(Error::<T>::AssetConversionFailed)?;
						let asset_location: MultiLocation = xtransfer_asset.clone().into();
						(asset_location, amount.into()).into()
					};

					// Two main tasks of transfer_fungible is:
					// first) withdraw asset from reserve_id, e.g. bridge account here.(BURN OP)
					// second) deposit asset into sovereign account of dest chain.(MINT OP)
					//
					// So if the reserve account does not have enough asset, transaction would fail.
					// When someone transfer assets to EVM account from local chain our other parachains,
					// assets would be deposited into reserve account, in other words, bridge transfer
					// always based on reserve mode.
					T::XcmTransactor::transfer_fungible(
						reserve_id.clone(),
						Junction::AccountId32 {
							network: NetworkId::Any,
							id: reserve_id.into(),
						}
						.into(),
						multi_asset,
						dest_location,
						6000000000u64.into(),
					)?;
				}
				_ => return Err(Error::<T>::DestUnrecognised.into()),
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
