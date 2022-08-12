pub use self::pallet::*;

#[frame_support::pallet]
pub mod pallet {
	use crate::traits::*;
	use assets_registry::{
		AccountId32Conversion, ExtractReserveLocation, GetAssetRegistryInfo, NativeAssetChecker,
		PB_PATH_KEY,
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
	use phala_pallet_common::WrapSlice;
	use phala_pallets::pallet_mq;
	use phala_types::{
        contract::{
			ContractClusterId, ContractId,
		},
        messaging::{bind_topic, DecodedMessage, MessageOrigin}
    };
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
	use xcm_executor::traits::{TransactAsset, WeightBounds};

	bind_topic!(PBridgeReport, b"phala/contract/pbridge/report");
	#[derive(Encode, Decode, Debug, TypeInfo)]
	pub enum PBridgeReport {
		// Encoded xcm message
		Xcm(Vec<u8>),
	}

	const LOG_TARGET: &str = "runtime::pbridge";
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
	pub trait Config: frame_system::Config + pallet_assets::Config + pallet_mq::Config {
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

		/// Origin used to administer the pallet
		type BridgeCommitteeOrigin: EnsureOrigin<Self::Origin>;

		type Currency: Currency<Self::AccountId>;

		/// Filter native asset
		type NativeAssetChecker: NativeAssetChecker;

		/// Treasury account to receive assets fee
		type TreasuryAccount: Get<Self::AccountId>;

		/// Asset adapter to do withdraw, deposit etc.
		type FungibleAdapter: TransactAsset;

		/// Fungible assets registry
		type AssetsRegistry: GetAssetRegistryInfo<<Self as pallet_assets::Config>::AssetId>;

		/// Something to execute an XCM message.
		type XcmExecutor: ExecuteXcm<Self::Call>;

		/// Means of measuring the weight consumed by an XCM message locally.
		type Weigher: WeightBounds<Self::Call>;

		#[pallet::constant]
		type ContractSelector: Get<[u8; 4]>;
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// Relayer added to set
		BridgeContractAdded {
			cluster_id: u8,
			bridge_contract: [u8; 32],
		},
		/// Relayer removed from set
		BridgeContractRemoved {
			cluster_id: u8,
			bridge_contract: [u8; 32],
		},
	}

	#[pallet::error]
	pub enum Error<T> {
		IllegalOrigin,
		DestUnrecognized,
		ExtractDestFailed,
		InsufficientBalance,
		AssetNotRegistered,
		AssetConversionFailed,
		XcmDecodeFailed,
		ExtractAssetFailed,
		XcmExecutionFailed,
		UnweighableMessage,
		TransactFailed,
		CannotDetermineReservedLocation,
		BridgeContractAlreadyExisted,
		BridgeContractNotExisted,
		/// Asset not been registered or not been supported
		AssetNotFound,
		BridgeContractNotFound,
		/// Extract dest location failed
		IllegalDestination,
		/// Can not transfer asset to dest
		CannotDepositAsset,
		/// Transfer type not valid
		UnknownTransferType,
		/// Unimplemented function
		Unimplemented,
	}

	/// Map cluster and coressponding deployed bridge contract
	#[pallet::storage]
	#[pallet::getter(fn storemans)]
	pub type BridgeContracts<T: Config> = StorageMap<_, Twox64Concat, u8, [u8; 32]>;

	#[pallet::call]
	impl<T: Config> Pallet<T>
	where
		<T as frame_system::Config>::AccountId: From<[u8; 32]> + Into<[u8; 32]>,
		<T as pallet_assets::Config>::Balance: From<u128> + Into<u128>,
		BalanceOf<T>: From<u128> + Into<u128>,
	{
		/// Mark account as storeman on chain
		#[pallet::weight(195_000_000)]
		pub fn add_bridgecontract(
			origin: OriginFor<T>,
			cluster_id: u8,
			bridge_contract: [u8; 32],
		) -> DispatchResult {
			T::BridgeCommitteeOrigin::ensure_origin(origin)?;

			ensure!(
				BridgeContracts::<T>::get(&cluster_id).is_none(),
				Error::<T>::BridgeContractAlreadyExisted
			);
			BridgeContracts::<T>::insert(&cluster_id, &bridge_contract);

			Self::deposit_event(Event::BridgeContractAdded {
				cluster_id,
				bridge_contract,
			});
			Ok(())
		}

		/// Remove an account as storeman on chain
		#[pallet::weight(195_000_000)]
		pub fn remove_bridgecontract(origin: OriginFor<T>, cluster_id: u8) -> DispatchResult {
			T::BridgeCommitteeOrigin::ensure_origin(origin)?;

			let bridge_contract = BridgeContracts::<T>::get(&cluster_id)
				.ok_or(Error::<T>::BridgeContractNotExisted)?;
			BridgeContracts::<T>::remove(&cluster_id);

			Self::deposit_event(Event::BridgeContractRemoved {
				cluster_id,
				bridge_contract,
			});
			Ok(())
		}
	}

	impl<T: Config> Pallet<T>
	where
		BalanceOf<T>: From<u128> + Into<u128>,
		<T as frame_system::Config>::AccountId: From<[u8; 32]> + Into<[u8; 32]>,
		<T as pallet_assets::Config>::Balance: From<u128> + Into<u128>,
	{
		/// Triggered by a initial transfer on source chain, executed by pbridge storeman group when proposal was resolved.
		#[transactional]
		pub fn on_fatcontract_message_received(
			message: DecodedMessage<PBridgeReport>,
		) -> DispatchResult {
			let _ = match message.sender {
				MessageOrigin::Cluster(cluster) => cluster,
				_ => return Err(Error::<T>::IllegalOrigin.into()),
			};

			match message.payload {
				PBridgeReport::Xcm(encoded_xcm) => {
					let origin_location = Junction::AccountId32 {
						network: NetworkId::Any,
						id: Self::fatcontract_reserve_location()
							.clone()
							.into_account()
							.into(),
					}
					.into();
					let xcm: &mut Xcm<T::Call> = &mut Decode::decode(&mut encoded_xcm.as_slice())
						.map_err(|_| Error::<T>::XcmDecodeFailed)?;
					let weight =
						T::Weigher::weight(xcm).map_err(|()| Error::<T>::UnweighableMessage)?;
					// TODO: barriers check
					T::XcmExecutor::execute_xcm_in_credit(
						origin_location,
						xcm.clone(),
						weight,
						weight,
					)
					.ensure_complete()
					.map_err(|_| Error::<T>::XcmExecutionFailed)?;
				}
			}

			Ok(())
		}

		fn extract_fungible(asset: MultiAsset) -> Option<(MultiLocation, u128)> {
			match (asset.fun, asset.id) {
				(Fungible(amount), Concrete(location)) => Some((location, amount)),
				_ => None,
			}
		}

		fn extract_dest(dest: &MultiLocation) -> Option<Vec<u8>> {
			match (dest.parents, &dest.interior) {
				// Destnation is a standalone chain.
				(0, Junctions::X2(GeneralKey(pb_key), GeneralKey(recipient))) => {
					if pb_key.clone().into_inner() == PB_PATH_KEY.to_vec() {
						// Return account in FatContract
						Some(recipient.to_vec())
					} else {
						None
					}
				}
				_ => None,
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

		fn fatcontract_reserve_location() -> MultiLocation {
			(0, X1(GeneralKey(WrapSlice(PB_PATH_KEY).into()))).into()
		}

		fn parse_asset_contract(asset_location: &MultiLocation) -> Option<ContractId> {
			// TODO
			None
		}

		fn parse_bridge_contract(cluster: &ContractClusterId) -> Option<ContractId> {
			// TODO
			None
		}

		fn parse_cluster(contract: &ContractId) -> Option<ContractClusterId> {
			// TODO
			None
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
				(Some((asset_location, _)), Some(_)) => {
					// Reject all non-native assets that are not registered in the registry
					if !T::NativeAssetChecker::is_native_asset(&asset)
						&& T::AssetsRegistry::id(&asset_location) == None
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
			let recipient =
				Pallet::<T>::extract_dest(&dest).ok_or(Error::<T>::ExtractDestFailed)?;
			let asset_contract = Pallet::<T>::parse_asset_contract(&asset_location)
				.ok_or(Error::<T>::AssetNotFound)?;
			let cluster = Pallet::<T>::parse_cluster(&asset_contract)
				.ok_or(Error::<T>::AssetConversionFailed)?;
			let bridge_contract = Pallet::<T>::parse_bridge_contract(&cluster)
				.ok_or(Error::<T>::BridgeContractNotFound)?;

			log::trace!(
				target: LOG_TARGET,
				" pbridge fungible transfer, sender: {:?}, asset: {:?}, dest: {:?}.",
				sender,
				&asset,
				&dest,
			);

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

			// Deposit `amount` of asset to reserve account if asset is not reserved in dest.
			let dest_reserve_location = Pallet::<T>::fatcontract_reserve_location();
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
					&(asset.id.clone(), Fungible((amount).into())).into(),
					&Junction::AccountId32 {
						network: NetworkId::Any,
						id: reserve_account.into(),
					}
					.into(),
				)
				.map_err(|_| Error::<T>::TransactFailed)?;
			}

			// Call bridge contract method deployed in pRuntime
			let payload = (
				T::ContractSelector::get(),
				asset_contract,
				recipient,
				amount,
			)
				.encode();
            // TODO: waiting to be merged: https://github.com/Phala-Network/phala-blockchain/pull/918
			// Pallet::<T>::push_ink_message(bridge_contract, payload);

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
