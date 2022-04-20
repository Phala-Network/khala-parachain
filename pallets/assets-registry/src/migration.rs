use super::*;

pub mod assets_registry_migration {
	use super::*;
	use frame_support::{
		ensure,
		traits::{Get, StorageVersion},
	};
	use log;
	use sp_std::{boxed::Box, vec, vec::Vec};
	use xcm::latest::{prelude::*, MultiLocation};

	const EXPECTED_STORAGE_VERSION: StorageVersion = StorageVersion::new(0);
	const FINAL_STORAGE_VERSION: StorageVersion = StorageVersion::new(1);

	fn remove_assetswrapper_storage() -> frame_support::weights::Weight {
		frame_support::migration::remove_storage_prefix(b"AssetsWrapper", b"IdByAssets", &[]);
		frame_support::migration::remove_storage_prefix(b"AssetsWrapper", b"AssetByIds", &[]);
		frame_support::migration::remove_storage_prefix(
			b"AssetsWrapper",
			b"AssetByResourceIds",
			&[],
		);
		frame_support::migration::remove_storage_prefix(
			b"AssetsWrapper",
			b"RegistryInfoByIds",
			&[],
		);

		4
	}

	fn migrate_asset_register<T: Config>(
		location: MultiLocation,
		asset_id: T::AssetId,
		properties: AssetProperties,
	) -> frame_support::weights::Weight {
		IdByLocations::<T>::insert(&location, asset_id);
		RegistryInfoByIds::<T>::insert(
			asset_id,
			AssetRegistryInfo {
				location: location.clone(),
				reserve_location: location.clone().reserve_location(),
				// Xcmp will be enabled when assets being registered.
				enabled_bridges: vec![XBridge {
					config: XBridgeConfig::Xcmp,
					metadata: Box::new(Vec::new()),
				}],
				properties: properties.clone(),
			},
		);

		2
	}

	pub fn pre_migrate<T>() -> Result<(), &'static str>
	where
		T: Config,
		<T as pallet_assets::Config>::AssetId: From<u32>,
	{
		let dev_id: T::AssetId = 1u32.into();

		ensure!(
			RegistryInfoByIds::<T>::get(&dev_id) == None,
			"DEV already registered"
		);

		ensure!(
			StorageVersion::get::<Pallet<T>>() == EXPECTED_STORAGE_VERSION,
			"Incorrect AssetRegistry storage version in pre migrate"
		);

		log::info!("Assets registry pre migration check passed👏");

		Ok(())
	}

	pub fn migrate<T>() -> frame_support::weights::Weight
	where
		T: Config,
		<T as pallet_assets::Config>::AssetId: From<u32>,
	{
		if StorageVersion::get::<Pallet<T>>() == EXPECTED_STORAGE_VERSION {
			log::info!("Start assets-registry migration");
			let mut weight = 0;

			// Clean storage items in old pallet
			weight += remove_assetswrapper_storage();

			// Migrate KAR registry
			weight += migrate_asset_register::<T>(
				MultiLocation::new(1, X2(Parachain(1000), PalletInstance(3))),
				1u32.into(),
				AssetProperties {
					name: b"DEV".to_vec(),
					symbol: b"DEV".to_vec(),
					decimals: 18,
				},
			);

			// Set new storage version
			StorageVersion::new(1).put::<Pallet<T>>();

			log::info!("Assets registry migration done👏");

			weight += T::DbWeight::get().writes(weight + 1);
			weight
		} else {
			T::DbWeight::get().reads(1)
		}
	}

	pub fn post_migrate<T>() -> Result<(), &'static str>
	where
		T: Config,
		<T as pallet_assets::Config>::AssetId: From<u32>,
	{
		let dev_id: T::AssetId = 1u32.into();

		ensure!(
			StorageVersion::get::<Pallet<T>>() == FINAL_STORAGE_VERSION,
			"Incorrect AssetRegistry storage version in post migrate"
		);

		let dev_id =
			RegistryInfoByIds::<T>::get(&dev_id).ok_or(Error::<T>::AssetNotRegistered)?;
		ensure!(
			&dev_id.properties.symbol == b"DEV",
			"Incorrect registry info of DEV"
		);
		log::info!("Assets registry post migration check passed👏");

		Ok(())
	}
}
