pub use self::pallet::*;

#[frame_support::pallet]
pub mod pallet {
	use crate::traits::*;
	use assets_registry::{
		AccountId32Conversion, ExtractReserveLocation, GetAssetRegistryInfo, NativeAssetChecker,
		WB_PATH_KEY,
	};
	use codec::{Decode, Encode, EncodeLike};
	pub use frame_support::{
		pallet_prelude::*,
		traits::{tokens::fungibles::Inspect, Currency, StorageVersion},
		transactional,
		weights::GetDispatchInfo,
		PalletId, Parameter,
	};
	use frame_system::{self as system, pallet_prelude::*};
	use scale_info::TypeInfo;
	pub use sp_core::U256;
	use sp_runtime::traits::{AccountIdConversion, Dispatchable};
	use sp_runtime::RuntimeDebug;
	use sp_std::{
		convert::{From, Into, TryInto},
		prelude::*,
	};
	use xcm::latest::{
		prelude::*, AssetId as XcmAssetId, Fungibility::Fungible, MultiAsset, MultiLocation,
	};
	use xcm_executor::traits::TransactAsset;

	const LOG_TARGET: &str = "runtime::wanbridge";
	const STORAGE_VERSION: StorageVersion = StorageVersion::new(2);
	const MODULE_ID: PalletId = PalletId(*b"phala/bg");

	type BalanceOf<T> =
		<<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	#[pallet::storage_version(STORAGE_VERSION)]
	// TODO: remove when we Vec get replaced by BoundedVec
	#[pallet::without_storage_info]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: frame_system::Config + pallet_assets::Config {
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

		/// Origin used to administer the pallet
		type BridgeCommitteeOrigin: EnsureOrigin<Self::Origin>;

		/// The identifier for this chain.
		/// This must be unique and must not collide with existing IDs within a set of bridged chains.
		#[pallet::constant]
		type CurrentChainId: Get<u8>;

		type Currency: Currency<Self::AccountId>;

		/// Filter native asset
		type NativeAssetChecker: NativeAssetChecker;

		/// Token pair of native asset
		type NativeTokenPair: Get<u32>;

		/// Execution price in PHA
		type NativeExecutionPrice: Get<u128>;

		/// Treasury account to receive assets fee
		type TreasuryAccount: Get<Self::AccountId>;

		/// Asset adapter to do withdraw, deposit etc.
		type FungibleAdapter: TransactAsset;

		/// Fungible assets registry
		type AssetsRegistry: GetAssetRegistryInfo<<Self as pallet_assets::Config>::AssetId>;
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// Chain now available for transfers (chain_id)
		ChainWhitelisted {
			chain_id: U256,
		},
		ChainBlacklisted {
			chain_id: U256,
		},
		FeeUpdated {
			chain_id: U256,
			fee: u128,
		},
		/// Relayer added to set
		StoreManAdded {
			account: T::AccountId,
		},
		/// Relayer removed from set
		StoreManRemoved {
			account: T::AccountId,
		},
		/// Assets sent to standalone chains.
		FungibleTransfer {
			smg_id: [u8; 32],
			token_pair: u32,
			chain_id: U256,
			amount: U256,
			recipient: Vec<u8>,
		},
	}

	#[pallet::error]
	pub enum Error<T> {
		DestUnrecognized,
		FeeTooExpensive,
		ExtractDestFailed,
		CannotPayAsFee,
		InsufficientBalance,
		AssetNotRegistered,
		AssetConversionFailed,
		ExtractAssetFailed,
		TransactFailed,
		CannotDetermineReservedLocation,
		/// Given chain id is not valid
		InvalidChainId,
		ChainAlreadyWhitelisted,
		ChainHasnotWhitelisted,
		StoremanAlreadyExisted,
		StoremanNotExisted,
		DuplicateSourceTx,
		MustBeStoreMan,
		/// Missing fee specification
		FeePaymentEmpty,
		/// Asset not been registered or not been supported
		AssetNotFound,
		/// Extract dest location failed
		IllegalDestination,
		/// Can not transfer asset to dest
		CannotDepositAsset,
		/// Transfer type not valid
		UnknownTransferType,
		/// Unimplemented function
		Unimplemented,
	}

	#[pallet::storage]
	#[pallet::getter(fn chain_whitelists)]
	pub type ChainWhitelists<T> = StorageMap<_, Twox64Concat, U256, bool, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn storemans)]
	pub type StoreMans<T: Config> = StorageMap<_, Twox64Concat, T::AccountId, bool, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn source_txs)]
	pub type SourceTxs<T: Config> = StorageMap<_, Twox64Concat, Vec<u8>, bool, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn bridge_fee)]
	pub type BridgeFee<T: Config> = StorageMap<_, Twox64Concat, U256, u128>;

	#[pallet::call]
	impl<T: Config> Pallet<T>
	where
		<T as frame_system::Config>::AccountId: From<[u8; 32]> + Into<[u8; 32]>,
		<T as pallet_assets::Config>::Balance: From<u128> + Into<u128>,
		BalanceOf<T>: From<u128> + Into<u128>,
	{
		/// Enable a chain ID as a source or destination for a bridge transfer.
		#[pallet::weight(195_000_000)]
		pub fn whitelist_chain(origin: OriginFor<T>, chain_id: U256) -> DispatchResult {
			T::BridgeCommitteeOrigin::ensure_origin(origin)?;

			// Cannot whitelist this chain
			ensure!(
				chain_id != U256::from(T::CurrentChainId::get()),
				Error::<T>::InvalidChainId
			);
			// Cannot whitelist with an existing entry
			ensure!(
				!ChainWhitelists::<T>::get(chain_id),
				Error::<T>::ChainAlreadyWhitelisted
			);
			ChainWhitelists::<T>::insert(&chain_id, true);
			Self::deposit_event(Event::ChainWhitelisted { chain_id });
			Ok(())
		}

		/// Disable a chain ID as a source or destination for a bridge transfer.
		#[pallet::weight(195_000_000)]
		pub fn blacklist_chain(origin: OriginFor<T>, chain_id: U256) -> DispatchResult {
			T::BridgeCommitteeOrigin::ensure_origin(origin)?;

			// Cannot whitelist this chain
			ensure!(
				chain_id != U256::from(T::CurrentChainId::get()),
				Error::<T>::InvalidChainId
			);
			// Cannot blacklist with a non-existing entry
			ensure!(
				ChainWhitelists::<T>::get(chain_id),
				Error::<T>::ChainHasnotWhitelisted
			);
			ChainWhitelists::<T>::insert(&chain_id, false);
			Self::deposit_event(Event::ChainBlacklisted { chain_id });
			Ok(())
		}

		/// Change extra bridge transfer fee that user should pay
		#[pallet::weight(195_000_000)]
		pub fn update_fee(origin: OriginFor<T>, fee: u128, chain_id: U256) -> DispatchResult {
			T::BridgeCommitteeOrigin::ensure_origin(origin)?;
			BridgeFee::<T>::insert(chain_id, fee);
			Self::deposit_event(Event::FeeUpdated { chain_id, fee });
			Ok(())
		}

		/// Mark account as storeman on chain
		#[pallet::weight(195_000_000)]
		pub fn add_storeman(origin: OriginFor<T>, account: T::AccountId) -> DispatchResult {
			T::BridgeCommitteeOrigin::ensure_origin(origin)?;

			ensure!(
				!StoreMans::<T>::get(&account),
				Error::<T>::StoremanAlreadyExisted
			);
			StoreMans::<T>::insert(&account, true);

			Self::deposit_event(Event::StoreManAdded { account });
			Ok(())
		}

		/// Remove an account as storeman on chain
		#[pallet::weight(195_000_000)]
		pub fn remove_storeman(origin: OriginFor<T>, account: T::AccountId) -> DispatchResult {
			T::BridgeCommitteeOrigin::ensure_origin(origin)?;

			ensure!(
				StoreMans::<T>::get(&account),
				Error::<T>::StoremanNotExisted
			);
			StoreMans::<T>::insert(&account, false);

			Self::deposit_event(Event::StoreManRemoved { account });
			Ok(())
		}

		/// Triggered by a initial transfer on source chain, executed by wanbridge storeman group when proposal was resolved.
		#[pallet::weight(195_000_000)]
		#[transactional]
		pub fn handle_fungible_transfer(
			origin: OriginFor<T>,
			_smg_id: [u8; 32],
			token_pair: u32,
			src_chainid: U256,
			amount: BalanceOf<T>,
			recipient: Vec<u8>,
			tx_id: Vec<u8>,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			ensure!(StoreMans::<T>::get(&who), Error::<T>::MustBeStoreMan);

			// Make sure we haven't handled this incomming transfer
			ensure!(!SourceTxs::<T>::get(&tx_id), Error::<T>::DuplicateSourceTx);

			// TODO.wf: check according to smg_id

			let src_chainid = TryInto::<u128>::try_into(src_chainid)
				.map_err(|_| Error::<T>::CannotDetermineReservedLocation)?;
			let src_reserve_location: MultiLocation = (
				0,
				X2(GeneralKey(WB_PATH_KEY.to_vec()), GeneralIndex(src_chainid)),
			)
				.into();
			let dest_location: MultiLocation = Decode::decode(&mut recipient.as_slice())
				.map_err(|_| Error::<T>::DestUnrecognized)?;
			let asset_location = Self::tokenpair_to_location(token_pair)?;
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
			// in the local or other parachains/relaychain. That means we had
			// reserved the asset in a reserve account while it was transfered
			// the the source chain, so here we need withdraw/burn from the reserve
			// account in advance.
			//
			// Note: If we received asset send from its reserve chain, we just need
			// mint the same amount of asset at local
			if asset_reserve_location != src_reserve_location {
				let reserve_account: T::AccountId = if token_pair == T::NativeTokenPair::get() {
					// PHA need to be released from bridge account due to historical reason
					MODULE_ID.into_account_truncating()
				} else {
					src_reserve_location.into_account().into()
				};
				T::FungibleAdapter::withdraw_asset(
					&(asset_location.clone(), amount.into()).into(),
					&Junction::AccountId32 {
						network: NetworkId::Any,
						id: reserve_account.into(),
					}
					.into(),
				)
				.map_err(|_| Error::<T>::TransactFailed)?;
				log::trace!(
                    target: LOG_TARGET,
                    "Reserve of asset and src dismatch, withdraw asset form source reserve location.",
                );
			}

			// If dest_location is not a local account, adapter will forward it to other bridges
			T::FungibleAdapter::deposit_asset(
				&(asset_location.clone(), amount.into()).into(),
				&dest_location,
			)
			.map_err(|_| Error::<T>::TransactFailed)?;

			// Everything is done, make this crosschain transfer as handled
			SourceTxs::<T>::insert(&tx_id, true);

			Ok(())
		}
	}

	impl<T: Config> Pallet<T>
	where
		BalanceOf<T>: From<u128> + Into<u128>,
		<T as frame_system::Config>::AccountId: From<[u8; 32]> + Into<[u8; 32]>,
		<T as pallet_assets::Config>::Balance: From<u128> + Into<u128>,
	{
		fn extract_fungible(asset: MultiAsset) -> Option<(MultiLocation, u128)> {
			match (asset.fun, asset.id) {
				(Fungible(amount), Concrete(location)) => Some((location, amount)),
				_ => None,
			}
		}

		fn extract_dest(dest: &MultiLocation) -> Option<(u128, Vec<u8>)> {
			match (dest.parents, &dest.interior) {
				// Destnation is a standalone chain.
				(
					0,
					Junctions::X3(GeneralKey(_), GeneralIndex(chain_id), GeneralKey(recipient)),
				) => {
					if let Some(chain_id) = TryInto::<u128>::try_into(*chain_id).ok() {
						// We don't verify wb_key here because can_deposit_asset did it.
						Some((chain_id, recipient.to_vec()))
					} else {
						None
					}
				}
				_ => None,
			}
		}

		pub fn tokenpair_to_location(token_pair: u32) -> Result<MultiLocation, DispatchError> {
			if token_pair == T::NativeTokenPair::get() {
				Ok(MultiLocation::here())
			} else {
				let location = T::AssetsRegistry::lookup_by_token_pair(token_pair)
					.ok_or(Error::<T>::AssetConversionFailed)?;
				Ok(location)
			}
		}

		fn multiasset_to_tokenpair(asset: &MultiAsset) -> Result<u32, DispatchError> {
			if T::NativeAssetChecker::is_native_asset(asset) {
				Ok(T::NativeTokenPair::get())
			} else {
				let (asset_location, _) =
					Self::extract_fungible(asset.clone()).ok_or(Error::<T>::ExtractAssetFailed)?;
				let token_pair = T::AssetsRegistry::wanbridge_token_pair(&asset_location)
					.ok_or(Error::<T>::AssetConversionFailed)?;
				Ok(token_pair)
			}
		}

		fn check_balance(sender: T::AccountId, asset: &MultiAsset, amount: u128) -> DispatchResult {
			let balance: u128 = if T::NativeAssetChecker::is_native_asset(asset) {
				<T as Config>::Currency::free_balance(&sender).into()
			} else {
				let (asset_location, _) =
					Self::extract_fungible(asset.clone()).ok_or(Error::<T>::ExtractAssetFailed)?;
				let asset_id: <T as pallet_assets::Config>::AssetId =
					T::AssetsRegistry::id(&asset_location).ok_or(Error::<T>::AssetNotRegistered)?;
				let reducible_balance = <pallet_assets::pallet::Pallet<T>>::reducible_balance(
					asset_id.into(),
					&sender,
					false,
				);
				reducible_balance.into()
			};

			if balance >= amount {
				Ok(())
			} else {
				Err(Error::<T>::InsufficientBalance.into())
			}
		}

		fn to_e12(amount: u128, decimals: u8) -> u128 {
			if decimals > 12 {
				amount.saturating_div(10u128.saturating_pow(decimals as u32 - 12))
			} else {
				amount.saturating_mul(10u128.saturating_pow(12 - decimals as u32))
			}
		}

		fn from_e12(amount: u128, decimals: u8) -> u128 {
			if decimals > 12 {
				amount.saturating_mul(10u128.saturating_pow(decimals as u32 - 12))
			} else {
				amount.saturating_div(10u128.saturating_pow(12 - decimals as u32))
			}
		}

		fn convert_fee_from_pha(fee_in_pha: u128, price: u128, decimals: u8) -> u128 {
			let fee_e12: u128 = fee_in_pha * price / T::NativeExecutionPrice::get();
			Self::from_e12(fee_e12, decimals)
		}

		fn get_fee(chain_id: U256, asset: &MultiAsset) -> Option<u128> {
			if let Some((location, _)) = Self::extract_fungible(asset.clone()) {
				let decimals = if T::NativeAssetChecker::is_native_asset(asset) {
					12
				} else {
					let id = T::AssetsRegistry::id(&location.clone())?;
					T::AssetsRegistry::decimals(&id).unwrap_or(12)
				};
				if let Some(fee_in_pha) = BridgeFee::<T>::get(chain_id) {
					if T::NativeAssetChecker::is_native_asset(asset) {
						Some(fee_in_pha)
					} else {
						// TODO.wf: Calculate fee according to relay rate
						Some(Self::convert_fee_from_pha(
							fee_in_pha,
							T::NativeExecutionPrice::get(),
							decimals,
						))
					}
				} else {
					None
				}
			} else {
				None
			}
			// TODO: Calculate NonFungible asset fee
		}
	}

	impl<T: Config> BridgeChecker for Pallet<T>
	where
		BalanceOf<T>: From<u128> + Into<u128>,
		<T as frame_system::Config>::AccountId: From<[u8; 32]> + Into<[u8; 32]>,
		<T as pallet_assets::Config>::Balance: From<u128> + Into<u128>,
	{
		fn can_deposit_asset(asset: MultiAsset, dest: MultiLocation) -> bool {
			match (
				Self::extract_fungible(asset.clone()),
				Self::extract_dest(&dest),
			) {
				(Some((asset_location, _)), Some((dest_id, _))) => {
					// Verify if dest chain has been whitelisted
					if !ChainWhitelists::<T>::get(&U256::from(dest_id)) {
						return false;
					}

					// Verify if destination has fee set
					if BridgeFee::<T>::contains_key(&U256::from(dest_id)) == false {
						return false;
					}

					// Reject all non-native assets that are not registered in the registry
					if !T::NativeAssetChecker::is_native_asset(&asset)
						&& T::AssetsRegistry::id(&asset_location) == None
					{
						return false;
					}

					// Verify if asset was enabled chainbridge transfer if is not native
					if !T::NativeAssetChecker::is_native_asset(&asset)
						&& Self::multiasset_to_tokenpair(&asset).is_err()
					{
						return false;
					}

					true
				}
				_ => false,
			}
			// TODO: NonFungible verification
		}

		fn can_send_data(_data: &Vec<u8>, _dest: MultiLocation) -> bool {
			// TODO: impl
			false
		}
	}

	pub struct BridgeTransactImpl<T>(PhantomData<T>);
	impl<T: Config> BridgeTransact for BridgeTransactImpl<T>
	where
		BalanceOf<T>: From<u128> + Into<u128>,
		<T as frame_system::Config>::AccountId: From<[u8; 32]> + Into<[u8; 32]>,
		<T as pallet_assets::Config>::Balance: From<u128> + Into<u128>,
	{
		fn new() -> Self {
			Self(PhantomData)
		}

		/// Initiates a transfer of a fungible asset out of the chain. This should be called by another pallet.
		fn transfer_fungible(
			&self,
			sender: [u8; 32],
			asset: MultiAsset,
			dest: MultiLocation,
			_max_weight: Option<Weight>,
		) -> DispatchResult {
			// Check if we can deposit asset into dest.
			ensure!(
				Pallet::<T>::can_deposit_asset(asset.clone(), dest.clone()),
				Error::<T>::CannotDepositAsset
			);

			let (asset_location, amount) = Pallet::<T>::extract_fungible(asset.clone())
				.ok_or(Error::<T>::ExtractAssetFailed)?;
			let (dest_id, recipient) =
				Pallet::<T>::extract_dest(&dest).ok_or(Error::<T>::ExtractDestFailed)?;

			log::trace!(
				target: LOG_TARGET,
				" Wanbridge fungible transfer, sender: {:?}, asset: {:?}, dest: {:?}.",
				sender,
				&asset,
				&dest,
			);

			let fee = Pallet::<T>::get_fee(
				U256::from(dest_id),
				&(
					Concrete(asset_location.clone().into()),
					Fungible(amount.into()),
				)
					.into(),
			)
			.ok_or(Error::<T>::CannotPayAsFee)?;
			// No need to transfer to to dest chains if it's not enough to pay fee.
			ensure!(amount > fee, Error::<T>::FeeTooExpensive);
			// Ensure we have sufficient free balance
			Pallet::<T>::check_balance(sender.into(), &asset, amount)?;

			// Withdraw `amount` of asset from sender
			T::FungibleAdapter::withdraw_asset(
				&(asset.id.clone(), Fungible(amount.into())).into(),
				&Junction::AccountId32 {
					network: NetworkId::Any,
					id: sender,
				}
				.into(),
			)
			.map_err(|_| Error::<T>::TransactFailed)?;

			// Deposit `fee` of asset to treasury account
			T::FungibleAdapter::deposit_asset(
				&(asset.id.clone(), Fungible(fee.into())).into(),
				&Junction::AccountId32 {
					network: NetworkId::Any,
					id: T::TreasuryAccount::get().into(),
				}
				.into(),
			)
			.map_err(|_| Error::<T>::TransactFailed)?;

			// Deposit `amount - fee` of asset to reserve account if asset is not reserved in dest.
			let dest_reserve_location: MultiLocation = (
				0,
				X2(
					GeneralKey(WB_PATH_KEY.to_vec()),
					GeneralIndex(dest_id.into()),
				),
			)
				.into();
			let asset_reserve_location = asset_location
				.clone()
				.reserve_location()
				.ok_or(Error::<T>::CannotDetermineReservedLocation)?;
			let reserve_account = if T::NativeAssetChecker::is_native_asset(&asset) {
				MODULE_ID.into_account_truncating()
			} else {
				dest_reserve_location.clone().into_account()
			};
			if T::NativeAssetChecker::is_native_asset(&asset)
				|| asset_reserve_location != dest_reserve_location
			{
				T::FungibleAdapter::deposit_asset(
					&(asset.id.clone(), Fungible((amount - fee).into())).into(),
					&Junction::AccountId32 {
						network: NetworkId::Any,
						id: reserve_account.into(),
					}
					.into(),
				)
				.map_err(|_| Error::<T>::TransactFailed)?;
			}

			// Notify storeman the crosschain transfer
			// TODO.wf: Put right smg id here
			let smg_id = [0; 32];
			let token_pair = Pallet::<T>::multiasset_to_tokenpair(&asset)?;
			Pallet::<T>::deposit_event(Event::FungibleTransfer {
				smg_id,
				token_pair,
				chain_id: U256::from(dest_id),
				amount: U256::from(amount - fee),
				recipient,
			});
			Ok(())
		}

		/// Initiates a transfer of a nonfungible asset out of the chain. This should be called by another pallet.
		fn transfer_nonfungible(
			&self,
			_sender: [u8; 32],
			_asset: MultiAsset,
			_dest: MultiLocation,
			_max_weight: Option<Weight>,
		) -> DispatchResult {
			Err(Error::<T>::Unimplemented.into())
		}

		/// Initiates a transfer of generic data out of the chain. This should be called by another pallet.
		fn transfer_generic(
			&self,
			_sender: [u8; 32],
			_data: &Vec<u8>,
			_dest: MultiLocation,
			_max_weight: Option<Weight>,
		) -> DispatchResult {
			Err(Error::<T>::Unimplemented.into())
		}
	}
}
