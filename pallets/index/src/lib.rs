#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(test)]
mod mock;

pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
	use codec::Encode;
	use frame_support::{
		dispatch::DispatchResult, pallet_prelude::*, traits::StorageVersion, transactional,
		PalletId,
	};
	use frame_system::pallet_prelude::*;
	use sp_runtime::traits::AccountIdConversion;
	use sp_std::{collections::vec_deque::VecDeque, vec, vec::Vec};
	use xcm::latest::{prelude::*, AssetId as XcmAssetId};
	use xcm_executor::traits::TransactAsset;

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	#[pallet::storage_version(STORAGE_VERSION)]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(_);

	pub type RequestId = [u8; 32];
	pub const MODULE_ID: PalletId = PalletId(*b"index/ac");

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
		/// Asset adapter to do withdraw, deposit etc.
		type AssetTransactor: TransactAsset;
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
		TransactFailed,
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
			ensure!(
				Workers::<T>::get(&worker.clone().into()),
				Error::<T>::WorkerNotSet
			);
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
				asset: asset.clone(),
				amount,
				recipient,
				request,
			};
			DepositRecords::<T>::insert(&request_id, &deposit_info);

			// Withdraw from sender account
			T::AssetTransactor::withdraw_asset(
				&(asset.clone(), amount).into(),
				&Junction::AccountId32 {
					network: NetworkId::Any,
					id: sender.into(),
				}
				.into(),
			)
			.map_err(|_| Error::<T>::TransactFailed)?;
			// Deposit into module account
			let module_account: T::AccountId = MODULE_ID.into_account_truncating();
			T::AssetTransactor::deposit_asset(
				&(asset.clone(), amount).into(),
				&Junction::AccountId32 {
					network: NetworkId::Any,
					id: module_account.into(),
				}
				.into(),
			)
			.map_err(|_| Error::<T>::TransactFailed)?;

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
			let _ = worker_task_queue
				.pop_front()
				.ok_or(Error::<T>::TaskQueueEmpty)?;
			ActivedRequests::<T>::insert(&worker, &worker_task_queue);
			let deposit_info = DepositRecords::<T>::get(&request_id).unwrap();

			// Withdraw from module account
			let module_account: T::AccountId = MODULE_ID.into_account_truncating();
			T::AssetTransactor::withdraw_asset(
				&(deposit_info.asset.clone(), deposit_info.amount).into(),
				&Junction::AccountId32 {
					network: NetworkId::Any,
					id: module_account.into(),
				}
				.into(),
			)
			.map_err(|_| Error::<T>::TransactFailed)?;
			// Deposit into worker account
			T::AssetTransactor::deposit_asset(
				&(deposit_info.asset.clone(), deposit_info.amount).into(),
				&Junction::AccountId32 {
					network: NetworkId::Any,
					id: worker,
				}
				.into(),
			)
			.map_err(|_| Error::<T>::TransactFailed)?;
			// Delete deposit record
			DepositRecords::<T>::remove(&request_id);

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
			// Check reqeust exist in actived task queue
			ensure!(worker_task_queue.len() > 0, Error::<T>::NotFoundInTaskQueue);
			// Put an empty task queue
			ActivedRequests::<T>::insert(&worker, &VecDeque::<RequestId>::new());
			for request_id in worker_task_queue.iter() {
				let deposit_info = DepositRecords::<T>::get(&request_id).unwrap();
				// Withdraw from module account
				let module_account: T::AccountId = MODULE_ID.into_account_truncating();
				T::AssetTransactor::withdraw_asset(
					&(deposit_info.asset.clone(), deposit_info.amount).into(),
					&Junction::AccountId32 {
						network: NetworkId::Any,
						id: module_account.into(),
					}
					.into(),
				)
				.map_err(|_| Error::<T>::TransactFailed)?;
				// Deposit into worker account
				T::AssetTransactor::deposit_asset(
					&(deposit_info.asset.clone(), deposit_info.amount).into(),
					&Junction::AccountId32 {
						network: NetworkId::Any,
						id: worker,
					}
					.into(),
				)
				.map_err(|_| Error::<T>::TransactFailed)?;
				// Delete deposit record
				DepositRecords::<T>::remove(&request_id);
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
		use crate::{
			ActivedRequests, DepositRecords, Event as PalletIndexEvent, Workers, MODULE_ID,
		};
		use frame_support::{assert_noop, assert_ok};
		use pallet_index::mock::{
			assert_events, new_test_ext, Balances, PalletIndex, RuntimeEvent as Event,
			RuntimeOrigin as Origin, Test, ALICE, BOB, ENDOWED_BALANCE,
		};
		use sp_runtime::traits::AccountIdConversion;
		use xcm::latest::MultiLocation;

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

		#[test]
		fn test_deposit_task_should_work() {
			new_test_ext().execute_with(|| {
				let alice_key: [u8; 32] = ALICE.into();
				let bob_key: [u8; 32] = BOB.into();
				let request_id = [2; 32];
				let module_account: <Test as frame_system::Config>::AccountId =
					MODULE_ID.into_account_truncating();

				assert_noop!(
					PalletIndex::deposit_task(
						Origin::signed(ALICE),
						MultiLocation::here().into(),
						100u128,
						[1, 2, 3].to_vec(),
						bob_key,
						[2; 32],
						[1, 2, 3, 4, 5, 6, 7, 8].to_vec(),
					),
					pallet_index::Error::<Test>::WorkerNotSet
				);

				assert_ok!(PalletIndex::force_add_worker(Origin::root(), BOB.clone()));
				assert_eq!(Balances::free_balance(ALICE), ENDOWED_BALANCE);
				assert_eq!(Balances::free_balance(module_account.clone()), 0);
				assert_ok!(PalletIndex::deposit_task(
					Origin::signed(ALICE),
					MultiLocation::here().into(),
					100u128,
					[1, 2, 3].to_vec(),
					bob_key,
					request_id,
					[1, 2, 3, 4, 5, 6, 7, 8].to_vec(),
				));
				assert_eq!(Balances::free_balance(ALICE), ENDOWED_BALANCE - 100);
				assert_eq!(Balances::free_balance(module_account), 100);
				assert_eq!(
					ActivedRequests::<Test>::get(&bob_key),
					[request_id].to_vec()
				);
				assert_eq!(
					DepositRecords::<Test>::get(&request_id).unwrap().sender,
					alice_key
				);

				assert_noop!(
					PalletIndex::deposit_task(
						Origin::signed(ALICE),
						MultiLocation::here().into(),
						100u128,
						[1, 2, 3].to_vec(),
						bob_key,
						[2; 32],
						[1, 2, 3, 4, 5, 6, 7, 8].to_vec(),
					),
					pallet_index::Error::<Test>::RequestAlreadyExist
				);
			})
		}

		#[test]
		fn test_claim_task_should_work() {
			new_test_ext().execute_with(|| {
				let bob_key: [u8; 32] = BOB.into();
				let request_id_1 = [1; 32];
				let request_id_2 = [2; 32];
				let request_id_3 = [3; 32];
				let module_account: <Test as frame_system::Config>::AccountId =
					MODULE_ID.into_account_truncating();

				assert_ok!(PalletIndex::force_add_worker(Origin::root(), BOB.clone()));
				assert_ok!(PalletIndex::deposit_task(
					Origin::signed(ALICE),
					MultiLocation::here().into(),
					100u128,
					[1, 2, 3].to_vec(),
					bob_key,
					request_id_1,
					[1, 2, 3, 4, 5, 6, 7, 8].to_vec(),
				));
				assert_ok!(PalletIndex::deposit_task(
					Origin::signed(ALICE),
					MultiLocation::here().into(),
					200u128,
					[1, 2, 3].to_vec(),
					bob_key,
					request_id_2,
					[1, 2, 3, 4, 5, 6, 7, 8].to_vec(),
				));
				assert_ok!(PalletIndex::deposit_task(
					Origin::signed(ALICE),
					MultiLocation::here().into(),
					300u128,
					[1, 2, 3].to_vec(),
					bob_key,
					request_id_3,
					[1, 2, 3, 4, 5, 6, 7, 8].to_vec(),
				));
				assert_eq!(Balances::free_balance(module_account.clone()), 600);
				assert_eq!(Balances::free_balance(ALICE), ENDOWED_BALANCE - 600);
				assert_eq!(Balances::free_balance(BOB), ENDOWED_BALANCE);

				// Claim failed if sender is not worker
				assert_noop!(
					PalletIndex::claim_task(Origin::signed(ALICE), request_id_1,),
					pallet_index::Error::<Test>::WorkerMismatch
				);
				assert_noop!(
					PalletIndex::claim_all_task(Origin::signed(ALICE),),
					pallet_index::Error::<Test>::WorkerMismatch
				);

				// Claim one task
				assert_ok!(PalletIndex::claim_task(Origin::signed(BOB), request_id_1,));
				assert_eq!(Balances::free_balance(module_account.clone()), 500);
				assert_eq!(Balances::free_balance(BOB), ENDOWED_BALANCE + 100);

				// Claim rest of tasks
				assert_ok!(PalletIndex::claim_all_task(Origin::signed(BOB),));
				assert_eq!(Balances::free_balance(module_account), 0);
				assert_eq!(Balances::free_balance(BOB), ENDOWED_BALANCE + 600);

				// Claim failed if no task exist
				assert_noop!(
					PalletIndex::claim_task(Origin::signed(BOB), request_id_1,),
					pallet_index::Error::<Test>::NotFoundInTaskQueue
				);
				assert_noop!(
					PalletIndex::claim_all_task(Origin::signed(BOB),),
					pallet_index::Error::<Test>::NotFoundInTaskQueue
				);
			})
		}
	}
}
