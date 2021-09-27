#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;
#[frame_support::pallet]
pub mod pallet {
	use frame_support::{
		pallet_prelude::*,
		traits::{Currency, ExistenceRequirement, StorageVersion},
		weights::Weight,
	};
	use frame_system::pallet_prelude::*;

	use cumulus_pallet_xcm::{ensure_sibling_para, Origin as CumulusOrigin};
	use cumulus_primitives_core::ParaId;
	use frame_system::Config as SystemConfig;
	use sp_runtime::traits::{Convert, Saturating};
	use sp_std::{prelude::*, vec};

	use xcm::{
		v1::prelude::*, Version as XcmVersion, VersionedMultiAssets, VersionedMultiLocation,
		VersionedXcm,
	};
	use xcm_executor::traits::{
		ConvertOrigin, InvertLocation, VersionChangeNotifier, WeightBounds,
	};

	const STORAGE_VERSION: StorageVersion = StorageVersion::new(0);

	#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug)]
	pub enum XCMTransferType {
		/// Transfer reserve asset to other location
		TransferReserveAsset,
		/// Transfer assets to its reserve location
		TransferToReserve,
		/// Transfer assets to non-reserve location
		TransferToNonReserve,
	}

	/// XCM message infos provide for XCM sender and executor
	/// TODO.wf XCM message lifecycle management
	pub trait XCMMessageInfo<Call> {
		/// Return XCM message transfer type.
		fn kind() -> Option<XCMTransferType>;
		/// Return generated XCM message, could be executed directly.
		fn message() -> Option<Xcm<Call>>;
		/// Return weights would be cost by this XCM message.
		fn weight() -> Option<Weight>;
	}

	type BalanceOf<T> =
		<<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	#[pallet::storage_version(STORAGE_VERSION)]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: frame_system::Config {
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
	}

	/// Mapping asset name to corresponding MultiAsset
	#[pallet::storage]
	#[pallet::getter(fn registered_assets)]
	pub type RegisteredAssets<T: Config> = StorageMap<_, Twox64Concat, Vec<u8>, MultiAsset>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	#[pallet::metadata(T::BlockNumber = "BlockNumber", T::AccountId = "AccountId", BalanceOf<T> = "Balance")]
	pub enum Event<T: Config> {
		/// Send PHA to other chains. \[from, paraId, to, amount, outcome\]
		ReserveAssetSent(
			T::AccountId,
			ParaId,
			T::AccountId,
			BalanceOf<T>,
			xcm::v1::Outcome,
		),
		/// Received PHA from other chains. \[from, paraId, to, amount\]
		ReserveAssetReceived(
			T::AccountId,
			ParaId,
			T::AccountId,
			BalanceOf<T>,
			xcm::v1::Outcome,
		),
	}

	#[pallet::error]
	pub enum Error<T> {
		UnknownError,
		CannotReanchor,
		UnweighableMessage,
		FeePaymentEmpty,
	}

	#[pallet::call]
	impl<T: Config> Pallet<T>
	where
		T::AccountId: Into<[u8; 32]>,
		BalanceOf<T>: Into<u128>,
	{
		#[pallet::weight(0)]
		pub fn transfer_reserve(
			origin: OriginFor<T>,
			para_id: ParaId,
			recipient: T::AccountId,
            amount: BalanceOf<T>,
            fee: BalanceOf<T>,
		) -> DispatchResult {
			let origin_location = T::ExecuteXcmOrigin::ensure_origin(origin.clone())?;
			let who = ensure_signed(origin)?;
			let assets: MultiAssets = (Here, amount.into()).into();
            let max_assets = assets.len() as u32;
            let dest = MultiLocation { parents: 1, interior: X1(Parachain(para_id.into())) };
			let dest_weight = 6_000_000_000u64.into();
			let beneficiary: MultiLocation = Junction::AccountId32 {
				network: NetworkId::Any,
				id: recipient.clone().into(),
			}
			.into();

			let fees = (Here, fee.into()).into();

			let mut message = Xcm::TransferReserveAsset {
				assets,
				dest,
				effects: vec![
					BuyExecution {
						fees,
						// Zero weight for additional instructions/orders (since there are none to execute)
						weight: 0,
						debt: dest_weight, // covers this, `TransferReserveAsset` xcm, and `DepositAsset` order.
						halt_on_error: false,
						instructions: vec![],
					},
					DepositAsset {
						assets: Wild(All),
						max_assets,
						beneficiary: beneficiary.into(),
					},
				],
			};
			let weight =
				T::Weigher::weight(&mut message).map_err(|()| Error::<T>::UnweighableMessage)?;
			let outcome =
                T::XcmExecutor::execute_xcm_in_credit(origin_location, message, weight, weight);
            
			Self::deposit_event(Event::ReserveAssetSent(
				who, para_id, recipient, amount, outcome,
			));

			Ok(())
		}

		#[pallet::weight(0)]
		pub fn transfer(
			origin: OriginFor<T>,
			name: Vec<u8>,
			para_id: ParaId,
			recipient: T::AccountId,
			amount: BalanceOf<T>,
		) -> DispatchResult {
			// TODO.wf
			Ok(())
		}

		#[pallet::weight(0)]
		pub fn register(origin: OriginFor<T>, name: Vec<u8>, asset: MultiAsset) -> DispatchResult {
			// TODO.wf
			Ok(())
		}
	}

	impl<T: Config> Pallet<T> {
		pub fn create_message(
			from: T::AccountId,
			name: Vec<u8>,
			para_id: ParaId,
			recipient: T::AccountId,
			amount: BalanceOf<T>,
		) -> Option<XCMMessage<T::AccountId, T::Call>> {
			// TODO.wf
			None
		}
	}

	impl<AccountId, Call> XCMMessageInfo<Call> for XCMMessage<AccountId, Call> {
		fn kind() -> Option<XCMTransferType> {
			// TODO.wf
			None
		}

		fn message() -> Option<Xcm<Call>> {
			// TODO.wf
			None
		}

		fn weight() -> Option<Weight> {
			// TODO.wf
			None
		}
	}

	/// Instance of a XCM message.
	#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug)]
	pub struct XCMMessage<AccountId, Call> {
		from: AccountId,
		asset: MultiAsset,
		dest: MultiLocation,
		message: Option<Xcm<Call>>,
	}
}
