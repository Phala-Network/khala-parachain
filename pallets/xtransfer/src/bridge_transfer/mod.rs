#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

pub use self::pallet::*;
#[frame_support::pallet]
pub mod pallet {
	use crate::pallet_assets_wrapper::{
		AccountId32Conversion, ExtractReserveLocation, GetAssetRegistryInfo, XTransferAsset,
		CB_ASSET_KEY,
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
	use crate::xcm_helper::NativeAssetChecker;
	use frame_system::pallet_prelude::*;
	use sp_arithmetic::traits::SaturatedConversion;
	use sp_core::U256;
	use sp_std::prelude::*;
	use xcm::latest::{
		prelude::*, AssetId as XcmAssetId, Fungibility::Fungible, MultiAsset, MultiLocation,
	};

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
		type AssetsWrapper: GetAssetRegistryInfo<<Self as pallet_assets::Config>::AssetId>;

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

		/// The handler to absorb the fee.
		type OnFeePay: OnUnbalanced<NegativeImbalanceOf<Self>>;

		/// Check whether an asset is PHA
		type NativeChecker: NativeAssetChecker;

		/// Execution price in PHA
		type NativeExecutionPrice: Get<u128>;

		/// Execution price information
		type ExecutionPriceInfo: Get<Vec<(XcmAssetId, u128)>>;
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
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
		DestUnrecognized,
		Unimplemented,
		CannotDetermineReservedLocation,
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
			let dest_reserve_location: MultiLocation = (
				0,
				X2(
					GeneralKey(CB_ASSET_KEY.to_vec()),
					GeneralIndex(dest_id as u128),
				),
			)
				.into();
			let asset_reserve_location = asset
				.reserve_location()
				.ok_or(Error::<T>::CannotDetermineReservedLocation)?;

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
			// Ensure asset is setup for the solo chain
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

			let fee = Self::estimate_fee_in_pha(dest_id, amount);
			// Check native balance to cover fee
			let native_free_balance = <T as Config>::Currency::free_balance(&sender);
			ensure!(native_free_balance >= fee, Error::<T>::InsufficientBalance);

			// Pay fee to treasury
			let imbalance = <T as Config>::Currency::withdraw(
				&sender,
				fee,
				WithdrawReasons::FEE,
				ExistenceRequirement::AllowDeath,
			)?;
			T::OnFeePay::on_unbalanced(imbalance);

			let asset_amount = T::BalanceConverter::to_asset_balance(amount, asset_id)
				.map_err(|_| Error::<T>::BalanceConversionFailed)?;

			if asset_reserve_location == dest_reserve_location {
				// Burn if transfer back to its reserve location
				pallet_assets::pallet::Pallet::<T>::burn_from(asset_id, &sender, asset_amount)
					.map_err(|_| Error::<T>::FailedToTransactAsset)?;
			} else {
				// Transfer asset from sender to reserve account
				<pallet_assets::pallet::Pallet<T> as FungibleTransfer<T::AccountId>>::transfer(
					asset_id,
					&sender,
					&dest_reserve_location.into_account().into(),
					asset_amount,
					false,
				)
				.map_err(|_| Error::<T>::FailedToTransactAsset)?;
			}

			// Send message to evm chains
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
			let fee = Self::estimate_fee_in_pha(dest_id, amount);
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
				Self::gen_pha_rid(dest_id),
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
			let bridge_account = T::BridgeOrigin::ensure_origin(origin.clone())?;
			// For solo chain assets, we encode solo chain id as the first byte of resourceId
			let src_chainid: bridge::BridgeChainId = Self::get_chainid(&rid);
			let src_reserve_location: MultiLocation = (
				0,
				X2(
					GeneralKey(CB_ASSET_KEY.to_vec()),
					GeneralIndex(src_chainid as u128),
				),
			)
				.into();

			let dest_location: MultiLocation =
				Decode::decode(&mut dest.as_slice()).map_err(|_| Error::<T>::DestUnrecognized)?;
			let dest_reserve_location = dest_location
				.reserve_location()
				.ok_or(Error::<T>::DestUnrecognized)?;

			let asset_location = Self::rid_to_location(&rid)?;
			let asset_reserve_location = asset_location
				.reserve_location()
				.ok_or(Error::<T>::CannotDetermineReservedLocation)?;

			log::trace!(
				target: LOG_TARGET,
				"Reserve location of assset ${:?}, reserve location of source: {:?}.",
				&asset_reserve_location,
				&src_reserve_location,
			);

			// We received asset send from non-reserve chain, which reserved
			// in the local our other parachains/relaychain. That means we had
			// reserved the asset in a reserve account while it was transfered
			// the the source chain, so here we need withdraw/burn from the reserve
			// account in advance.
			//
			// Note: If we received asset send from its reserve chain, we just need
			// mint the same amount of asset at local
			if asset_reserve_location != src_reserve_location {
				if rid == Self::gen_pha_rid(src_chainid) {
					// ERC20 PHA save reserved assets in bridge account
					let _imbalance = <T as Config>::Currency::withdraw(
						&bridge_account,
						amount,
						WithdrawReasons::TRANSFER,
						ExistenceRequirement::AllowDeath,
					)?;
				} else {
					let asset_id = Self::rid_to_assetid(&rid)?;
					let asset_amount = T::BalanceConverter::to_asset_balance(amount, asset_id)
						.map_err(|_| Error::<T>::BalanceConversionFailed)?;

					// burn from source reserve account
					pallet_assets::pallet::Pallet::<T>::burn_from(
						asset_id,
						&src_reserve_location.into_account().into(),
						asset_amount,
					)
					.map_err(|_| Error::<T>::FailedToTransactAsset)?;
				};
				log::trace!(
					target: LOG_TARGET,
					"Reserve of asset and src dismatch, burn asset form source reserve location.",
				);
			}

			// The asset already being "mint" or "withdrawn from reserve account", now settle to dest
			match (dest_location.parents, &dest_location.interior) {
				// To local account
				(0, &X1(AccountId32 { network: _, id })) => {
					if rid == Self::gen_pha_rid(src_chainid) {
						// ERC20 PHA transfer
						<T as Config>::Currency::deposit_creating(&id.into(), amount);
					} else {
						let asset_id = Self::rid_to_assetid(&rid)?;
						let asset_amount = T::BalanceConverter::to_asset_balance(amount, asset_id)
							.map_err(|_| Error::<T>::BalanceConversionFailed)?;

						// Mint asset into recipient
						pallet_assets::pallet::Pallet::<T>::mint_into(
							asset_id,
							&id.into(),
							asset_amount,
						)
						.map_err(|_| Error::<T>::FailedToTransactAsset)?;
					}
				}
				// To relaychain or other parachain, forward it by xcm
				(1, X1(AccountId32 { .. })) | (1, X2(Parachain(_), AccountId32 { .. })) => {
					let dest_reserve_account = dest_reserve_location.clone().into_account();
					if asset_reserve_location != dest_reserve_location {
						log::trace!(
							target: LOG_TARGET,
							"Reserve of asset and dest dismatch, deposit asset to dest reserve location.",
						);
						if rid == Self::gen_pha_rid(src_chainid) {
							<T as Config>::Currency::deposit_creating(
								&dest_reserve_account.clone().into(),
								amount,
							);
						} else {
							let asset_id = Self::rid_to_assetid(&rid)?;
							let asset_amount =
								T::BalanceConverter::to_asset_balance(amount, asset_id)
									.map_err(|_| Error::<T>::BalanceConversionFailed)?;
							// Mint asset into dest reserve account
							pallet_assets::pallet::Pallet::<T>::mint_into(
								asset_id,
								&dest_reserve_account.clone().into(),
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
						Junction::AccountId32 {
							network: NetworkId::Any,
							id: dest_reserve_account,
						}
						.into(),
						(asset_location, amount.into()).into(),
						dest_location,
						6000000000u64.into(),
					)?;
				}
				// To other evm chains
				(
					0,
					X3(GeneralKey(_cb_key), GeneralIndex(_evm_chain_id), GeneralKey(_evm_account)),
				) => {
					// TODO
					return Err(Error::<T>::DestUnrecognized.into());
				}
				_ => return Err(Error::<T>::DestUnrecognized.into()),
			}
			Ok(())
		}
	}

	impl<T: Config> Pallet<T> {
		// TODO.wf: A more proper way to estimate fee
		pub fn estimate_fee_in_pha(
			dest_id: bridge::BridgeChainId,
			amount: BalanceOf<T>,
		) -> BalanceOf<T> {
			let (min_fee, fee_scale) = Self::bridge_fee(dest_id);
			let fee_estimated = amount * fee_scale.into() / 1000u32.into();
			if fee_estimated > min_fee {
				fee_estimated
			} else {
				min_fee
			}
		}

		pub fn rid_to_location(rid: &[u8; 32]) -> Result<MultiLocation, DispatchError> {
			let src_chainid: bridge::BridgeChainId = Self::get_chainid(rid);
			let asset_location: MultiLocation = if *rid == Self::gen_pha_rid(src_chainid) {
				MultiLocation::here()
			} else {
				let xtransfer_asset: XTransferAsset = T::AssetsWrapper::lookup_by_resource_id(&rid)
					.ok_or(Error::<T>::AssetConversionFailed)?;
				xtransfer_asset.into()
			};
			Ok(asset_location)
		}

		pub fn rid_to_assetid(
			rid: &[u8; 32],
		) -> Result<<T as pallet_assets::Config>::AssetId, DispatchError> {
			let src_chainid: bridge::BridgeChainId = Self::get_chainid(rid);
			// PHA based on pallet_balances, not pallet_assets
			if *rid == Self::gen_pha_rid(src_chainid) {
				return Err(Error::<T>::AssetNotRegistered.into());
			}
			let xtransfer_asset: XTransferAsset = T::AssetsWrapper::lookup_by_resource_id(&rid)
				.ok_or(Error::<T>::AssetConversionFailed)?;
			let asset_id: <T as pallet_assets::Config>::AssetId =
				T::AssetsWrapper::id(&xtransfer_asset).ok_or(Error::<T>::AssetNotRegistered)?;
			Ok(asset_id)
		}

		pub fn gen_pha_rid(chain_id: bridge::BridgeChainId) -> bridge::ResourceId {
			XTransferAsset(MultiLocation::here()).into_rid(chain_id)
		}

		pub fn get_chainid(rid: &bridge::ResourceId) -> bridge::BridgeChainId {
			rid[0]
		}
	}

	pub trait GetBridgeFee {
		fn get_fee(chain_id: bridge::BridgeChainId, asset: &MultiAsset) -> Option<u128>;
	}
	impl<T: Config> GetBridgeFee for Pallet<T>
	where
		BalanceOf<T>: From<u128> + Into<u128>,
	{
		fn get_fee(chain_id: bridge::BridgeChainId, asset: &MultiAsset) -> Option<u128> {
			return match (&asset.id, &asset.fun) {
				(Concrete(asset_id), Fungible(amount)) => {
					let fee_estimated_in_pha =
						Self::estimate_fee_in_pha(chain_id, (*amount).into());
					if T::NativeChecker::is_native_asset(asset) {
						Some(fee_estimated_in_pha.into())
					} else {
						let fee_in_asset;
						let fee_prices = T::ExecutionPriceInfo::get();
						if let Some(idx) = fee_prices.iter().position(|(fee_asset_id, _)| {
							fee_asset_id == &Concrete(asset_id.clone())
						}) {
							fee_in_asset = Some(
								fee_estimated_in_pha.into() * fee_prices[idx].1
									/ T::NativeExecutionPrice::get(),
							)
						} else {
							fee_in_asset = None
						}
						fee_in_asset
					}
				}
				_ => None,
			};
		}
	}

	impl GetBridgeFee for () {
		fn get_fee(_chain_id: bridge::BridgeChainId, _asset: &MultiAsset) -> Option<u128> {
			Some(0)
		}
	}
}
