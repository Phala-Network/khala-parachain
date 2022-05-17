pub use self::pallet::*;

#[frame_support::pallet]
pub mod pallet {
	use crate::helper::*;
	use crate::traits::*;
	use assets_registry::{GetAssetRegistryInfo, NativeAssetChecker};
	use frame_support::{
		dispatch::DispatchResult,
		pallet_prelude::*,
		traits::{Currency, StorageVersion},
		weights::Weight,
	};
	use scale_info::TypeInfo;
	use sp_runtime::DispatchError;
	use sp_std::{prelude::*, vec};
	use xcm::latest::{prelude::*, Fungibility::Fungible, MultiAsset, MultiLocation};
	use xcm_executor::traits::{InvertLocation, WeightBounds};

	/// The logging target.
	const LOG_TARGET: &str = "runtime::xcm-transfer";
	const STORAGE_VERSION: StorageVersion = StorageVersion::new(3);

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	#[pallet::storage_version(STORAGE_VERSION)]
	// TODO: remove when we Vec get replaced by BoundedVec
	#[pallet::without_storage_info]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: frame_system::Config + pallet_assets::Config {
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

		type Currency: Currency<Self::AccountId>;

		/// Required origin for sending XCM messages. If successful, then it resolves to `MultiLocation`
		/// which exists as an interior location within this chain's XCM context.
		type SendXcmOrigin: EnsureOrigin<Self::Origin, Success = MultiLocation>;

		/// The type used to actually dispatch an XCM to its destination.
		type XcmRouter: SendXcm;

		/// Required origin for executing XCM messages, including the teleport functionality. If successful,
		/// then it resolves to `MultiLocation` which exists as an interior location within this chain's XCM
		/// context.
		type ExecuteXcmOrigin: EnsureOrigin<Self::Origin, Success = MultiLocation>;

		/// Something to execute an XCM message.
		type XcmExecutor: ExecuteXcm<Self::Call>;

		/// Means of measuring the weight consumed by an XCM message locally.
		type Weigher: WeightBounds<Self::Call>;

		/// Means of inverting a location.
		type LocationInverter: InvertLocation;

		/// Filter native asset
		type NativeAssetChecker: NativeAssetChecker;

		/// Fungible assets registry
		type AssetsRegistry: GetAssetRegistryInfo<<Self as pallet_assets::Config>::AssetId>;
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// Assets sent to parachain or relaychain.
		AssetTransfered {
			asset: MultiAsset,
			origin: MultiLocation,
			dest: MultiLocation,
		},
	}

	#[pallet::error]
	pub enum Error<T> {
		_UnknownError,
		/// Can not reanchor asset location according dest
		CannotReanchor,
		/// Failed to measure weight of a XCM message
		UnweighableMessage,
		_FeePaymentEmpty,
		/// XCM message executeion failed due to some reasons
		ExecutionFailed,
		_UnknownTransfer,
		/// Asset not been registered or not been supported
		AssetNotFound,
		_LocationInvertFailed,
		/// Extract dest location failed
		IllegalDestination,
		/// Can not transfer asset to dest
		CannotDepositAsset,
		/// Transfer type not valid
		UnknownTransferType,
		/// Unimplemented function
		Unimplemented,
	}

	#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, TypeInfo)]
	pub enum TransferType {
		/// Transfer assets reserved by the origin chain
		FromNative,
		/// Transfer assets reserved by the dest chain
		ToReserve,
		/// Transfer assets not reserved by the dest chain
		ToNonReserve,
	}

	#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, TypeInfo)]
	struct XCMSession<T: Config> {
		asset: MultiAsset,
		fee: MultiAsset,
		origin: MultiLocation,
		dest_location: MultiLocation,
		beneficiary: MultiLocation,
		dest_weight: Weight,
		_marker: PhantomData<T>,
	}

	impl<T: Config> XCMSession<T> {
		fn kind(&self) -> Option<TransferType> {
			ConcrateAsset::origin(&self.asset).map(|asset_reserve_location| {
				if T::NativeAssetChecker::is_native_asset_location(&asset_reserve_location) {
					TransferType::FromNative
				} else if asset_reserve_location == self.dest_location {
					TransferType::ToReserve
				} else {
					TransferType::ToNonReserve
				}
			})
		}

		// The buy execution xcm instructions always executed on the relative dest chain,
		// so when xcm instructions forwarded between different chains, the path should be
		// inverted.
		fn buy_execution_on(
			&self,
			location: &MultiLocation,
		) -> Result<Instruction<()>, DispatchError> {
			let ancestry = T::LocationInverter::ancestry();

			let fees = self
				.fee
				.clone()
				.reanchored(&location, &ancestry)
				.map_err(|_| Error::<T>::CannotReanchor)?;

			Ok(BuyExecution {
				fees,
				weight_limit: WeightLimit::Limited(self.dest_weight),
			})
		}

		/// For relaychain, parachian location always have 0 `parents`, that means
		/// when we transfer assets that reserved in relaychain to parachains, we need
		/// reanchore the dest location, e.g. set `parents` field to 0.
		fn invert_based_reserve(
			&self,
			reserve: MultiLocation,
			location: MultiLocation,
		) -> MultiLocation {
			if reserve == MultiLocation::parent() {
				(0, location.interior().clone()).into()
			} else {
				location
			}
		}
	}

	/// Xcm message handler, generate a session that can execute xcm message and track its execution result
	pub trait MessageHandler<T: Config> {
		fn message(&self) -> Result<Xcm<T::Call>, DispatchError>;
		fn execute(&self, message: &mut Xcm<T::Call>) -> DispatchResult;
	}

	impl<T: Config> MessageHandler<T> for XCMSession<T>
	where
		T::AccountId: Into<[u8; 32]> + From<[u8; 32]>,
	{
		fn execute(&self, message: &mut Xcm<T::Call>) -> DispatchResult {
			let weight =
				T::Weigher::weight(message).map_err(|()| Error::<T>::UnweighableMessage)?;
			T::XcmExecutor::execute_xcm_in_credit(
				self.origin.clone(),
				message.clone(),
				weight,
				weight,
			)
			.ensure_complete()
			.map_err(|_| Error::<T>::ExecutionFailed)?;
			Ok(())
		}

		fn message(&self) -> Result<Xcm<T::Call>, DispatchError> {
			// If self.asset.id == self.fee.id, self.asset must contains self.fee
			let (withdraw_asset, max_assets) = if self.asset.contains(&self.fee) {
				// The assets to pay the fee is the same as the main assets. Only one withdraw is required.
				(WithdrawAsset(self.asset.clone().into()), 1)
			} else {
				(
					WithdrawAsset(vec![self.asset.clone(), self.fee.clone()].into()),
					2,
				)
			};

			let deposit_asset = DepositAsset {
				assets: Wild(All),
				max_assets,
				beneficiary: self.beneficiary.clone(),
			};

			let kind = self.kind().ok_or(Error::<T>::UnknownTransferType)?;
			log::trace!(target: LOG_TARGET, "Transfer type is {:?}.", kind.clone(),);
			let message = match kind {
				TransferType::FromNative => Xcm(vec![
					withdraw_asset,
					DepositReserveAsset {
						assets: Wild(All),
						max_assets,
						dest: self.dest_location.clone(),
						xcm: Xcm(vec![
							self.buy_execution_on(&self.dest_location)?,
							deposit_asset,
						]),
					},
				]),
				TransferType::ToReserve => {
					let asset_reserve_location = self.dest_location.clone();
					Xcm(vec![
						withdraw_asset,
						InitiateReserveWithdraw {
							assets: Wild(All),
							reserve: asset_reserve_location,
							xcm: Xcm(vec![
								self.buy_execution_on(&self.dest_location)?,
								deposit_asset,
							]),
						},
					])
				}
				TransferType::ToNonReserve => {
					if let Some(asset_reserve_location) = ConcrateAsset::origin(&self.asset) {
						Xcm(vec![
							withdraw_asset,
							InitiateReserveWithdraw {
								assets: Wild(All),
								reserve: asset_reserve_location.clone(),
								xcm: Xcm(vec![
									self.buy_execution_on(&asset_reserve_location)?,
									DepositReserveAsset {
										assets: Wild(All),
										max_assets,
										dest: self.invert_based_reserve(
											asset_reserve_location.clone(),
											self.dest_location.clone(),
										),
										xcm: Xcm(vec![
											self.buy_execution_on(&self.dest_location)?,
											deposit_asset,
										]),
									},
								]),
							},
						])
					} else {
						return Err(Error::<T>::AssetNotFound.into());
					}
				}
			};
			Ok(message)
		}
	}

	impl<T: Config> Pallet<T> {
		// Return dest chain and beneficiary on dest chain
		pub fn extract_dest(dest: &MultiLocation) -> Option<(MultiLocation, MultiLocation)> {
			if dest.is_here() {
				None
			} else {
				match dest.last() {
					Some(_) => {
						let mut dest_location = dest.clone();
						let beneficiary = dest_location.take_last().unwrap().into();
						Some((dest_location, beneficiary))
					}
					_ => None,
				}
			}
		}

		fn extract_fungible(asset: MultiAsset) -> Option<(MultiLocation, u128)> {
			match (asset.fun, asset.id) {
				(Fungible(amount), Concrete(location)) => Some((location, amount)),
				_ => None,
			}
		}
	}

	impl<T: Config> BridgeChecker for Pallet<T>
	where
		<T as frame_system::Config>::AccountId: From<[u8; 32]> + Into<[u8; 32]>,
	{
		fn can_deposit_asset(asset: MultiAsset, dest: MultiLocation) -> bool {
			// Only support transfer to non-local destination.
			if dest.parents == 0 {
				return false;
			}

			if let Some((asset_location, _)) = Self::extract_fungible(asset.clone()) {
				// Reject all non-native assets that are not registered in the registry
				if !T::NativeAssetChecker::is_native_asset(&asset)
					&& T::AssetsRegistry::id(&asset_location) == None
				{
					return false;
				}
			} else {
				return false;
			}

			true
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
		<T as frame_system::Config>::AccountId: From<[u8; 32]> + Into<[u8; 32]>,
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
			max_weight: Option<Weight>,
		) -> DispatchResult {
			log::trace!(
				target: LOG_TARGET,
				"Xcm fungible transfer, sender: {:?}, asset: {:?}, dest: {:?}, max_weight: {:?}.",
				sender,
				&asset,
				&dest,
				max_weight,
			);
			// Check if we can deposit asset into dest.
			ensure!(
				Pallet::<T>::can_deposit_asset(asset.clone(), dest.clone()),
				Error::<T>::CannotDepositAsset
			);

			let origin_location = Junction::AccountId32 {
				network: NetworkId::Any,
				id: sender,
			}
			.into();
			let (dest_location, beneficiary) =
				Pallet::<T>::extract_dest(&dest).ok_or(Error::<T>::IllegalDestination)?;

			// We can not grantee asset has been supported by destination
			let fee = asset.clone();

			let xcm_session = XCMSession::<T> {
				asset: asset.clone(),
				fee,
				origin: origin_location.clone(),
				dest_location,
				beneficiary,
				dest_weight: max_weight.unwrap_or(6_000_000_000u64.into()),
				_marker: PhantomData,
			};
			let mut msg = xcm_session.message()?;
			log::trace!(
				target: LOG_TARGET,
				"Trying to exectute xcm message {:?}.",
				msg.clone(),
			);

			xcm_session.execute(&mut msg)?;

			Pallet::<T>::deposit_event(Event::AssetTransfered {
				asset,
				origin: origin_location,
				dest,
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

	#[cfg(test)]
	mod test {
		use super::*;
		use crate::mock::para::Runtime;
		use crate::mock::{
			para, ParaA, ParaAssets as Assets, ParaAssetsRegistry as AssetsRegistry, ParaB,
			ParaBalances, TestNet, ALICE, BOB, ENDOWED_BALANCE,
		};
		use frame_support::assert_ok;
		use polkadot_parachain::primitives::Sibling;
		use sp_runtime::traits::AccountIdConversion;
		use sp_runtime::AccountId32;

		use assets_registry::AssetProperties;
		use xcm::latest::MultiLocation;
		use xcm_simulator::TestExt;

		fn sibling_account(para_id: u32) -> AccountId32 {
			Sibling::from(para_id).into_account()
		}

		#[test]
		fn test_transfer_native_to_parachain() {
			TestNet::reset();

			let para_a_location = MultiLocation {
				parents: 1,
				interior: X1(Parachain(1)),
			};

			ParaB::execute_with(|| {
				// ParaB register the native asset of paraA
				assert_ok!(AssetsRegistry::force_register_asset(
					para::Origin::root(),
					para_a_location.clone().into(),
					0,
					AssetProperties {
						name: b"ParaAAsset".to_vec(),
						symbol: b"PAA".to_vec(),
						decimals: 12,
					},
				));
				// ParaB set price of the native asset of paraA
				assert_ok!(AssetsRegistry::force_set_price(para::Origin::root(), 0, 1,));
			});

			ParaA::execute_with(|| {
				let bridge_impl = BridgeTransactImpl::<Runtime>::new();
				// ParaA send it's own native asset to paraB
				assert_ok!(bridge_impl.transfer_fungible(
					ALICE.into(),
					(Concrete(MultiLocation::new(0, Here)), Fungible(10u128)).into(),
					MultiLocation::new(
						1,
						X2(
							Parachain(2u32.into()),
							Junction::AccountId32 {
								network: NetworkId::Any,
								id: BOB.into()
							}
						)
					),
					Some(1),
				));

				assert_eq!(ParaBalances::free_balance(&ALICE), ENDOWED_BALANCE - 10);
				assert_eq!(ParaBalances::free_balance(&sibling_account(2)), 10);
			});

			ParaB::execute_with(|| {
				assert_eq!(Assets::balance(0u32.into(), &BOB), 10 - 1);
			});
		}

		#[test]
		fn test_transfer_to_reserve_parachain() {
			TestNet::reset();

			let para_a_location = MultiLocation {
				parents: 1,
				interior: X1(Parachain(1)),
			};

			ParaB::execute_with(|| {
				// ParaB register the native asset of paraA
				assert_ok!(AssetsRegistry::force_register_asset(
					para::Origin::root(),
					para_a_location.clone().into(),
					0,
					AssetProperties {
						name: b"ParaAAsset".to_vec(),
						symbol: b"PAA".to_vec(),
						decimals: 12,
					},
				));
				// ParaB set price of the native asset of paraA
				assert_ok!(AssetsRegistry::force_set_price(para::Origin::root(), 0, 1,));
			});

			ParaA::execute_with(|| {
				let bridge_impl = BridgeTransactImpl::<Runtime>::new();
				// ParaA send it's own native asset to paraB
				assert_ok!(bridge_impl.transfer_fungible(
					ALICE.into(),
					(Concrete(MultiLocation::new(0, Here)), Fungible(10u128)).into(),
					MultiLocation::new(
						1,
						X2(
							Parachain(2u32.into()),
							Junction::AccountId32 {
								network: NetworkId::Any,
								id: BOB.into()
							}
						)
					),
					Some(1),
				));

				assert_eq!(ParaBalances::free_balance(&ALICE), ENDOWED_BALANCE - 10);
				assert_eq!(ParaBalances::free_balance(&sibling_account(2)), 10);
			});

			ParaB::execute_with(|| {
				assert_eq!(Assets::balance(0u32.into(), &BOB), 10 - 1);
			});

			// Now, let's transfer back to paraA
			ParaB::execute_with(|| {
				let bridge_impl = BridgeTransactImpl::<Runtime>::new();
				// ParaB send back ParaA's native asset
				assert_ok!(bridge_impl.transfer_fungible(
					BOB.into(),
					(Concrete(para_a_location.clone()), Fungible(5u128)).into(),
					MultiLocation::new(
						1,
						X2(
							Parachain(1u32.into()),
							Junction::AccountId32 {
								network: NetworkId::Any,
								id: ALICE.into()
							}
						)
					),
					Some(1),
				));

				assert_eq!(Assets::balance(0u32.into(), &BOB), 9 - 5);
			});

			ParaA::execute_with(|| {
				assert_eq!(ParaBalances::free_balance(&sibling_account(2)), 5);
				assert_eq!(ParaBalances::free_balance(&ALICE), ENDOWED_BALANCE - 10 + 4);
			});
		}

		#[test]
		fn test_transfer_to_nonreserve_parachain() {}
	}
}
