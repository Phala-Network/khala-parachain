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

	fn migrate_chainbridge<T: Config>(
		asset_id: T::AssetId,
		chain_id: u8,
		is_mintable: bool,
		metadata: Box<Vec<u8>>,
	) -> frame_support::weights::Weight {
		if let Some(mut info) = RegistryInfoByIds::<T>::get(&asset_id) {
			let resource_id: [u8; 32] = info.location.clone().into_rid(chain_id);

			IdByResourceId::<T>::insert(&resource_id, &asset_id);
			// Save into registry info, here save chain id can not be added more than twice
			let reserve_location: MultiLocation = (
				0,
				X2(
					GeneralKey(CB_ASSET_KEY.to_vec()),
					GeneralIndex(chain_id as u128),
				),
			)
				.into();
			info.enabled_bridges.push(XBridge {
				config: XBridgeConfig::ChainBridge {
					chain_id,
					resource_id: resource_id.clone(),
					reserve_account: reserve_location.into_account(),
					is_mintable,
				},
				metadata,
			});
			RegistryInfoByIds::<T>::insert(&asset_id, &info);

			2
		} else {
			0
		}
	}

	pub fn pre_migrate<T>() -> Result<(), &'static str>
	where
		T: Config,
		<T as pallet_assets::Config>::AssetId: From<u32>,
	{
		let ksm_id: T::AssetId = 0u32.into();
		let kar_id: T::AssetId = 1u32.into();
		let bnc_id: T::AssetId = 2u32.into();
		let zlk_id: T::AssetId = 3u32.into();
		let ausd_id: T::AssetId = 4u32.into();

		ensure!(
			RegistryInfoByIds::<T>::get(&ksm_id) == None,
			"KSM already registered"
		);
		ensure!(
			RegistryInfoByIds::<T>::get(&kar_id) == None,
			"KAR already registered"
		);
		ensure!(
			RegistryInfoByIds::<T>::get(&bnc_id) == None,
			"BNC already registered"
		);
		ensure!(
			RegistryInfoByIds::<T>::get(&zlk_id) == None,
			"ZLK already registered"
		);
		ensure!(
			RegistryInfoByIds::<T>::get(&ausd_id) == None,
			"aUSD already registered"
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
			let mut write_count = 0;

			// Clean storage items in old pallet
			write_count += remove_assetswrapper_storage();

			// Migrate KSM registry
			write_count += migrate_asset_register::<T>(
				MultiLocation::new(1, Here),
				0u32.into(),
				AssetProperties {
					name: b"Kusama".to_vec(),
					symbol: b"KSM".to_vec(),
					decimals: 12,
				},
			);

			// Migrate KAR registry
			write_count += migrate_asset_register::<T>(
				MultiLocation::new(1, X2(Parachain(2000), GeneralKey([0x0, 0x80].to_vec()))),
				1u32.into(),
				AssetProperties {
					name: b"Karura".to_vec(),
					symbol: b"KAR".to_vec(),
					decimals: 12,
				},
			);

			// Migrate BNC registry
			write_count += migrate_asset_register::<T>(
				MultiLocation::new(1, X2(Parachain(2001), GeneralKey([0x0, 0x01].to_vec()))),
				2u32.into(),
				AssetProperties {
					name: b"Bifrost".to_vec(),
					symbol: b"BNC".to_vec(),
					decimals: 12,
				},
			);

			// Migrate ZLK registry
			write_count += migrate_asset_register::<T>(
				MultiLocation::new(1, X2(Parachain(2001), GeneralKey([0x02, 0x07].to_vec()))),
				3u32.into(),
				AssetProperties {
					name: b"Zenlink".to_vec(),
					symbol: b"ZLK".to_vec(),
					decimals: 18,
				},
			);

			// Migrate aUSD registry
			write_count += migrate_asset_register::<T>(
				MultiLocation::new(1, X2(Parachain(2000), GeneralKey([0x0, 0x81].to_vec()))),
				4u32.into(),
				AssetProperties {
					name: b"aUSD".to_vec(),
					symbol: b"aUSD".to_vec(),
					decimals: 12,
				},
			);

			// Enable ZLK Chainbridge transfer
			write_count += migrate_chainbridge::<T>(
				3u32.into(),
				2, // Moonriver
				false,
				Box::new(Vec::new()),
			);

			// Set new storage version
			StorageVersion::new(1).put::<Pallet<T>>();

			log::info!("Assets registry migration done👏");

			T::DbWeight::get().writes(write_count + 1)
		} else {
			T::DbWeight::get().reads(1)
		}
	}

	pub fn post_migrate<T>() -> Result<(), &'static str>
	where
		T: Config,
		<T as pallet_assets::Config>::AssetId: From<u32>,
	{
		let ksm_id: T::AssetId = 0u32.into();
		let kar_id: T::AssetId = 1u32.into();
		let bnc_id: T::AssetId = 2u32.into();
		let zlk_id: T::AssetId = 3u32.into();
		let ausd_id: T::AssetId = 4u32.into();

		ensure!(
			StorageVersion::get::<Pallet<T>>() == FINAL_STORAGE_VERSION,
			"Incorrect AssetRegistry storage version in post migrate"
		);

		let ksm_info =
			RegistryInfoByIds::<T>::get(&ksm_id).ok_or(Error::<T>::AssetNotRegistered)?;
		ensure!(
			&ksm_info.properties.symbol == b"KSM",
			"Incorrect registry info of KSM"
		);
		let kar_info =
			RegistryInfoByIds::<T>::get(&kar_id).ok_or(Error::<T>::AssetNotRegistered)?;
		ensure!(
			&kar_info.properties.symbol == b"KAR",
			"Incorrect registry info of KAR"
		);
		let bnc_info =
			RegistryInfoByIds::<T>::get(&bnc_id).ok_or(Error::<T>::AssetNotRegistered)?;
		ensure!(
			&bnc_info.properties.symbol == b"BNC",
			"Incorrect registry info of BNC"
		);
		let zlk_info =
			RegistryInfoByIds::<T>::get(&zlk_id).ok_or(Error::<T>::AssetNotRegistered)?;
		ensure!(
			&zlk_info.properties.symbol == b"ZLK",
			"Incorrect registry info of ZLK"
		);
		let ausd_info =
			RegistryInfoByIds::<T>::get(&ausd_id).ok_or(Error::<T>::AssetNotRegistered)?;
		ensure!(
			&ausd_info.properties.symbol == b"aUSD",
			"Incorrect registry info of aUSD"
		);
		log::info!("Assets registry post migration check passed👏");

		Ok(())
	}
}
