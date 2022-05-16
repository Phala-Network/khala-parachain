use crate::{chainbridge, xcmbridge, xtransfer};

pub mod subbridge_v3_migration {
	use super::*;
	use frame_support::{
		ensure,
		traits::{Get, StorageVersion},
	};

	type Versions = (
		// Version of chainbridge
		StorageVersion,
		// Version of xcmbridge
		StorageVersion,
		// Version of xtransfer
		StorageVersion,
	);

	const EXPECTED_STORAGE_VERSION: Versions = (
		StorageVersion::new(1),
		StorageVersion::new(0),
		StorageVersion::new(0),
	);

	const FINAL_STORAGE_VERSION: Versions = (
		StorageVersion::new(3),
		StorageVersion::new(3),
		StorageVersion::new(3),
	);

	fn get_versions<T>() -> Versions
	where
		T: chainbridge::Config + xcmbridge::Config + xtransfer::Config,
	{
		(
			StorageVersion::get::<chainbridge::Pallet<T>>(),
			StorageVersion::get::<xcmbridge::Pallet<T>>(),
			StorageVersion::get::<xtransfer::Pallet<T>>(),
		)
	}

	pub fn pre_migrate<T>() -> Result<(), &'static str>
	where
		T: chainbridge::Config + xcmbridge::Config + xtransfer::Config,
	{
		log::info!("Subbridge current version: {:?}", get_versions::<T>());
		ensure!(
			get_versions::<T>() == EXPECTED_STORAGE_VERSION,
			"Incorrect subbridge storage version in pre migrate"
		);
		log::info!("Subbridge pre migration check passed👏");
		Ok(())
	}

	pub fn migrate<T>() -> frame_support::weights::Weight
	where
		T: chainbridge::Config + xcmbridge::Config + xtransfer::Config,
	{
		if get_versions::<T>() == EXPECTED_STORAGE_VERSION {
			log::info!("Start subbridge migration");

			// Remove old chainbridge::BridgeFee storage
			frame_support::migration::remove_storage_prefix(b"BridgeTransfer", b"BridgeFee", &[]);

			// Create new storage items of fee setting in pallet ChainBridge
			chainbridge::BridgeFee::<T>::insert(0, 300_000_000_000_000);
			chainbridge::BridgeFee::<T>::insert(2, 1_000_000_000_000);

			// Set new storage version
			StorageVersion::new(3).put::<chainbridge::Pallet<T>>();
			StorageVersion::new(3).put::<xcmbridge::Pallet<T>>();
			StorageVersion::new(3).put::<xtransfer::Pallet<T>>();

			log::info!("Subbridge migration done👏");

			T::DbWeight::get().writes(6)
		} else {
			T::DbWeight::get().reads(1)
		}
	}

	pub fn post_migrate<T>() -> Result<(), &'static str>
	where
		T: chainbridge::Config + xcmbridge::Config + xtransfer::Config,
	{
		ensure!(
			get_versions::<T>() == FINAL_STORAGE_VERSION,
			"Incorrect subbridge storage version in post migrate"
		);
		log::info!("Subbridge post migration check passed👏");

		Ok(())
	}
}
