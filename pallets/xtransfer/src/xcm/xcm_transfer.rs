pub use self::pallet::*;

#[allow(unused_variables)]
#[frame_support::pallet]
pub mod pallet {
	use crate::pallet_assets_wrapper;
	use crate::xcm_helper::{ConcrateAsset, NativeAssetChecker};
	use frame_support::{
		dispatch::DispatchResult,
		pallet_prelude::*,
		traits::{Currency, StorageVersion},
		weights::Weight,
		PalletId,
	};
	use frame_system::pallet_prelude::*;
	use scale_info::TypeInfo;
	use sp_runtime::{traits::AccountIdConversion, DispatchError};
	use sp_std::{convert::TryInto, prelude::*, vec};
	use xcm::latest::{prelude::*, Fungibility::Fungible, MultiAsset, MultiLocation};
	use xcm_executor::traits::{InvertLocation, WeightBounds};

	/// The logging target.
	const LOG_TARGET: &str = "runtime::xcm-transfer";
	const STORAGE_VERSION: StorageVersion = StorageVersion::new(0);

	type BalanceOf<T> =
		<<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	#[pallet::storage_version(STORAGE_VERSION)]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: frame_system::Config {
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

		/// Assets that can be used to pay xcm execution
		type FeeAssets: Get<MultiAssets>;

		/// Default xcm fee(PHA) used to buy execution on dest parachain
		type DefaultFee: Get<u128>;
	}

	/// Mapping asset name to corresponding MultiAsset
	#[pallet::storage]
	#[pallet::getter(fn registered_assets)]
	pub type RegisteredAssets<T: Config> = StorageMap<_, Twox64Concat, Vec<u8>, MultiAsset>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// Assets sent to parachain or relaychain.
		AssetTransfered {
			asset: MultiAsset,
			origin: MultiLocation,
			dest: MultiLocation,
		},
		/// Assets being deposited, including being deposited to reserve account of solo chains.
		XcmDeposited {
			what: MultiAsset,
			who: MultiLocation,
		},
		/// Assets being withdrawn
		XcmWithdrawn {
			what: MultiAsset,
			who: MultiLocation,
		},
	}

	#[pallet::error]
	pub enum Error<T> {
		UnknownError,
		CannotReanchor,
		UnweighableMessage,
		FeePaymentEmpty,
		ExecutionFailed,
		UnknownTransfer,
		AssetNotFound,
		LocationInvertFailed,
		IllegalDestination,
	}

	#[pallet::call]
	impl<T: Config> Pallet<T>
	where
		T::AccountId: Into<[u8; 32]> + From<[u8; 32]>,
		BalanceOf<T>: Into<u128>,
	{
		#[pallet::weight(195_000_000 + Pallet::<T>::estimate_transfer_weight())]
		pub fn transfer_asset(
			origin: OriginFor<T>,
			asset: Box<pallet_assets_wrapper::XTransferAsset>,
			dest: MultiLocation,
			amount: BalanceOf<T>,
			dest_weight: Weight,
		) -> DispatchResult {
			let origin = T::ExecuteXcmOrigin::ensure_origin(origin)?;
			// Get asset location by asset id
			let asset_location: MultiLocation =
				(*asset).try_into().map_err(|_| Error::<T>::AssetNotFound)?;
			let multi_asset: MultiAsset = (asset_location, amount.into()).into();

			Self::do_transfer_multiasset(origin, multi_asset, dest, dest_weight)
		}

		#[pallet::weight(195_000_000 + Pallet::<T>::estimate_transfer_weight())]
		pub fn transfer_native(
			origin: OriginFor<T>,
			dest: MultiLocation,
			amount: BalanceOf<T>,
			dest_weight: Weight,
		) -> DispatchResult {
			let origin = T::ExecuteXcmOrigin::ensure_origin(origin)?;
			let asset_location: MultiLocation = (0, Here).into();
			let asset: MultiAsset = (asset_location, amount.into()).into();

			Self::do_transfer_multiasset(origin, asset, dest, dest_weight)
		}
	}

	impl<T: Config> Pallet<T>
	where
		T::AccountId: Into<[u8; 32]> + From<[u8; 32]>,
		BalanceOf<T>: Into<u128>,
	{
		/// Returns the estimated max weight for a xcm based on non-reserve xcm transfer cost
		pub fn estimate_transfer_weight() -> Weight {
			// We treat nonreserve xcm transfer cost the most weight
			let nonreserve_xcm_transfer_session = XCMSession::<T> {
				asset: (MultiLocation::new(1, Here), Fungible(0u128)).into(),
				fee: (MultiLocation::new(1, Here), Fungible(0u128)).into(),
				origin: MultiLocation::new(1, X1(Parachain(1u32))),
				dest_location: MultiLocation::new(1, X1(Parachain(2u32))),
				recipient: PalletId(*b"phala/bg").into_account(),
				dest_weight: 0,
			};
			let mut msg = nonreserve_xcm_transfer_session
				.message()
				.expect("Xcm message must be generated; qed.");
			T::Weigher::weight(&mut msg).map_or(Weight::max_value(), |w| w)
		}

		pub fn do_transfer_multiasset(
			origin: MultiLocation,
			asset: MultiAsset,
			dest: MultiLocation,
			dest_weight: Weight,
		) -> DispatchResult {
			let mut dest_location = dest.clone();
			// Make sure we are processing crosschain transfer and we got correct path
			ensure!(!dest_location.is_here(), Error::<T>::IllegalDestination);
			// FIXME: what if someone give a Parachain junction at the end?
			// After take_last(), dest only contains reserve location of the recipient.
			let recipient = match dest_location.take_last() {
				Some(Junction::AccountId32 {
					network: _,
					id: recipient,
				}) => Some(recipient),
				_ => None,
			};
			ensure!(!recipient.is_none(), Error::<T>::IllegalDestination);

			let fee = if T::FeeAssets::get().contains(&asset)
				|| T::NativeAssetChecker::is_native_asset(&asset)
			{
				match asset.fun {
					// So far only half of amount are allowed to be used as fee
					Fungible(amount) => MultiAsset {
						fun: Fungible(amount / 2),
						id: asset.id.clone(),
					},
					// We do not support unfungible asset transfer, nor support it as fee
					_ => return Err(Error::<T>::AssetNotFound.into()),
				}
			} else {
				// Basiclly, if the asset is supported as fee in our system, it should be also supported in the dest
				// parachain, so if we are not support use this asset as fee, try use PHA as fee asset instead
				MultiAsset {
					fun: Fungible(T::DefaultFee::get()),
					id: T::NativeAssetChecker::native_asset_id().into(),
				}
			};

			let xcm_session = XCMSession::<T> {
				asset: asset.clone(),
				fee,
				origin: origin.clone(),
				dest_location,
				recipient: recipient.unwrap().into(),
				dest_weight,
			};
			let mut msg = xcm_session.message()?;
			log::trace!(
				target: LOG_TARGET,
				"Trying to exectute xcm message {:?}.",
				msg.clone(),
			);

			xcm_session.execute(&mut msg)?;

			Self::deposit_event(Event::AssetTransfered {
				asset,
				origin,
				dest,
			});

			Ok(())
		}
	}

	pub trait XcmTransact<T: frame_system::Config> {
		fn transfer_fungible(
			origin: MultiLocation,
			asset: MultiAsset,
			dest: MultiLocation,
			dest_weight: Weight,
		) -> DispatchResult;
	}

	// Impl XcmTransact trait for () only for testing purpose, would
	// be removed when release
	impl<T: frame_system::Config> XcmTransact<T> for () {
		fn transfer_fungible(
			origin: MultiLocation,
			asset: MultiAsset,
			dest: MultiLocation,
			dest_weight: Weight,
		) -> DispatchResult {
			Ok(())
		}
	}

	impl<T: Config> XcmTransact<T> for Pallet<T>
	where
		T::AccountId: From<[u8; 32]> + Into<[u8; 32]>,
		BalanceOf<T>: Into<u128>,
	{
		fn transfer_fungible(
			origin: MultiLocation,
			asset: MultiAsset,
			dest: MultiLocation,
			dest_weight: Weight,
		) -> DispatchResult {
			Self::do_transfer_multiasset(origin, asset, dest, dest_weight)
		}
	}

	pub trait XcmOnDeposited {
		fn on_deposited(what: MultiAsset, who: MultiLocation) -> DispatchResult;
	}
	impl<T: Config> XcmOnDeposited for Pallet<T> {
		fn on_deposited(what: MultiAsset, who: MultiLocation) -> DispatchResult {
			Self::deposit_event(Event::XcmDeposited { what, who });
			Ok(())
		}
	}

	pub trait XcmOnWithdrawn {
		fn on_withdrawn(what: MultiAsset, who: MultiLocation) -> DispatchResult;
	}
	impl<T: Config> XcmOnWithdrawn for Pallet<T> {
		fn on_withdrawn(what: MultiAsset, who: MultiLocation) -> DispatchResult {
			Self::deposit_event(Event::XcmWithdrawn { what, who });
			Ok(())
		}
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

	pub trait MessageHandler<T: Config> {
		fn message(&self) -> Result<Xcm<T::Call>, DispatchError>;
		fn execute(&self, message: &mut Xcm<T::Call>) -> DispatchResult;
	}

	#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, TypeInfo)]
	struct XCMSession<T: Config> {
		asset: MultiAsset,
		fee: MultiAsset,
		origin: MultiLocation,
		// Where recipient located in
		dest_location: MultiLocation,
		recipient: T::AccountId,
		dest_weight: Weight,
	}

	impl<T: Config> XCMSession<T> {
		fn kind(&self) -> Option<TransferType> {
			let mut transfer_type = None;
			ConcrateAsset::origin(&self.asset).map(|asset_reserve_location| {
				if T::NativeAssetChecker::is_native_asset_id(&asset_reserve_location) {
					transfer_type = Some(TransferType::FromNative);
				} else if asset_reserve_location == self.dest_location {
					transfer_type = Some(TransferType::ToReserve);
				} else {
					transfer_type = Some(TransferType::ToNonReserve);
				}
			});
			transfer_type
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

	impl<T: Config> MessageHandler<T> for XCMSession<T>
	where
		T::AccountId: Into<[u8; 32]>,
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
			.map_err(|e| Error::<T>::ExecutionFailed)?;
			Ok(())
		}

		fn message(&self) -> Result<Xcm<T::Call>, DispatchError> {
			let beneficiary: MultiLocation = Junction::AccountId32 {
				network: NetworkId::Any,
				id: self.recipient.clone().into(),
			}
			.into();

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
				beneficiary: beneficiary.into(),
			};

			let kind = self.kind().ok_or(Error::<T>::UnknownTransfer)?;
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
					let asset_reserve_location = ConcrateAsset::origin(&self.asset).unwrap();
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
				}
			};
			Ok(message)
		}
	}
}

#[cfg(test)]
mod test {
	use crate::xcm::mock::*;
	use frame_support::{assert_err, assert_noop, assert_ok};
	use polkadot_parachain::primitives::Sibling;
	use sp_runtime::traits::AccountIdConversion;
	use sp_runtime::{AccountId32, DispatchError};
	use sp_std::convert::TryInto;

	use xcm::latest::{prelude::*, MultiLocation};
	use xcm_simulator::TestExt;

	use crate::pallet_assets_wrapper;
	use crate::pallet_assets_wrapper::GetAssetRegistryInfo;
	use assert_matches::assert_matches;

	fn sibling_account(para_id: u32) -> AccountId32 {
		Sibling::from(para_id).into_account()
	}

	#[test]
	fn test_asset_register() {
		TestNet::reset();

		ParaA::execute_with(|| {
			// Register first asset, id = 0
			let para_a_location: MultiLocation = MultiLocation {
				parents: 1,
				interior: X1(Parachain(1)),
			};
			let para_a_asset: pallet_assets_wrapper::XTransferAsset =
				para_a_location.try_into().unwrap();

			// Should be failed if origin is from sudo user
			assert_err!(
				ParaAssetsWrapper::force_register_asset(
					Some(ALICE).into(),
					para_a_asset.clone().into(),
					0,
					ALICE,
				),
				DispatchError::BadOrigin
			);

			assert_ok!(ParaAssetsWrapper::force_register_asset(
				para::Origin::root(),
				para_a_asset.clone().into(),
				0,
				ALICE,
			));

			let ev: Vec<para::Event> = para_take_events();
			let _expected_ev: Vec<para::Event> = [pallet_assets_wrapper::Event::AssetRegistered {
				asset_id: 0u32.into(),
				asset: para_a_asset.clone(),
			}
			.into()]
			.to_vec();
			assert_matches!(ev, _expected_ev);
			assert_eq!(ParaAssetsWrapper::id(&para_a_asset).unwrap(), 0u32);
			assert_eq!(
				ParaAssetsWrapper::asset(&0u32.into()).unwrap(),
				para_a_asset
			);
			assert_eq!(ParaAssets::total_supply(0u32.into()), 0);

			// Same asset location register again, should be failed
			assert_noop!(
				ParaAssetsWrapper::force_register_asset(
					para::Origin::root(),
					para_a_asset.clone().into(),
					1,
					ALICE,
				),
				pallet_assets_wrapper::Error::<para::Runtime>::AssetAlreadyExist
			);

			let para_b_location: MultiLocation = MultiLocation {
				parents: 1,
				interior: X1(Parachain(2)),
			};
			let para_b_asset: pallet_assets_wrapper::XTransferAsset =
				para_b_location.try_into().unwrap();

			// Same asset id register again, should be failed
			assert_noop!(
				ParaAssetsWrapper::force_register_asset(
					para::Origin::root(),
					para_b_asset.clone().into(),
					0,
					ALICE,
				),
				pallet_assets_wrapper::Error::<para::Runtime>::AssetAlreadyExist
			);

			// Register another asset, id = 1
			let para_b_location: MultiLocation = MultiLocation {
				parents: 1,
				interior: X1(Parachain(2)),
			};
			let para_b_asset: pallet_assets_wrapper::XTransferAsset =
				para_b_location.try_into().unwrap();
			assert_ok!(ParaAssetsWrapper::force_register_asset(
				para::Origin::root(),
				para_b_asset.clone().into(),
				1,
				ALICE,
			));
			assert_eq!(ParaAssetsWrapper::id(&para_b_asset).unwrap(), 1u32);
			assert_eq!(
				ParaAssetsWrapper::asset(&1u32.into()).unwrap(),
				para_b_asset
			);

			// Unregister asset
			assert_ok!(ParaAssetsWrapper::force_unregister_asset(
				para::Origin::root(),
				1
			));
			assert_eq!(ParaAssetsWrapper::id(&para_b_asset), None);
			assert_eq!(ParaAssetsWrapper::asset(&1u32.into()), None);
		});
	}

	#[test]
	fn test_transfer_native_to_parachain() {
		TestNet::reset();

		let para_a_location: MultiLocation = MultiLocation {
			parents: 1,
			interior: X1(Parachain(1)),
		};
		let para_a_asset: pallet_assets_wrapper::XTransferAsset =
			para_a_location.try_into().unwrap();

		ParaB::execute_with(|| {
			// ParaB register the native asset of paraA
			assert_ok!(ParaAssetsWrapper::force_register_asset(
				para::Origin::root(),
				para_a_asset.clone().into(),
				0,
				ALICE,
			));
		});

		ParaA::execute_with(|| {
			// ParaA send it's own native asset to paraB
			assert_ok!(XcmTransfer::transfer_native(
				Some(ALICE).into(),
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
				10,
				1,
			));

			assert_eq!(ParaBalances::free_balance(&ALICE), 1_000 - 10);
			assert_eq!(ParaBalances::free_balance(&sibling_account(2)), 10);
		});

		ParaB::execute_with(|| {
			assert_eq!(ParaAssets::balance(0u32.into(), &BOB), 10 - 1);
		});
	}

	#[test]
	fn test_transfer_to_reserve_parachain() {
		TestNet::reset();

		let para_a_location: MultiLocation = MultiLocation {
			parents: 1,
			interior: X1(Parachain(1)),
		};
		let para_a_asset: pallet_assets_wrapper::XTransferAsset =
			para_a_location.try_into().unwrap();

		ParaB::execute_with(|| {
			// ParaB register the native asset of paraA
			assert_ok!(ParaAssetsWrapper::force_register_asset(
				para::Origin::root(),
				para_a_asset.clone().into(),
				0,
				ALICE,
			));
		});

		ParaA::execute_with(|| {
			// ParaA send it's own native asset to paraB
			assert_ok!(XcmTransfer::transfer_native(
				Some(ALICE).into(),
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
				10,
				1,
			));

			assert_eq!(ParaBalances::free_balance(&ALICE), 1_000 - 10);
			assert_eq!(ParaBalances::free_balance(&sibling_account(2)), 10);
		});

		ParaB::execute_with(|| {
			assert_eq!(ParaAssets::balance(0u32.into(), &BOB), 10 - 1);
		});

		// Now, let's transfer back to paraA
		ParaB::execute_with(|| {
			// ParaB send back ParaA's native asset
			assert_ok!(XcmTransfer::transfer_asset(
				Some(BOB).into(),
				Box::new(para_a_asset.clone().into()),
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
				5,
				1,
			));

			assert_eq!(ParaAssets::balance(0u32.into(), &BOB), 9 - 5);
		});

		ParaA::execute_with(|| {
			assert_eq!(ParaBalances::free_balance(&sibling_account(2)), 5);
			assert_eq!(ParaBalances::free_balance(&ALICE), 1_000 - 10 + 4);
		});
	}

	#[test]
	fn test_transfer_to_nonreserve_parachain() {}
}
