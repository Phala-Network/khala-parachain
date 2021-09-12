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
	use sp_runtime::traits::Saturating;
	use sp_std::{prelude::*, vec};
	use xcm::latest::{Error as XcmError, ExecuteXcm, MultiLocation, MultiAsset, Junction, OriginKind, SendXcm, Xcm};
	
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

		type Origin: From<<Self as SystemConfig>::Origin>
			+ Into<Result<CumulusOrigin, <Self as Config>::Origin>>;

		type Currency: Currency<Self::AccountId>;

		type XcmSender: SendXcm;

		type XcmExecutor: ExecuteXcm<Self::Call>;
	}

	/// Mapping asset name to corresponding MultiAsset
	#[pallet::storage]
	#[pallet::getter(fn registered_assets)]
	pub type RegisteredAssets<T: Config> = StorageMap<_, Twox64Concat, Vec<u8>, MultiAsset>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	#[pallet::metadata(T::BlockNumber = "BlockNumber", T::AccountId = "AccountId", BalanceOf<T> = "Balance")]
	pub enum Event<T: Config> {
		/// Send PHA to other chains. \[paraId, from, to, amount\]
		ReserveAssetSent(ParaId, T::AccountId, T::AccountId, BalanceOf<T>),
		/// Received PHA from other chains. \[paraId, from, to, amount\]
		ReserveAssetReceived(ParaId, T::AccountId, T::AccountId, BalanceOf<T>),
	}

	#[pallet::error]
	pub enum Error<T> {
		UnknownError,
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::weight(0)]
		pub fn transfer(
			origin: OriginFor<T>,
			name: Vec<u8>,
			paraId: ParaId,
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
			paraId: ParaId,
			recipient: T::AccountId,
			amount: BalanceOf<T>,
		) -> Option<XCMMessage<T::AccountId, T::Call>> {
			// TODO.wf
			None
		}
	}

	impl<AccountId, Call> XCMMessageInfo<Call> for XCMMessage<AccountId, Call>
	{
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
		message: Option<Xcm<Call>>
	}
}
