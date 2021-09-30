pub use self::pallet::*;

#[allow(unused_variables)]
#[frame_support::pallet]
pub mod pallet {
	use crate::assets as xtransfer_assets;
	use cumulus_primitives_core::ParaId;
	use frame_support::{
		pallet_prelude::*,
		traits::{Currency, StorageVersion},
		weights::Weight,
	};
	use frame_system::pallet_prelude::*;
	use sp_std::{convert::TryInto, prelude::*, vec};
	use xcm::v1::{
		prelude::*, AssetId::Concrete, Fungibility::Fungible, MultiAsset, MultiLocation,
	};
	use xcm_executor::traits::{InvertLocation, WeightBounds};

	/// The logging target.
	const LOG_TARGET: &str = "xcm-transfer";
	const STORAGE_VERSION: StorageVersion = StorageVersion::new(0);

	type BalanceOf<T> =
		<<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	#[pallet::storage_version(STORAGE_VERSION)]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: frame_system::Config + xtransfer_assets::Config {
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

		type Currency: Currency<Self::AccountId>;

		/// Required origin for sending XCM messages. If successful, the it resolves to `MultiLocation`
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

		/// ParachainID
		type ParachainInfo: Get<ParaId>;
	}

	/// Mapping asset name to corresponding MultiAsset
	#[pallet::storage]
	#[pallet::getter(fn registered_assets)]
	pub type RegisteredAssets<T: Config> = StorageMap<_, Twox64Concat, Vec<u8>, MultiAsset>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	#[pallet::metadata(T::BlockNumber = "BlockNumber", T::AccountId = "AccountId", BalanceOf<T> = "Balance")]
	pub enum Event<T: Config> {
		/// Assets sent to parachain or relaychain. \[from, paraId, to, amount\]
		AssetTransfered(T::AccountId, ParaId, T::AccountId, BalanceOf<T>),
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
		AssetNotSupported,
	}

	#[pallet::call]
	impl<T: Config> Pallet<T>
	where
		T::AccountId: Into<[u8; 32]>,
		BalanceOf<T>: Into<u128>,
	{
		#[pallet::weight(0)]
		pub fn transfer_by_asset_identity(
			origin: OriginFor<T>,
			asset_identity: Vec<u8>,
			para_id: ParaId,
			recipient: T::AccountId,
			amount: BalanceOf<T>,
			dest_weight: Weight,
		) -> DispatchResult {
			// get asset location by identity
			let asset_info = xtransfer_assets::AssetsIdentityToInfo::<T>::get(&asset_identity)
				.ok_or(Error::<T>::AssetNotFound)?;
			let asset_location: MultiLocation = asset_info
				.try_into()
				.map_err(|_| Error::<T>::AssetNotSupported)?;
			let asset: MultiAsset = (asset_location, amount.into()).into();

			Self::do_transfer(origin, asset, para_id, recipient, amount, dest_weight)
		}

		#[pallet::weight(0)]
		pub fn transfer_by_asset_id(
			origin: OriginFor<T>,
			asset_id: xtransfer_assets::XTransferAssetId,
			para_id: ParaId,
			recipient: T::AccountId,
			amount: BalanceOf<T>,
			dest_weight: Weight,
		) -> DispatchResult {
			// get asset location by asset id
			let asset_info = xtransfer_assets::AssetIdToInfo::<T>::get(&asset_id)
				.ok_or(Error::<T>::AssetNotFound)?;
			let asset_location: MultiLocation = asset_info
				.try_into()
				.map_err(|_| Error::<T>::AssetNotSupported)?;
			let asset: MultiAsset = (asset_location, amount.into()).into();

			Self::do_transfer(origin, asset, para_id, recipient, amount, dest_weight)
		}
	}

	impl<T: Config> Pallet<T>
	where
		T::AccountId: Into<[u8; 32]>,
		BalanceOf<T>: Into<u128>,
	{
		pub fn do_transfer(
			origin: OriginFor<T>,
			asset: MultiAsset,
			para_id: ParaId,
			recipient: T::AccountId,
			amount: BalanceOf<T>,
			dest_weight: Weight,
		) -> DispatchResult {
			sp_runtime::runtime_logger::RuntimeLogger::init();

			let sender = ensure_signed(origin.clone())?;
			let origin_location = T::ExecuteXcmOrigin::ensure_origin(origin)?;
			let dest_location = MultiLocation {
				parents: 1,
				interior: X1(Parachain(para_id.into())),
			};

			let xcm_session = XCMSession::<T> {
				asset,
				origin_location,
				dest_location,
				sender: sender.clone(),
				recipient: recipient.clone(),
				dest_weight,
			};
			let mut msg = xcm_session.message()?;
			log::error!(
				target: LOG_TARGET,
				"Trying to exectute xcm message {:?}.",
				msg.clone(),
			);
			xcm_session.execute(&mut msg)?;

			Self::deposit_event(Event::AssetTransfered(sender, para_id, recipient, amount));

			Ok(())
		}
	}
	#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug)]
	pub enum TransferType {
		/// Transfer assets that reserved by origin chain
		FromNative,
		/// Transfer assets that reserved by dest chain
		ToReserve,
		/// Transfer assets that nont  reserved by dest chain
		ToNonReserve,
	}

	pub trait MessageHandler<T: Config> {
		fn message(&self) -> Result<Xcm<T::Call>, DispatchError>;
		fn execute(&self, message: &mut Xcm<T::Call>) -> DispatchResult;
	}

	#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug)]
	struct XCMSession<T: Config> {
		asset: MultiAsset,
		origin_location: MultiLocation,
		dest_location: MultiLocation,
		sender: T::AccountId,
		recipient: T::AccountId,
		dest_weight: Weight,
	}

	impl<T: Config> XCMSession<T> {
		fn reserve(&self) -> Option<MultiLocation> {
			match (&self.asset.id, &self.asset.fun) {
				(Concrete(ref id), Fungible(ref amount)) => Some(id.clone()),
				_ => None,
			}
		}

		fn kind(&self) -> Option<TransferType> {
			let reserve_locations = [
				MultiLocation {
					parents: 0,
					interior: Here,
				},
				MultiLocation {
					parents: 1,
					interior: X1(Parachain(T::ParachainInfo::get().into())),
				},
			];
			match self.reserve() {
				Some(asset_reserve_location) => {
					if reserve_locations.contains(&asset_reserve_location) {
						Some(TransferType::FromNative)
					} else if asset_reserve_location == self.dest_location {
						Some(TransferType::ToReserve)
					} else {
						Some(TransferType::ToNonReserve)
					}
				}
				None => None,
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
				self.origin_location.clone(),
				message.clone(),
				weight,
				weight,
			)
			.ensure_complete()
			.map_err(|_| Error::<T>::ExecutionFailed)?;
			Ok(())
		}

		fn message(&self) -> Result<Xcm<T::Call>, DispatchError> {
			let inv_dest = T::LocationInverter::invert_location(&self.dest_location);
			let fees = self
				.asset
				.clone()
				.reanchored(&inv_dest)
				.map_err(|_| Error::<T>::CannotReanchor)?;
			let beneficiary: MultiLocation = Junction::AccountId32 {
				network: NetworkId::Any,
				id: self.recipient.clone().into(),
			}
			.into();

			let buy_execution = BuyExecution {
				fees,
				// Zero weight for additional instructions/orders (since there are none to execute)
				weight: 0,
				debt: self.dest_weight, // covers this, `TransferReserveAsset` xcm, and `DepositAsset` order.
				halt_on_error: false,
				instructions: vec![],
			};

			let deposit_asset = DepositAsset {
				assets: Wild(All),
				max_assets: 1u32,
				beneficiary: beneficiary.into(),
			};

			let kind = self.kind().ok_or(Error::<T>::UnknownTransfer)?;
			log::error!(target: LOG_TARGET, "Transfer type is {:?}.", kind.clone(),);
			let message = match kind {
				TransferType::FromNative => Xcm::TransferReserveAsset {
					assets: self.asset.clone().into(),
					dest: self.dest_location.clone(),
					effects: vec![buy_execution.clone(), deposit_asset],
				},
				TransferType::ToReserve => {
					let asset_reserve_location = self.dest_location.clone();
					WithdrawAsset {
						assets: self.asset.clone().into(),
						effects: vec![InitiateReserveWithdraw {
							assets: self.asset.clone().into(),
							reserve: asset_reserve_location,
							effects: vec![buy_execution, deposit_asset],
						}],
					}
				}
				TransferType::ToNonReserve => {
					let asset_reserve_location = self.reserve().unwrap();
					WithdrawAsset {
						assets: self.asset.clone().into(),
						effects: vec![InitiateReserveWithdraw {
							assets: self.asset.clone().into(),
							reserve: asset_reserve_location,
							effects: vec![
								buy_execution.clone(),
								DepositReserveAsset {
									assets: self.asset.clone().into(),
									max_assets: 1u32,
									dest: self.dest_location.clone(),
									effects: vec![buy_execution, deposit_asset],
								},
							],
						}],
					}
				}
			};
			Ok(message)
		}
	}
}
