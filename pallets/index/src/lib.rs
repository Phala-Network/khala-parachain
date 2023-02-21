#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(test)]
mod mock;

pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
	use codec::Encode;
	use frame_support::{
		dispatch::DispatchResult, pallet_prelude::*, traits::StorageVersion, transactional,
	};
	use frame_system::pallet_prelude::*;
	use sp_std::{collections::vec_deque::VecDeque, vec, vec::Vec};
	use xcm::latest::{
		prelude::*, AssetId as XcmAssetId, Fungibility::Fungible, MultiAsset, MultiLocation,
		Weight as XCMWeight,
	};
	use xcm_executor::traits::TransactAsset;

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	#[pallet::storage_version(STORAGE_VERSION)]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(_);

	pub type RequestId = [u8; 32];

	#[derive(Clone, Decode, Encode, Eq, PartialEq, Ord, PartialOrd, Debug, TypeInfo)]
	pub struct DepositInfo {
		/// Sender address on source chain
		pub sender: [u8; 32],
		/// Deposit asset
		pub asset: XcmAssetId,
		/// Deposit amount
		pub amount: u128,
		/// Recipient address on dest chain
		pub recipient: Vec<u8>,
		/// Encoded execution plan produced by Solver
		pub request: Vec<u8>,
	}

	#[pallet::config]
	pub trait Config: frame_system::Config {
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
		type CommitteeOrigin: EnsureOrigin<Self::RuntimeOrigin>;
	}

	const STORAGE_VERSION: StorageVersion = StorageVersion::new(0);

	/// Pre-set index worker sr25519 pubkey
	#[pallet::storage]
	#[pallet::getter(fn executors)]
	pub type Workers<T: Config> = StorageMap<_, Twox64Concat, [u8; 32], bool, ValueQuery>;

	/// Mapping request_id to the full deposit data
	#[pallet::storage]
	#[pallet::getter(fn deposit_records)]
	pub type DepositRecords<T: Config> = StorageMap<_, Twox64Concat, RequestId, DepositInfo>;

	/// Mapping the worker sr25519 pubkey and its actived task queue
	#[pallet::storage]
	#[pallet::getter(fn actived_requests)]
	pub type ActivedRequests<T: Config> =
		StorageMap<_, Twox64Concat, [u8; 32], VecDeque<RequestId>, ValueQuery>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// Worker is set.
		WorkerAdd { worker: T::AccountId },
		/// Worker is set.
		WorkerRemove { worker: T::AccountId },
		/// New reqeust saved.
		NewRequest {
			/// Record
			deposit_info: DepositInfo,
		},
		/// Task has been claimed.
		Claimed { requests: Vec<RequestId> },
	}

	#[pallet::error]
	pub enum Error<T> {
		WorkerAlreadySet,
		WorkerNotSet,
		WorkerMismatch,
		RequestAlreadyExist,
		NotFoundInTaskQueue,
		TaskQueueEmpty,
	}

	#[pallet::call]
	impl<T: Config> Pallet<T>
	where
		T::AccountId: Into<[u8; 32]> + From<[u8; 32]>,
	{
		#[pallet::weight(195_000_000)]
		#[pallet::call_index(0)]
		#[transactional]
		pub fn force_add_worker(origin: OriginFor<T>, worker: T::AccountId) -> DispatchResult {
			T::CommitteeOrigin::ensure_origin(origin)?;
			ensure!(
				Workers::<T>::get(&worker.clone().into()) == false,
				Error::<T>::WorkerAlreadySet
			);
			Workers::<T>::insert(&worker.clone().into(), true);
			Self::deposit_event(Event::WorkerAdd { worker });
			Ok(())
		}

		#[pallet::weight(195_000_000)]
		#[pallet::call_index(1)]
		#[transactional]
		pub fn force_remove_worker(origin: OriginFor<T>, worker: T::AccountId) -> DispatchResult {
			T::CommitteeOrigin::ensure_origin(origin)?;
			ensure!(Workers::<T>::get(&worker.clone().into()), Error::<T>::WorkerNotSet);
			Workers::<T>::insert(&worker.clone().into(), false);
			Self::deposit_event(Event::WorkerAdd { worker });
			Ok(())
		}

		#[pallet::weight(195_000_000)]
		#[pallet::call_index(2)]
		#[transactional]
		pub fn deposit_task(
			origin: OriginFor<T>,
			asset: XcmAssetId,
			amount: u128,
			recipient: Vec<u8>,
			worker: [u8; 32],
			request_id: RequestId,
			request: Vec<u8>,
		) -> DispatchResult {
			let sender: [u8; 32] = ensure_signed(origin)?.into();

			// Ensure given worker was registered
			ensure!(Workers::<T>::get(&worker), Error::<T>::WorkerNotSet);

			// TODO: Check if asset was registered on our chain

			// Check if record already exist
			ensure!(
				DepositRecords::<T>::get(&request_id).is_none(),
				Error::<T>::RequestAlreadyExist
			);

			// Save record to corresponding worker task queue
			let mut worker_task_queue = ActivedRequests::<T>::get(&worker);
			worker_task_queue.push_back(request_id);
			ActivedRequests::<T>::insert(&worker, &worker_task_queue);
			// Save record data
			let deposit_info = DepositInfo {
				sender,
				asset,
				amount,
				recipient,
				request,
			};
			DepositRecords::<T>::insert(&request_id, &deposit_info);

			// TODO: Transfer asset from user account to pallet account

			Self::deposit_event(Event::NewRequest { deposit_info });
			Ok(())
		}

		/// Claim the oldest task that saved in actived task queue
		#[pallet::weight(195_000_000)]
		#[pallet::call_index(3)]
		#[transactional]
		pub fn claim_task(origin: OriginFor<T>, request_id: RequestId) -> DispatchResult {
			// Check origin, must be the worker
			let worker: [u8; 32] = ensure_signed(origin)?.into();
			ensure!(
				Workers::<T>::get(&worker) == true,
				Error::<T>::WorkerMismatch
			);

			let mut worker_task_queue = ActivedRequests::<T>::get(&worker);
			// Check reqeust exist in actived task queue
			ensure!(
				worker_task_queue.contains(&request_id),
				Error::<T>::NotFoundInTaskQueue
			);

			// Pop the oldest task from worker task queue
			let _deposit_info = worker_task_queue
				.pop_front()
				.ok_or(Error::<T>::TaskQueueEmpty)?;
			ActivedRequests::<T>::insert(&worker, &worker_task_queue);

			// TODO: Transfer asset from pallet account to worker account

			Self::deposit_event(Event::Claimed {
				requests: vec![request_id],
			});
			Ok(())
		}

		#[pallet::weight(195_000_000)]
		#[pallet::call_index(4)]
		#[transactional]
		pub fn claim_all_task(origin: OriginFor<T>) -> DispatchResult {
			// Check origin, must be the worker
			let worker: [u8; 32] = ensure_signed(origin)?.into();
			ensure!(
				Workers::<T>::get(&worker) == true,
				Error::<T>::WorkerMismatch
			);

			let worker_task_queue = ActivedRequests::<T>::get(&worker);

			// Put an empty task queue
			ActivedRequests::<T>::insert(&worker, &VecDeque::<RequestId>::new());

			for deposit_info in worker_task_queue.iter() {
				// TODO: Transfer asset from pallet account to worker account
			}

			Self::deposit_event(Event::Claimed {
				requests: worker_task_queue.into(),
			});
			Ok(())
		}
	}

	#[cfg(test)]
	mod tests {
		use crate as pallet_index;
		use crate::{Event as PalletIndexEvent, Workers};
		use codec::Encode;
		use frame_support::{assert_noop, assert_ok};
		use pallet_index::mock::{
			assert_events, new_test_ext, PalletIndex, RuntimeEvent as Event,
			RuntimeOrigin as Origin, Test, BOB,
		};

		#[test]
		fn test_add_executor_should_work() {
			new_test_ext().execute_with(|| {
				let bob_key: [u8; 32] = BOB.into();
				assert_ok!(PalletIndex::force_add_worker(Origin::root(), BOB.clone()));
				assert_eq!(Workers::<Test>::get(bob_key), true);
				assert_events(vec![Event::PalletIndex(PalletIndexEvent::WorkerAdd {
					worker: BOB,
				})]);
				assert_ok!(PalletIndex::force_remove_worker(
					Origin::root(),
					BOB.clone()
				));
				assert_eq!(Workers::<Test>::get(bob_key), false);
			});
		}
	}
}
