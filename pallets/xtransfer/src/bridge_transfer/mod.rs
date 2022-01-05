#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

pub use self::pallet::*;
#[frame_support::pallet]
pub mod pallet {
	use crate::pallet_assets_wrapper::{
		AccountId32Conversion, Resolve, XTransferAsset, XTransferAssetInfo,
	};
	use frame_support::{
		pallet_prelude::*,
		traits::{
			tokens::{
				fungibles::{Inspect, Mutate as FungibleMutate, Transfer as FungibleTransfer},
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
	use xcm::latest::{prelude::*, MultiLocation};

	type ResourceId = bridge::ResourceId;

	type BalanceOf<T> =
		<<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;
	type NegativeImbalanceOf<T> = <<T as Config>::Currency as Currency<
		<T as frame_system::Config>::AccountId,
	>>::NegativeImbalance;

	const LOG_TARGET: &str = "runtime::bridge-transfer";
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
			let u128_dest_id: u128 = dest_id
				.clone()
				.try_into()
				.expect("Convert from chain id to u128 must be ok; qed.");
			let dest_resolve_location: MultiLocation = (
				0,
				X2(GeneralKey(b"solo".to_vec()), GeneralIndex(u128_dest_id)),
			)
				.into();
			let asset_resolve_location: MultiLocation = asset
				.clone()
				.resolve()
				.ok_or(Error::<T>::AssetConversionFailed)?;

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

			let rid: bridge::ResourceId = asset.clone().into_rid(dest_id);
			// ensure asset is setup for the solo chain
			ensure!(
				Self::rid_to_assetid(&rid).is_ok(),
				Error::<T>::AssetConversionFailed
			);

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

			if asset_resolve_location == dest_resolve_location {
				// burn if transfer back to its resolve location
				<pallet_assets::pallet::Pallet<T> as FungibleMutate<T::AccountId>>::burn_from(
					asset_id,
					&sender,
					asset_amount,
				)
				.map_err(|_| Error::<T>::FailedToTransactAsset)?;
			} else {
				// transfer asset from sender to reserve account
				<pallet_assets::pallet::Pallet<T> as FungibleTransfer<T::AccountId>>::transfer(
					asset_id,
					&sender,
					&dest_resolve_location.into_account().into(),
					asset_amount,
					false,
				)
				.map_err(|_| Error::<T>::FailedToTransactAsset)?;
			}

			// send message to evm chains
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
			let bridge_id = T::BridgeOrigin::ensure_origin(origin.clone())?;
			// for solo chain assets, we encode solo chain id as the first byte of resourceId
			let src_id: u128 = rid[0]
				.try_into()
				.expect("Convert from chain id to u128 must be ok; qed.");
			let src_resolve_location: MultiLocation =
				(0, X2(GeneralKey(b"solo".to_vec()), GeneralIndex(src_id))).into();

			let dest_location: MultiLocation =
				Decode::decode(&mut dest.as_slice()).map_err(|_| Error::<T>::DestUnrecognised)?;
			let dest_resolve_location: MultiLocation = dest_location
				.clone()
				.resolve()
				.ok_or(Error::<T>::DestUnrecognised)?;

			let asset_location: MultiLocation = Self::rid_to_location(&rid)?;
			let asset_resolve_location: MultiLocation = asset_location
				.clone()
				.resolve()
				.ok_or(Error::<T>::AssetConversionFailed)?;

			log::trace!(
				target: LOG_TARGET,
				"Resolve location of assset ${:?}, resolve location of source: {:?}.",
				&asset_resolve_location,
				&src_resolve_location,
			);
			let imbalance = if asset_resolve_location == src_resolve_location {
				// create
				amount
			} else {
				let imbalance = if rid == T::NativeTokenResourceId::get() {
					// ERC20 PHA save resolved assets in bridge account
					let _imbalance = <T as Config>::Currency::withdraw(
						&bridge_id,
						amount,
						WithdrawReasons::TRANSFER,
						ExistenceRequirement::AllowDeath,
					)?;
					amount
				} else {
					let asset_id = Self::rid_to_assetid(&rid)?;
					let asset_amount = T::BalanceConverter::to_asset_balance(amount, asset_id)
						.map_err(|_| Error::<T>::BalanceConversionFailed)?;

					// burn from source resolve account
					<pallet_assets::pallet::Pallet<T> as FungibleMutate<T::AccountId>>::burn_from(
						asset_id,
						&src_resolve_location.into_account().into(),
						asset_amount,
					)
					.map_err(|_| Error::<T>::FailedToTransactAsset)?;
					amount
				};
				log::trace!(
					target: LOG_TARGET,
					"Resolve of asset and src dismatch, burn asset form source resolve location.",
				);
				imbalance
			};

			// settle to dest
			match (
				dest_location.clone().parents,
				dest_location.clone().interior,
			) {
				// to local account
				(0, X1(AccountId32 { network: _, id })) => {
					if rid == T::NativeTokenResourceId::get() {
						// ERC20 PHA transfer
						<T as Config>::Currency::deposit_creating(&id.into(), imbalance);
					} else {
						let asset_id = Self::rid_to_assetid(&rid)?;
						let asset_amount =
							T::BalanceConverter::to_asset_balance(imbalance, asset_id)
								.map_err(|_| Error::<T>::BalanceConversionFailed)?;

						// mint asset into recipient
						<pallet_assets::pallet::Pallet<T> as FungibleMutate<T::AccountId>>::mint_into(
							asset_id,
							&id.into(),
							asset_amount,
						)
						.map_err(|_| Error::<T>::FailedToTransactAsset)?;
					}
				}
				// to relaychain or other parachain, forward it by xcm
				(1, X1(AccountId32 { network: _, id: _ }))
				| (1, X2(Parachain(_), AccountId32 { network: _, id: _ })) => {
					let dest_resolve_account = dest_resolve_location.clone().into_account();
					if asset_resolve_location != dest_resolve_location {
						log::trace!(
							target: LOG_TARGET,
							"Resolve of asset and dest dismatch, deposit asset to dest resolve location.",
						);
						if rid == T::NativeTokenResourceId::get() {
							<T as Config>::Currency::deposit_creating(
								&dest_resolve_account.clone().into(),
								imbalance,
							);
						} else {
							let asset_id = Self::rid_to_assetid(&rid)?;
							let asset_amount =
								T::BalanceConverter::to_asset_balance(imbalance, asset_id)
									.map_err(|_| Error::<T>::BalanceConversionFailed)?;
							// mint asset into dest resolve account
							<pallet_assets::pallet::Pallet<T> as FungibleMutate<T::AccountId>>::mint_into(
								asset_id,
								&dest_resolve_account.clone().into(),
								asset_amount,
							)
							.map_err(|_| Error::<T>::FailedToTransactAsset)?;
						}
					}

					// Two main tasks of transfer_fungible is:
					// first) withdraw asset from reserve_id
					// second) deposit asset into sovereign account of dest chain.(MINT OP)
					//
					// So if the reserve account does not have enough asset, transaction would fail.
					// When someone transfer assets to EVM account from local chain our other parachains,
					// assets would be deposited into reserve account, in other words, bridge transfer
					// always based on reserve mode.
					T::XcmTransactor::transfer_fungible(
						dest_resolve_account.clone().into(),
						Junction::AccountId32 {
							network: NetworkId::Any,
							id: dest_resolve_account,
						}
						.into(),
						(asset_location, amount.into()).into(),
						dest_location,
						6000000000u64.into(),
					)?;
				}
				// to other evm chains
				(
					0,
					X3(
						GeneralKey(_solo_key),
						GeneralIndex(_evm_chain_id),
						GeneralKey(_evm_account),
					),
				) => {
					// TODO
					return Err(Error::<T>::DestUnrecognised.into());
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

		pub fn rid_to_location(rid: &[u8; 32]) -> Result<MultiLocation, DispatchError> {
			let asset_location: MultiLocation = if *rid == T::NativeTokenResourceId::get() {
				MultiLocation::here()
			} else {
				let xtransfer_asset: XTransferAsset = T::AssetsWrapper::from_resource_id(&rid)
					.ok_or(Error::<T>::AssetConversionFailed)?;
				xtransfer_asset.into()
			};
			Ok(asset_location)
		}

		pub fn rid_to_assetid(
			rid: &[u8; 32],
		) -> Result<<T as pallet_assets::Config>::AssetId, DispatchError> {
			// PHA based on pallet_balances, not pallet_assets
			if *rid == T::NativeTokenResourceId::get() {
				return Err(Error::<T>::AssetNotRegistered.into());
			}
			let xtransfer_asset: XTransferAsset = T::AssetsWrapper::from_resource_id(&rid)
				.ok_or(Error::<T>::AssetConversionFailed)?;
			let asset_id: <T as pallet_assets::Config>::AssetId =
				T::AssetsWrapper::id(&xtransfer_asset).ok_or(Error::<T>::AssetNotRegistered)?;
			Ok(asset_id)
		}
	}
}
