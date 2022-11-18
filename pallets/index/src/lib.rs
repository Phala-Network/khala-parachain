#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(test)]
mod mock;
pub mod types;

pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
	use crate::types::{Task, TaskId};
	use codec::{Decode, Encode};
	use frame_support::{
		dispatch::DispatchResult, pallet_prelude::*, traits::StorageVersion, transactional,
	};
	use frame_system::pallet_prelude::*;
	use sp_std::{collections::vec_deque::VecDeque, vec, vec::Vec};

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	#[pallet::storage_version(STORAGE_VERSION)]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: frame_system::Config {
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
		type CommitteeOrigin: EnsureOrigin<Self::RuntimeOrigin>;
	}

	const STORAGE_VERSION: StorageVersion = StorageVersion::new(0);
	const LOG_TARGET: &str = "runtime::pallet-index";

	/// Pre-set index executor account
	#[pallet::storage]
	#[pallet::getter(fn executor)]
	pub type Executor<T: Config> = StorageValue<_, T::AccountId>;

	/// Mapping tak_id to the full task data
	#[pallet::storage]
	#[pallet::getter(fn tasks)]
	pub type Tasks<T: Config> = StorageMap<_, Twox64Concat, TaskId, Task>;

	/// Mapping the sender's address on the source chain to the history of tasks related to him
	#[pallet::storage]
	#[pallet::getter(fn account_tasks)]
	pub type AccountTasks<T: Config> =
		StorageMap<_, Twox64Concat, Vec<u8>, Vec<TaskId>, ValueQuery>;

	/// Queue that contains all unfinished tasks belongs to the worker.
	/// worker_account => task_queue
	#[pallet::storage]
	#[pallet::getter(fn pending_tasks)]
	pub type RunningTasks<T: Config> =
		StorageMap<_, Twox64Concat, T::AccountId, VecDeque<TaskId>, ValueQuery>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// Executor is set.
		ExecutorSet { executor: T::AccountId },
		/// Task has been updated.
		TaskUpdated {
			/// Encoded task
			task: Vec<u8>,
		},
	}

	#[pallet::error]
	pub enum Error<T> {
		ExecutorMismatch,
		TaskInvalid,
	}

	#[pallet::call]
	impl<T: Config> Pallet<T>
	where
		T::AccountId: Into<[u8; 32]> + From<[u8; 32]>,
	{
		#[pallet::weight(195_000_000)]
		#[transactional]
		pub fn force_set_executor(origin: OriginFor<T>, executor: T::AccountId) -> DispatchResult {
			T::CommitteeOrigin::ensure_origin(origin)?;
			Executor::<T>::set(Some(executor.clone()));
			Self::deposit_event(Event::ExecutorSet { executor });
			Ok(())
		}

		#[pallet::weight(195_000_000)]
		#[transactional]
		pub fn update_task(origin: OriginFor<T>, task: Task) -> DispatchResult {
			// Check origin, must be the executor
			Self::ensure_executor(origin)?;

			if let Some(old_task) = Tasks::<T>::get(&task.id) {
				ensure!(Self::verify_task(&old_task, &task), Error::<T>::TaskInvalid);
				if old_task.encode() == task.encode() {
					return Ok(());
				}
			} else {
				// Save to corresponding worker and sender task queue.
				let mut account_task_queue = AccountTasks::<T>::get(&task.sender);
				account_task_queue.push(task.id.clone());
				AccountTasks::<T>::insert(&task.sender, &account_task_queue);
				let worker_account: T::AccountId = task.worker.into();
				let mut worker_task_queue = RunningTasks::<T>::get(&worker_account);
				worker_task_queue.push_back(task.id.clone());
				RunningTasks::<T>::insert(&worker_account, &worker_task_queue);
			}
			Tasks::<T>::insert(&task.id, &task);

			Self::deposit_event(Event::TaskUpdated {
				task: task.encode(),
			});
			Ok(())
		}
	}

	impl<T: Config> Pallet<T> {
		fn ensure_executor(origin: OriginFor<T>) -> DispatchResult {
			let sender = ensure_signed(origin)?;
			ensure!(
				Executor::<T>::get() == Some(sender),
				Error::<T>::ExecutorMismatch
			);
			Ok(())
		}

		fn verify_task(old_task: &Task, task: &Task) -> bool {
			(
				old_task.id,
				&old_task.sender,
				&old_task.worker,
				&old_task.recipient,
			)
				.encode() == (task.id, &task.sender, &task.worker, &task.recipient).encode()
		}
	}

	#[cfg(test)]
	mod tests {
		use crate as pallet_index;
		use crate::{AccountTasks, Event as PalletIndexEvent, Executor, RunningTasks};
		use codec::Encode;
		use frame_support::{assert_noop, assert_ok, sp_runtime::traits::BadOrigin};
		use pallet_index::mock::{
			assert_events, new_test_ext, PalletIndex, RuntimeEvent as Event,
			RuntimeOrigin as Origin, Test, ALICE, BOB,
		};
		use pallet_index::types::{Task, TaskStatus};

		#[test]
		fn test_set_executor_should_work() {
			new_test_ext().execute_with(|| {
				assert_ok!(PalletIndex::force_set_executor(Origin::root(), BOB.clone()));
				assert_eq!(Executor::<Test>::get(), Some(BOB.clone()));
				assert_events(vec![Event::PalletIndex(PalletIndexEvent::ExecutorSet {
					executor: BOB,
				})]);
			});
		}

		#[test]
		fn test_updata_task_should_work() {
			new_test_ext().execute_with(|| {
				let mut task = Task {
					id: [1; 32],
					worker: [2; 32],
					status: TaskStatus::Initialized,
					edges: vec![],
					sender: vec![3],
					recipient: vec![4],
				};
				assert_ok!(PalletIndex::force_set_executor(Origin::root(), BOB.clone()));
				assert_ok!(PalletIndex::update_task(
					Origin::signed(BOB.into()),
					task.clone()
				));
				assert_eq!(AccountTasks::<Test>::get(&task.sender), vec![task.id]);
				let worker_account: <Test as frame_system::Config>::AccountId = task.worker.into();
				assert_eq!(RunningTasks::<Test>::get(&worker_account)[0], task.id);
				assert_events(vec![Event::PalletIndex(PalletIndexEvent::TaskUpdated {
					task: task.encode(),
				})]);

				// Update task
				task.status = TaskStatus::Completed;
				assert_ok!(PalletIndex::update_task(
					Origin::signed(BOB.into()),
					task.clone()
				));
				assert_events(vec![Event::PalletIndex(PalletIndexEvent::TaskUpdated {
					task: task.encode(),
				})]);
			});
		}
	}
}
