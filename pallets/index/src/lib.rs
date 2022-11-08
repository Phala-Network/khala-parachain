#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(test)]
mod mock;
pub mod types;

pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
	use codec::{Decode, Encode};
	use frame_support::{
		dispatch::DispatchResult,
		pallet_prelude::*,
		traits::StorageVersion,
		transactional,
	};
	use frame_system::pallet_prelude::*;
	use sp_std::{collections::vec_deque::VecDeque, vec, vec::Vec};
	use crate::types::{TaskId, Task};

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
	pub type Tasks<T: Config> =
		StorageMap<_, Twox64Concat, TaskId, Task>;

	/// Mapping the sender's address on the source chain to the history of tasks related to him
	#[pallet::storage]
	#[pallet::getter(fn account_tasks)]
	pub type AccountTasks<T: Config> =
		StorageMap<_, Twox64Concat, Vec<u8>, Vec<TaskId>>;

	/// Queue that contains all unfinished tasks belongs to the worker,
	/// Worker should read this storage to get the unfinished task and contines the task execution
	/// whenever scheduler triggered the query operation of InDex Ink contract.
	/// worker_account => task_queue
	#[pallet::storage]
	#[pallet::getter(fn pending_tasks)]
	pub type PendingTasks<T: Config> = StorageMap<_, Twox64Concat, T::AccountId, VecDeque<TaskId>>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// Executor is set.
		ExecutorSet {
			executor: T::AccountId,
		},
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
	{
		#[pallet::weight(195_000_000)]
		#[transactional]
		pub fn force_set_executor(
			origin: OriginFor<T>,
			executor: T::AccountId,
		) -> DispatchResult {
			T::CommitteeOrigin::ensure_origin(origin)?;
			Executor::<T>::set(Some(executor.clone()));
			Self::deposit_event(Event::ExecutorSet { executor });
			Ok(())
		}

		#[pallet::weight(195_000_000)]
		#[transactional]
		pub fn update_task(
			origin: OriginFor<T>,
			task: Task,
		) -> DispatchResult {
			// Check origin, must be the executor
			Self::ensure_executor(origin)?;

			// Create new record to `tasks` if task does not exist,
			// Also save to corresponding worker and sender task queue.

			// Else replace with new task data after verification passed.
			ensure!(Self::verify_task(&task), Error::<T>::TaskInvalid);

			Self::deposit_event(Event::TaskUpdated { task: task.encode() });
			Ok(())
		}
	}

	impl<T: Config> Pallet<T>
	{
		fn ensure_executor(origin: OriginFor<T>) -> DispatchResult {
			let sender = ensure_signed(origin)?;
			ensure!(Executor::<T>::get() == Some(sender), Error::<T>::ExecutorMismatch);
			Ok(())
		}

		fn verify_task(task: &Task) -> bool {
			// TODO.wf
			false
		}
	}


	#[cfg(test)]
	mod tests {
		use crate as pallet_index;

	}
}
