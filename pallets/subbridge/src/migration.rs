use crate::{chainbridge, xcmbridge, xtransfer};

pub mod subbridge_migration {
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
		StorageVersion::new(0),
		StorageVersion::new(0),
		StorageVersion::new(0),
	);

	const FINAL_STORAGE_VERSION: Versions = (
		StorageVersion::new(2),
		StorageVersion::new(2),
		StorageVersion::new(2),
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
			let mut weight = 0;
			// Clean storage exist in removed pallet BridgeTransfer
			frame_support::migration::remove_storage_prefix(b"BridgeTransfer", b"BridgeFee", &[]);

			// Clean torage used to save resource id in ChainBridge
			// see issue: https://github.com/Phala-Network/khala-parachain/issues/60
			frame_support::migration::remove_storage_prefix(b"ChainBridge", b"Resources", &[]);

			// Create new storage items of fee setting in pallet ChainBridge
			chainbridge::BridgeFee::<T>::insert(0, (300_000_000_000_000, 0));
			chainbridge::BridgeFee::<T>::insert(2, (1_000_000_000_000, 0));

			// Set new storage version
			StorageVersion::new(2).put::<chainbridge::Pallet<T>>();
			StorageVersion::new(2).put::<xcmbridge::Pallet<T>>();
			StorageVersion::new(2).put::<xtransfer::Pallet<T>>();

			log::info!("Subbridge migration done👏");

			weight += T::DbWeight::get().writes(7);
			weight
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
