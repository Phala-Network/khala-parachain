use crate::{chainbridge, xcmbridge, xtransfer};

mod subbridge_v3_migration_common {
	use super::*;
	use frame_support::traits::StorageVersion;

	pub type Versions = (
		// Version of chainbridge
		StorageVersion,
		// Version of xcmbridge
		StorageVersion,
		// Version of xtransfer
		StorageVersion,
	);

	pub const EXPECTED_RHALA_STORAGE_VERSION: Versions = (
		StorageVersion::new(1),
		StorageVersion::new(0),
		StorageVersion::new(0),
	);

	pub const EXPECTED_KHALA_STORAGE_VERSION: Versions = (
		StorageVersion::new(2),
		StorageVersion::new(2),
		StorageVersion::new(2),
	);

	pub const FINAL_STORAGE_VERSION: Versions = (
		StorageVersion::new(3),
		StorageVersion::new(3),
		StorageVersion::new(3),
	);

	pub fn get_versions<T>() -> Versions
	where
		T: chainbridge::Config + xcmbridge::Config + xtransfer::Config,
	{
		(
			StorageVersion::get::<chainbridge::Pallet<T>>(),
			StorageVersion::get::<xcmbridge::Pallet<T>>(),
			StorageVersion::get::<xtransfer::Pallet<T>>(),
		)
	}

	pub fn do_migration<T>() -> frame_support::weights::Weight
	where
		T: chainbridge::Config + xcmbridge::Config + xtransfer::Config,
	{
		// Remove old chainbridge::BridgeFee storage
		let _ = frame_support::migration::clear_storage_prefix(
			b"ChainBridge",
			b"BridgeFee",
			&[],
			None,
			None,
		);

		// Create new storage items of fee setting in pallet ChainBridge
		chainbridge::BridgeFee::<T>::insert(0, 300_000_000_000_000);
		chainbridge::BridgeFee::<T>::insert(2, 1_000_000_000_000);

		// Set new storage version
		StorageVersion::new(3).put::<chainbridge::Pallet<T>>();
		StorageVersion::new(3).put::<xcmbridge::Pallet<T>>();
		StorageVersion::new(3).put::<xtransfer::Pallet<T>>();

		6
	}
}

pub mod subbridge_v3_migration_for_rhala {
	use super::*;
	use frame_support::{ensure, traits::Get};
	use subbridge_v3_migration_common as common;

	pub fn pre_migrate<T>() -> Result<(), &'static str>
	where
		T: chainbridge::Config + xcmbridge::Config + xtransfer::Config,
	{
		ensure!(
			common::get_versions::<T>() == common::EXPECTED_RHALA_STORAGE_VERSION,
			"Incorrect subbridge storage version in pre migrate"
		);
		log::info!("Subbridge pre migration check passed👏");
		Ok(())
	}

	pub fn migrate<T>() -> frame_support::weights::Weight
	where
		T: chainbridge::Config + xcmbridge::Config + xtransfer::Config,
	{
		if common::get_versions::<T>() == common::EXPECTED_RHALA_STORAGE_VERSION {
			log::info!("Start subbridge migration");

			let write_count = common::do_migration::<T>();

			log::info!("Subbridge migration done👏");

			T::DbWeight::get().writes(write_count)
		} else {
			T::DbWeight::get().reads(1)
		}
	}

	pub fn post_migrate<T>() -> Result<(), &'static str>
	where
		T: chainbridge::Config + xcmbridge::Config + xtransfer::Config,
	{
		ensure!(
			common::get_versions::<T>() == common::FINAL_STORAGE_VERSION,
			"Incorrect subbridge storage version in post migrate"
		);
		log::info!("Subbridge post migration check passed👏");

		Ok(())
	}
}

pub mod subbridge_v3_migration_for_khala {
	use super::*;
	use frame_support::{ensure, traits::Get};
	use subbridge_v3_migration_common as common;

	pub fn pre_migrate<T>() -> Result<(), &'static str>
	where
		T: chainbridge::Config + xcmbridge::Config + xtransfer::Config,
	{
		ensure!(
			common::get_versions::<T>() == common::EXPECTED_KHALA_STORAGE_VERSION,
			"Incorrect subbridge storage version in pre migrate"
		);
		log::info!("Subbridge pre migration check passed👏");
		Ok(())
	}

	pub fn migrate<T>() -> frame_support::weights::Weight
	where
		T: chainbridge::Config + xcmbridge::Config + xtransfer::Config,
	{
		if common::get_versions::<T>() == common::EXPECTED_KHALA_STORAGE_VERSION {
			log::info!("Start subbridge migration");

			let write_count = common::do_migration::<T>();

			log::info!("Subbridge migration done👏");

			T::DbWeight::get().writes(write_count)
		} else {
			T::DbWeight::get().reads(1)
		}
	}

	pub fn post_migrate<T>() -> Result<(), &'static str>
	where
		T: chainbridge::Config + xcmbridge::Config + xtransfer::Config,
	{
		ensure!(
			common::get_versions::<T>() == common::FINAL_STORAGE_VERSION,
			"Incorrect subbridge storage version in post migrate"
		);
		log::info!("Subbridge post migration check passed👏");

		Ok(())
	}
}
