pub use self::pallet::*;

#[allow(unused_variables)]
#[frame_support::pallet]
pub mod pallet {
	use crate::traits::*;
	use assets_registry::{
		AccountId32Conversion, ExtractReserveLocation, GetAssetRegistryInfo, CR_PATH_KEY,
	};
	use codec::{Decode, Encode, EncodeLike};
	pub use frame_support::{
		pallet_prelude::*,
		traits::{tokens::fungibles::Inspect, Currency, StorageVersion},
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

	const LOG_TARGET: &str = "runtime::celerbridge";
	const STORAGE_VERSION: StorageVersion = StorageVersion::new(0);
	const MODULE_ID: PalletId = PalletId(*b"phala/bg");

	type BalanceOf<T> =
		<<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	#[pallet::storage_version(STORAGE_VERSION)]
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
		type BridgeChainId: Get<u64>;

		/// Currency impl
		type Currency: Currency<Self::AccountId>;

		/// Check whether an asset is PHA
		type NativeAssetChecker: NativeAssetChecker;

		/// Execution price in PHA
		type NativeExecutionPrice: Get<u128>;

		/// Execution price information
		type ExecutionPriceInfo: Get<Vec<(XcmAssetId, u128)>>;

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
		ChainWhitelisted(u64),
		/// Fee updated for the specific chain
		FeeUpdated {
			dest_id: u64,
			min_fee: u128,
			fee_scale: u32,
		},
		Deposited {
			deposit_id: [u8; 32],
			depositer: Vec<u8>,
			token: Vec<u8>,
			amount: U256,
			mint_chain_id: u64,
			mint_account: Vec<u8>,
		},
		Withdrawn {
			withdraw_id: [u8; 32],
			receiver: Vec<u8>,
			token: Vec<u8>,
			amount: U256,
			ref_chain_id: u64,
			ref_id: [u8; 32],
			burn_account: Vec<u8>,
		},
		Mint {
			mint_id: [u8; 32],
			token: Vec<u8>,
			account: Vec<u8>,
			amount: U256,
			ref_chain_id: u64,
			ref_id: [u8; 32],
			depositer: Vec<u8>,
		},
		Burn {
			burn_id: [u8; 32],
			token: Vec<u8>,
			account: Vec<u8>,
			amount: U256,
			withdraw_account: Vec<u8>,
		},
	}

	#[pallet::error]
	pub enum Error<T> {
		/// Provided chain Id is not valid
		InvalidChainId,
		/// Interactions with this chain is not permitted
		ChainNotWhitelisted,
		/// Chain has already been enabled
		ChainAlreadyWhitelisted,
		/// Got wrong paremeter when update fee
		InvalidFeeOption,
		/// Unkonwn asset
		ExtractAssetFailed,
		/// Unknown destnation
		ExtractDestFailed,
		/// Asset can not pay as fee
		CannotPayAsFee,
		/// Transfer failed
		TransactFailed,
		/// Infusficient balance to withdraw
		InsufficientBalance,
		/// Too expensive fee for withdrawn asset
		FeeTooExpensive,
		/// Can not extract asset reserve location
		CannotDetermineReservedLocation,
		/// Can not extract dest location
		DestUnrecognized,
		/// Assets not registered through pallet-assets or pallet-uniques
		AssetNotRegistered,
		/// Convertion failed from resource id
		AssetConversionFailed,
		/// Function unimplemented
		Unimplemented,
		/// Can not transfer assets to dest due to some reasons
		CannotDepositAsset,
		/// Same deposit id or burn id exist
		DuplicatedTransfer,
	}

	#[pallet::storage]
	#[pallet::getter(fn chains)]
	pub type ChainNonces<T> = StorageMap<_, Blake2_256, u64, u64>;

	#[pallet::storage]
	#[pallet::getter(fn nonces)]
	pub type TransferNonces<T> = StorageMap<_, Blake2_256, u64, u64, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn records)]
	pub type Records<T> = StorageMap<_, Twox64Concat, [u8; 32], bool, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn bridge_fee)]
	pub type BridgeFee<T: Config> = StorageMap<_, Twox64Concat, u64, (u128, u32)>;

	#[pallet::call]
	impl<T: Config> Pallet<T>
	where
		<T as frame_system::Config>::AccountId: From<[u8; 32]> + Into<[u8; 32]>,
		<T as pallet_assets::Config>::Balance: From<u128> + Into<u128>,
		BalanceOf<T>: From<u128> + Into<u128>,
	{
		/// Enables a chain ID as a source or destination for a bridge transfer.
		///
		/// # <weight>
		/// - O(1) lookup and insert
		/// # </weight>
		#[pallet::weight(195_000_000)]
		pub fn whitelist_chain(origin: OriginFor<T>, id: u64) -> DispatchResult {
			T::BridgeCommitteeOrigin::ensure_origin(origin)?;
			Self::whitelist(id)
		}

		/// Change extra bridge transfer fee that user should pay
		///
		/// # <weight>
		/// - O(1) lookup and insert
		/// # </weight>
		#[pallet::weight(195_000_000)]
		pub fn update_fee(
			origin: OriginFor<T>,
			min_fee: u128,
			fee_scale: u32,
			dest_id: u64,
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

		/// Triggered by a initial transfer on source chain, executed by relayer when proposal was resolved.
		#[pallet::weight(195_000_000)]
		pub fn withdraw(
			origin: OriginFor<T>,
			request: Vec<u8>,
			sigs: Vec<u8>,
			signers: Vec<[u8; 20]>,
			powers: Vec<U256>,
		) -> DispatchResult {
			// TODO: Impl
			Ok(())
		}

		/// Triggered by a initial transfer on source chain, executed by relayer when proposal was resolved.
		#[pallet::weight(195_000_000)]
		pub fn mint(
			origin: OriginFor<T>,
			request: Vec<u8>,
			sigs: Vec<u8>,
			signers: Vec<[u8; 20]>,
			powers: Vec<U256>,
		) -> DispatchResult {
			// TODO: Impl
			Ok(())
		}
	}

	impl<T: Config> Pallet<T>
	where
		<T as frame_system::Config>::AccountId: From<[u8; 32]> + Into<[u8; 32]>,
		<T as pallet_assets::Config>::Balance: From<u128> + Into<u128>,
		BalanceOf<T>: From<u128> + Into<u128>,
	{
		/// Provides an AccountId for the pallet.
		/// This is used both as an origin check and deposit/withdrawal account.
		pub fn account_id() -> T::AccountId {
			MODULE_ID.into_account()
		}

		/// Checks if a chain exists as a whitelisted destination
		pub fn chain_whitelisted(id: u64) -> bool {
			Self::chains(id).is_some()
		}

		/// Increments the deposit nonce for the specified chain ID
		fn bump_nonce(id: u64) -> u64 {
			let nonce = Self::chains(id).unwrap_or_default() + 1;
			ChainNonces::<T>::insert(id, nonce);
			nonce
		}

		/// Whitelist a chain ID for transfer
		pub fn whitelist(id: u64) -> DispatchResult {
			// Cannot whitelist this chain
			ensure!(id != T::BridgeChainId::get(), Error::<T>::InvalidChainId);
			// Cannot whitelist with an existing entry
			ensure!(
				!Self::chain_whitelisted(id),
				Error::<T>::ChainAlreadyWhitelisted
			);
			ChainNonces::<T>::insert(&id, 0);
			Self::deposit_event(Event::ChainWhitelisted(id));
			Ok(())
		}

		fn extract_fungible(asset: MultiAsset) -> Option<(MultiLocation, u128)> {
			return match (asset.fun, asset.id) {
				(Fungible(amount), Concrete(location)) => Some((location, amount)),
				_ => None,
			};
		}

		fn extract_dest(dest: &MultiLocation) -> Option<(u64, Vec<u8>)> {
			return match (dest.parents, &dest.interior) {
				// Destnation is a foreign chain. Forward it through the bridge
				(
					0,
					Junctions::X3(
						GeneralKey(cr_key),
						GeneralIndex(chain_id),
						GeneralKey(recipient),
					),
				) => {
					if &cr_key != &CR_PATH_KEY {
						return None;
					}
					if let Some(chain_id) = TryInto::<u64>::try_into(*chain_id).ok() {
						Some((chain_id, recipient.to_vec()))
					} else {
						None
					}
				}
				_ => None,
			};
		}

		fn get_fee(chain_id: u64, asset: &MultiAsset) -> Option<u128> {
			Some(1_000_000_000_000)
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

/*
		// 132 bytes
		fn gen_withdraw_id(
			receiver: [u8; 20],
			token: [u8; 20],
			amount: U256,
			burn_account: [u8; 20],
			ref_chain_id: u64,
			ref_id: [u8; 32],
		) -> [u8; 32] {
			let burn_id: Vec<_> = receiver
				.iter()
				.chain(token.iter())
				.chain(Into::<[u8; 32]>::into(amount).iter())
				.chain(burn_account.iter())
				.chain(ref_chain_id.to_le_bytes().iter())
				.chain(ref_id.iter())
				.collect();
			let vec: Vec<u8> = burn_id.to_vec();
			sp_io::hashing::keccak_256(&vec)
		}

		// 132 bytes
		fn gen_mint_id(
			account: [u8; 20],
			token: [u8; 20],
			amount: U256,
			depositor: [u8; 20],
			ref_chain_id: u64,
			ref_id: [u8; 32],
		) -> [u8; 32] {
			let burn_id: Vec<_> = account
				.iter()
				.chain(token.iter())
				.chain(Into::<[u8; 32]>::into(amount).iter())
				.chain(depositor.iter())
				.chain(ref_chain_id.to_le_bytes().iter())
				.chain(ref_id.iter())
				.collect();
			let vec: Vec<u8> = burn_id.to_vec();
			sp_io::hashing::keccak_256(&vec)
		}
*/
		pub fn when_asset_send(
			sender: &[u8; 32],
			asset: &MultiAsset,
			dest: &MultiLocation,
		) -> DispatchResult {
			let (asset_location, amount) = Pallet::<T>::extract_fungible(asset.clone())
				.ok_or(Error::<T>::ExtractAssetFailed)?;
			let (dest_id, recipient) =
				Pallet::<T>::extract_dest(&dest).ok_or(Error::<T>::ExtractDestFailed)?;

			let dest_reserve_location: MultiLocation = (
				0,
				X2(
					GeneralKey(CR_PATH_KEY.to_vec()),
					GeneralIndex(dest_id.into()),
				),
			)
				.into();
			let asset_reserve_location = asset_location
				.clone()
				.reserve_location()
				.ok_or(Error::<T>::CannotDetermineReservedLocation)?;
			let reserve_account = if T::NativeAssetChecker::is_native_asset(&asset) {
				MODULE_ID.into_account()
			} else {
				dest_reserve_location.clone().into_account()
			};

			let asset_id =
				T::AssetsRegistry::id(&asset_location).ok_or(Error::<T>::AssetNotRegistered)?;
			let evm_address: [u8; 20] = T::AssetsRegistry::evm_address(&asset_id)
				.ok_or(Error::<T>::AssetConversionFailed)?;
			let nonce = TransferNonces::<T>::get(&dest_id);
			let amount_bytes: [u8; 32] = U256::from(amount).into();

			let id = if T::NativeAssetChecker::is_native_asset(&asset)
				|| asset_reserve_location != dest_reserve_location
			{
				// Data length used to generate deposit id is 20 + 20 + 32 + 8 + 20 + 8 + 8 = 116 bytes
				// - Sender account, AccountId20
				// - ERC20 token address
				// - Transfer amount
				// - Mint chain id
				// - Mint account
				// - Deposit nonce
				// - Origin chain id
				let data: Vec<u8> = sender
					.iter()
					.chain(evm_address.iter())
					.chain(amount_bytes.iter())
					.chain(dest_id.to_le_bytes().iter())
					.chain(recipient.iter())
					.chain(nonce.to_le_bytes().iter())
					.chain(T::BridgeChainId::get().to_le_bytes().iter())
					.collect();
				let deposit_id = sp_io::hashing::keccak_256(data.as_slice());
				ensure!(
					Records::<T>::get(&deposit_id) == false,
					Error::<T>::DuplicatedTransfer
				);

				Self::deposit_event(Event::Deposited {
					deposit_id: deposit_id.clone(),
					depositer: sender.to_vec(),
					token: evm_address.to_vec(),
					amount: U256::from(amount),
					mint_chain_id: dest_id,
					mint_account: recipient,
				});
				deposit_id
			} else {
				// Data length used to generate burn id is 20 + 20 + 32 + 20 + 8 + 8 = 108 bytes.
				// - Sender account, AccountId20
				// - ERC20 token address
				// - Transfer amount
				// - Withdraw account on dest chain
				// - Deposit nonce
				// - Origin chain id
				let data: Vec<u8> = sender
					.iter()
					.chain(evm_address.iter())
					.chain(amount_bytes.iter())
					.chain(recipient.iter())
					.chain(nonce.to_le_bytes().iter())
					.chain(T::BridgeChainId::get().to_le_bytes().iter())
					.collect();
				let burn_id = sp_io::hashing::keccak_256(data.as_slice());
				ensure!(
					Records::<T>::get(&burn_id) == false,
					Error::<T>::DuplicatedTransfer
				);

				Self::deposit_event(Event::Burn {
					burn_id: burn_id.clone(),
					token: evm_address.to_vec(),
					account: sender.to_vec(),
					amount: U256::from(amount),
					withdraw_account: recipient,
				});
				burn_id
			};
			Records::<T>::insert(&id, true);
			TransferNonces::<T>::insert(&dest_id, nonce + 1);

			Ok(())
		}

		pub fn when_asset_received(asset: MultiAsset, dest: MultiLocation) -> DispatchResult {
			Ok(())
		}
	}

	impl<T: Config> BridgeChecker for Pallet<T>
	where
		<T as frame_system::Config>::AccountId: From<[u8; 32]> + Into<[u8; 32]>,
		<T as pallet_assets::Config>::Balance: From<u128> + Into<u128>,
		BalanceOf<T>: From<u128> + Into<u128>,
	{
		fn can_deposit_asset(asset: MultiAsset, dest: MultiLocation) -> bool {
			return match (
				Self::extract_fungible(asset.clone()),
				Self::extract_dest(&dest),
			) {
				(Some((asset_location, _)), Some((dest_id, _))) => {
					// Verify if dest chain has been whitelisted
					if Self::chain_whitelisted(dest_id) == false {
						return false;
					}

					// Verify if destination has fee set
					if BridgeFee::<T>::contains_key(&dest_id) == false {
						return false;
					}

					// Verify if asset was registered if is not native.
					if !T::NativeAssetChecker::is_native_asset(&asset)
						&& T::AssetsRegistry::id(&asset_location) == None
					{
						return false;
					}

					// TODO: Verify if asset was enabled celerbridge transfer if is not native

					true
				}
				_ => false,
			};
			// TODO: NonFungible verification
		}

		fn can_send_data(_data: &Vec<u8>, _dest: MultiLocation) -> bool {
			// TODO: impl
			return false;
		}
	}

	pub struct BridgeTransactImpl<T>(PhantomData<T>);
	impl<T: Config> BridgeTransact for BridgeTransactImpl<T>
	where
		<T as frame_system::Config>::AccountId: From<[u8; 32]> + Into<[u8; 32]>,
		<T as pallet_assets::Config>::Balance: From<u128> + Into<u128>,
		BalanceOf<T>: From<u128> + Into<u128>,
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
				" Celerbridge fungible transfer, sender: {:?}, asset: {:?}, dest: {:?}.",
				sender,
				&asset,
				&dest,
			);

			let fee = Pallet::<T>::get_fee(
				dest_id,
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
					GeneralKey(CR_PATH_KEY.to_vec()),
					GeneralIndex(dest_id.into()),
				),
			)
				.into();
			let asset_reserve_location = asset_location
				.clone()
				.reserve_location()
				.ok_or(Error::<T>::CannotDetermineReservedLocation)?;
			let reserve_account = if T::NativeAssetChecker::is_native_asset(&asset) {
				MODULE_ID.into_account()
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

			Pallet::<T>::when_asset_send(&sender, &asset, &dest)?;
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
