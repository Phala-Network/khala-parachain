#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(test)]
mod mock;

pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
	use assets_registry::GetAssetRegistryInfo;
	use codec::Encode;
	use frame_support::{
		dispatch::DispatchResult, pallet_prelude::*, traits::StorageVersion, transactional,
		PalletId,
	};
	use frame_system::pallet_prelude::*;
	use sp_runtime::traits::AccountIdConversion;
	use sp_std::{vec, vec::Vec};
	use xcm::latest::{prelude::*, AssetId as XcmAssetId};
	use xcm_executor::traits::TransactAsset;

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	#[pallet::storage_version(STORAGE_VERSION)]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(_);

	pub type TaskId = [u8; 32];
	pub const MODULE_ID: PalletId = PalletId(*b"index/ac");

	#[derive(Clone, Decode, Encode, Eq, PartialEq, Ord, PartialOrd, Debug, TypeInfo)]
	pub struct DepositInfo<AccountId> {
		/// Sender address on current chain
		pub sender: AccountId,
		/// Deposit asset
		pub asset: XcmAssetId,
		/// Deposit amount
		pub amount: u128,
		/// Recipient address on dest chain
		pub recipient: Vec<u8>,
		/// Encoded execution plan produced by Solver
		pub task: Vec<u8>,
	}

	#[pallet::config]
	pub trait Config: frame_system::Config + pallet_assets::Config {
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
		type CommitteeOrigin: EnsureOrigin<Self::RuntimeOrigin>;
		/// Asset adapter to do withdraw, deposit etc.
		type AssetTransactor: TransactAsset;
		/// Assets registry
		type AssetsRegistry: GetAssetRegistryInfo<<Self as pallet_assets::Config>::AssetId>;
	}

	const STORAGE_VERSION: StorageVersion = StorageVersion::new(0);

	/// Pre-set index worker account
	#[pallet::storage]
	#[pallet::getter(fn executors)]
	pub type Workers<T: Config> = StorageMap<_, Twox64Concat, T::AccountId, bool, ValueQuery>;

	/// Mapping task_id to the full deposit data
	#[pallet::storage]
	#[pallet::getter(fn deposit_records)]
	pub type DepositRecords<T: Config> =
		StorageMap<_, Twox64Concat, TaskId, DepositInfo<T::AccountId>>;

	/// Mapping the worker account and its actived task queue
	#[pallet::storage]
	#[pallet::getter(fn actived_tasks)]
	pub type ActivedTasks<T: Config> =
		StorageMap<_, Twox64Concat, T::AccountId, Vec<TaskId>, ValueQuery>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// Worker is set.
		WorkerAdd { worker: T::AccountId },
		/// Worker is set.
		WorkerRemove { worker: T::AccountId },
		/// New task saved.
		NewTask {
			/// Record
			deposit_info: DepositInfo<T::AccountId>,
		},
		/// Task has been claimed.
		Claimed { tasks: Vec<TaskId> },
	}

	#[pallet::error]
	pub enum Error<T> {
		AssetNotFound,
		WorkerAlreadySet,
		WorkerNotSet,
		WorkerMismatch,
		TaskAlreadyExist,
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
				Workers::<T>::get(&worker) == false,
				Error::<T>::WorkerAlreadySet
			);
			Workers::<T>::insert(&worker, true);
			Self::deposit_event(Event::WorkerAdd { worker });
			Ok(())
		}

		#[pallet::weight(195_000_000)]
		#[pallet::call_index(1)]
		#[transactional]
		pub fn force_remove_worker(origin: OriginFor<T>, worker: T::AccountId) -> DispatchResult {
			T::CommitteeOrigin::ensure_origin(origin)?;
			ensure!(Workers::<T>::get(&worker), Error::<T>::WorkerNotSet);
			Workers::<T>::insert(&worker, false);
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
			worker: T::AccountId,
			task_id: TaskId,
			task: Vec<u8>,
		) -> DispatchResult {
			let sender: T::AccountId = ensure_signed(origin)?;

			// Ensure given worker was registered
			ensure!(Workers::<T>::get(&worker), Error::<T>::WorkerNotSet);
			// Check if foreign asset was registered on our chain
			if &asset != &MultiLocation::here().into() {
				let asset_location = match &asset {
					Concrete(location) => Some(location),
					_ => None,
				}
				.ok_or(Error::<T>::AssetNotFound)?;
				ensure!(
					T::AssetsRegistry::id(&asset_location).is_some(),
					Error::<T>::AssetNotFound
				);
			}

			// Check if record already exist
			ensure!(
				DepositRecords::<T>::get(&task_id).is_none(),
				Error::<T>::TaskAlreadyExist
			);

			// Save record to corresponding worker task queue
			let mut worker_task_queue = ActivedTasks::<T>::get(&worker);
			worker_task_queue.push(task_id);
			ActivedTasks::<T>::insert(&worker, &worker_task_queue);
			// Save record data
			let deposit_info = DepositInfo {
				sender: sender.clone(),
				asset: asset.clone(),
				amount,
				recipient,
				task,
			};
			DepositRecords::<T>::insert(&task_id, &deposit_info);

			// Deposit into module account
			let module_account: T::AccountId = MODULE_ID.into_account_truncating();
			Self::do_asset_transact(asset.clone(), sender.clone(), module_account, amount)?;

			Self::deposit_event(Event::NewTask { deposit_info });
			Ok(())
		}

		/// Claim the oldest task that saved in actived task queue
		#[pallet::weight(195_000_000)]
		#[pallet::call_index(3)]
		#[transactional]
		pub fn claim_task(origin: OriginFor<T>, task_id: TaskId) -> DispatchResult {
			// Check origin, must be the worker
			let worker: T::AccountId = ensure_signed(origin)?;
			ensure!(
				Workers::<T>::get(&worker) == true,
				Error::<T>::WorkerMismatch
			);

			let worker_task_queue = ActivedTasks::<T>::get(&worker);
			// Check reqeust exist in actived task queue
			ensure!(
				worker_task_queue.contains(&task_id),
				Error::<T>::NotFoundInTaskQueue
			);
			// Remove the specific task from worker task queue
			let new_task_queue: Vec<TaskId> = worker_task_queue
				.into_iter()
				.filter(|item| *item != task_id)
				.collect();
			ActivedTasks::<T>::insert(&worker, &new_task_queue);
			let deposit_info = DepositRecords::<T>::get(&task_id).unwrap();

			// Withdraw from module account
			let module_account: T::AccountId = MODULE_ID.into_account_truncating();
			Self::do_asset_transact(
				deposit_info.asset.clone(),
				module_account,
				worker,
				deposit_info.amount,
			)?;

			// Delete deposit record
			DepositRecords::<T>::remove(&task_id);

			Self::deposit_event(Event::Claimed {
				tasks: vec![task_id],
			});
			Ok(())
		}

		#[pallet::weight(195_000_000)]
		#[pallet::call_index(4)]
		#[transactional]
		pub fn claim_all_task(origin: OriginFor<T>) -> DispatchResult {
			// Check origin, must be the worker
			let worker: T::AccountId = ensure_signed(origin)?;
			ensure!(
				Workers::<T>::get(&worker) == true,
				Error::<T>::WorkerMismatch
			);

			// Take the task queue of the worker
			let worker_task_queue = ActivedTasks::<T>::take(&worker);
			// Check reqeust exist in actived task queue
			ensure!(worker_task_queue.len() > 0, Error::<T>::NotFoundInTaskQueue);
			for task_id in worker_task_queue.iter() {
				let deposit_info = DepositRecords::<T>::get(&task_id).unwrap();

				// Withdraw from module account
				let module_account: T::AccountId = MODULE_ID.into_account_truncating();
				Self::do_asset_transact(
					deposit_info.asset.clone(),
					module_account,
					worker.clone(),
					deposit_info.amount,
				)?;

				// Delete deposit record
				DepositRecords::<T>::remove(&task_id);
			}

			Self::deposit_event(Event::Claimed {
				tasks: worker_task_queue.into(),
			});
			Ok(())
		}
	}

	impl<T: Config> Pallet<T>
	where
		T::AccountId: Into<[u8; 32]> + From<[u8; 32]>,
	{
		fn do_asset_transact(
			asset: XcmAssetId,
			sender: T::AccountId,
			recipient: T::AccountId,
			amount: u128,
		) -> DispatchResult {
			T::AssetTransactor::withdraw_asset(
				&(asset.clone(), amount).into(),
				&Junction::AccountId32 {
					network: NetworkId::Any,
					id: sender.into(),
				}
				.into(),
			)
			.or(Err(Error::<T>::TransactFailed))?;
			// Deposit into worker account
			T::AssetTransactor::deposit_asset(
				&(asset, amount).into(),
				&Junction::AccountId32 {
					network: NetworkId::Any,
					id: recipient.into(),
				}
				.into(),
			)
			.or(Err(Error::<T>::TransactFailed))?;

			Ok(())
		}
	}

	#[cfg(test)]
	mod tests {
		use crate as pallet_index;
		use crate::{ActivedTasks, DepositRecords, Event as PalletIndexEvent, Workers, MODULE_ID};
		use frame_support::{assert_noop, assert_ok};
		use pallet_index::mock::{
			assert_events, new_test_ext, Assets, AssetsRegistry, Balances, PalletIndex,
			RuntimeEvent as Event, RuntimeOrigin as Origin, Test, TestAssetAssetId,
			TestAssetLocation, ALICE, BOB, ENDOWED_BALANCE,
		};
		use sp_runtime::traits::AccountIdConversion;
		use xcm::latest::MultiLocation;

		#[test]
		fn test_add_executor_should_work() {
			new_test_ext().execute_with(|| {
				assert_ok!(PalletIndex::force_add_worker(Origin::root(), BOB));
				assert_eq!(Workers::<Test>::get(&BOB), true);
				assert_events(vec![Event::PalletIndex(PalletIndexEvent::WorkerAdd {
					worker: BOB,
				})]);
				assert_ok!(PalletIndex::force_remove_worker(Origin::root(), BOB));
				assert_eq!(Workers::<Test>::get(&BOB), false);
			});
		}

		#[test]
		fn test_deposit_task_should_work() {
			new_test_ext().execute_with(|| {
				let task_id = [2; 32];
				let module_account: <Test as frame_system::Config>::AccountId =
					MODULE_ID.into_account_truncating();

				assert_noop!(
					PalletIndex::deposit_task(
						Origin::signed(ALICE),
						MultiLocation::here().into(),
						100u128,
						[1, 2, 3].to_vec(),
						BOB,
						[2; 32],
						[1, 2, 3, 4, 5, 6, 7, 8].to_vec(),
					),
					pallet_index::Error::<Test>::WorkerNotSet
				);

				assert_ok!(PalletIndex::force_add_worker(Origin::root(), BOB));
				assert_eq!(Balances::free_balance(ALICE), ENDOWED_BALANCE);
				assert_eq!(Balances::free_balance(module_account.clone()), 0);
				assert_ok!(PalletIndex::deposit_task(
					Origin::signed(ALICE),
					MultiLocation::here().into(),
					100u128,
					[1, 2, 3].to_vec(),
					BOB,
					task_id,
					[1, 2, 3, 4, 5, 6, 7, 8].to_vec(),
				));
				assert_eq!(Balances::free_balance(ALICE), ENDOWED_BALANCE - 100);
				assert_eq!(Balances::free_balance(module_account.clone()), 100);
				assert_eq!(ActivedTasks::<Test>::get(&BOB), [task_id].to_vec());
				assert_eq!(DepositRecords::<Test>::get(&task_id).unwrap().sender, ALICE);

				assert_noop!(
					PalletIndex::deposit_task(
						Origin::signed(ALICE),
						MultiLocation::here().into(),
						100u128,
						[1, 2, 3].to_vec(),
						BOB,
						[2; 32],
						[1, 2, 3, 4, 5, 6, 7, 8].to_vec(),
					),
					pallet_index::Error::<Test>::TaskAlreadyExist
				);

				// Should fail if foreign asset not registered
				assert_noop!(
					PalletIndex::deposit_task(
						Origin::signed(ALICE),
						TestAssetLocation::get().into(),
						100u128,
						[1, 2, 3].to_vec(),
						BOB,
						[3; 32],
						[1, 2, 3, 4, 5, 6, 7, 8].to_vec(),
					),
					pallet_index::Error::<Test>::AssetNotFound
				);
				// Register asset
				assert_ok!(AssetsRegistry::force_register_asset(
					Origin::root(),
					TestAssetLocation::get().into(),
					TestAssetAssetId::get(),
					assets_registry::AssetProperties {
						name: b"TestAsset".to_vec(),
						symbol: b"TA".to_vec(),
						decimals: 12,
					},
				));
				// mint some asset to Alice, only ASSETS_REGISTRY_ID has mint permission
				let asset_owner: <Test as frame_system::Config>::AccountId =
					assets_registry::ASSETS_REGISTRY_ID.into_account_truncating();
				assert_ok!(Assets::mint(
					Origin::signed(asset_owner.clone()),
					TestAssetAssetId::get(),
					ALICE.into(),
					1_000
				));
				assert_eq!(Assets::balance(TestAssetAssetId::get(), &ALICE), 1_000);
				// Now, deposit should success
				assert_ok!(PalletIndex::deposit_task(
					Origin::signed(ALICE),
					TestAssetLocation::get().into(),
					100u128,
					[1, 2, 3].to_vec(),
					BOB,
					[3; 32],
					[1, 2, 3, 4, 5, 6, 7, 8].to_vec(),
				));
				assert_eq!(
					Assets::balance(TestAssetAssetId::get(), &ALICE),
					1_000 - 100
				);
				assert_eq!(
					Assets::balance(TestAssetAssetId::get(), &module_account),
					100
				);
			})
		}

		#[test]
		fn test_claim_task_should_work() {
			new_test_ext().execute_with(|| {
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
					BOB,
					request_id_1,
					[1, 2, 3, 4, 5, 6, 7, 8].to_vec(),
				));
				assert_ok!(PalletIndex::deposit_task(
					Origin::signed(ALICE),
					MultiLocation::here().into(),
					200u128,
					[1, 2, 3].to_vec(),
					BOB,
					request_id_2,
					[1, 2, 3, 4, 5, 6, 7, 8].to_vec(),
				));
				assert_ok!(PalletIndex::deposit_task(
					Origin::signed(ALICE),
					MultiLocation::here().into(),
					300u128,
					[1, 2, 3].to_vec(),
					BOB,
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
