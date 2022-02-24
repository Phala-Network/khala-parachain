pub use self::pallet::*;

#[allow(unused_variables)]
#[frame_support::pallet]
pub mod pallet {
	use codec::{Decode, Encode};
	use frame_support::{
		dispatch::DispatchResult, pallet_prelude::*, traits::StorageVersion, transactional,
	};
	use frame_system::pallet_prelude::*;
	use scale_info::TypeInfo;
	use sp_runtime::traits::StaticLookup;
	use sp_std::{boxed::Box, convert::From, vec, vec::Vec};
	use xcm::latest::{prelude::*, MultiLocation};

	#[derive(Clone, Decode, Encode, Eq, PartialEq, Ord, PartialOrd, Debug, TypeInfo)]
	pub struct XTransferAsset(pub MultiLocation);

	// Const used to indicate chainbridge assets. str "cb"
	pub const CB_ASSET_KEY: &[u8] = &[0x63, 0x62];

	#[derive(Clone, Decode, Encode, Eq, PartialEq, Ord, PartialOrd, Debug, TypeInfo)]
	pub struct XBridge {
		config: XBridgeConfig,
		metadata: Box<Vec<u8>>,
	}

	#[derive(Clone, Decode, Encode, Eq, PartialEq, Ord, PartialOrd, Debug, TypeInfo)]
	pub enum XBridgeConfig {
		Xcmp,
		ChainBridge {
			chain_id: u8,
			resource_id: [u8; 32],
			reserve_account: [u8; 32],
			is_mintable: bool,
		},
		// Potential other bridge solutions
	}

	#[derive(Clone, Decode, Encode, Eq, PartialEq, Ord, PartialOrd, Debug, TypeInfo)]
	pub struct AssetProperties {
		pub name: Vec<u8>,
		pub symbol: Vec<u8>,
		pub decimals: u8,
	}

	#[derive(Clone, Decode, Encode, Eq, PartialEq, Ord, PartialOrd, Debug, TypeInfo)]
	pub struct AssetRegistryInfo {
		location: MultiLocation,
		reserve_location: Option<MultiLocation>,
		enabled_bridges: Vec<XBridge>,
		properties: AssetProperties,
	}

	impl From<MultiLocation> for XTransferAsset {
		fn from(x: MultiLocation) -> Self {
			XTransferAsset(x)
		}
	}

	impl From<XTransferAsset> for MultiLocation {
		fn from(x: XTransferAsset) -> Self {
			x.0
		}
	}

	pub trait AccountId32Conversion {
		fn into_account(self) -> [u8; 32];
	}

	impl AccountId32Conversion for MultiLocation {
		fn into_account(self) -> [u8; 32] {
			sp_io::hashing::blake2_256(&self.encode())
		}
	}

	impl AccountId32Conversion for XTransferAsset {
		fn into_account(self) -> [u8; 32] {
			sp_io::hashing::blake2_256(&self.0.encode())
		}
	}

	// Split `Reserve` location from the given MultiLocation.
	// The reserve location represent which chain the location belong to.
	// By finding the reserve location, we can also identity where an asset
	// comes from.
	pub trait ExtractReserveLocation {
		fn reserve_location(&self) -> Option<MultiLocation>;
	}

	impl ExtractReserveLocation for Junctions {
		fn reserve_location(&self) -> Option<MultiLocation> {
			match (self.at(0), self.at(1)) {
				(Some(GeneralKey(cb_key)), Some(GeneralIndex(chain_id)))
					if &cb_key == &CB_ASSET_KEY =>
				{
					Some(
						(
							0,
							X2(GeneralKey((&cb_key).to_vec()), GeneralIndex(*chain_id)),
						)
							.into(),
					)
				}
				_ => None,
			}
		}
	}

	impl ExtractReserveLocation for MultiLocation {
		fn reserve_location(&self) -> Option<MultiLocation> {
			match (self.parents, self.first_interior()) {
				// Sibling parachain
				(1, Some(Parachain(id))) => {
					let mut interior = self.interior.clone();
					// Remove Junction::Parachain
					interior.take_first();
					interior
						.reserve_location()
						.or(Some(MultiLocation::new(1, X1(Parachain(*id)))))
				}
				// Parent
				(1, _) => Some(MultiLocation::parent()),
				// Local
				(0, _) => self
					.interior
					.reserve_location()
					.or(Some(MultiLocation::here())),
				_ => None,
			}
		}
	}

	impl ExtractReserveLocation for XTransferAsset {
		fn reserve_location(&self) -> Option<MultiLocation> {
			self.0.reserve_location()
		}
	}

	impl XTransferAsset {
		pub fn into_rid(self, chain_id: u8) -> [u8; 32] {
			let mut rid = sp_io::hashing::blake2_256(&self.0.encode());
			rid[0] = chain_id;
			rid
		}
	}

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	#[pallet::storage_version(STORAGE_VERSION)]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: frame_system::Config + pallet_assets::Config {
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
		type AssetsCommitteeOrigin: EnsureOrigin<Self::Origin>;
		#[pallet::constant]
		type MinBalance: Get<<Self as pallet_assets::Config>::Balance>;
	}

	const STORAGE_VERSION: StorageVersion = StorageVersion::new(0);

	/// Mapping asset to corresponding asset id
	#[pallet::storage]
	#[pallet::getter(fn asset_to_id)]
	pub type IdByAssets<T: Config> =
		StorageMap<_, Twox64Concat, XTransferAsset, <T as pallet_assets::Config>::AssetId>;

	/// Mapping asset id to corresponding asset
	#[pallet::storage]
	#[pallet::getter(fn id_to_asset)]
	pub type AssetByIds<T: Config> =
		StorageMap<_, Twox64Concat, <T as pallet_assets::Config>::AssetId, XTransferAsset>;

	/// Mapping resource id to corresponding asset
	#[pallet::storage]
	#[pallet::getter(fn resource_id_to_asset)]
	pub type AssetByResourceIds<T: Config> = StorageMap<_, Twox64Concat, [u8; 32], XTransferAsset>;

	// Mapping asset id to corresponding registry info
	#[pallet::storage]
	#[pallet::getter(fn id_to_registry_info)]
	pub type RegistryInfoByIds<T: Config> =
		StorageMap<_, Twox64Concat, <T as pallet_assets::Config>::AssetId, AssetRegistryInfo>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// Asset is registerd.
		AssetRegistered {
			asset_id: <T as pallet_assets::Config>::AssetId,
			asset: XTransferAsset,
		},
		/// Asset is unregisterd.
		AssetUnRegistered {
			asset_id: <T as pallet_assets::Config>::AssetId,
			asset: XTransferAsset,
		},
		/// Asset enabled chainbridge.
		ChainbridgeEnabled {
			asset_id: <T as pallet_assets::Config>::AssetId,
			chain_id: u8,
			resource_id: [u8; 32],
		},
		/// Asset disabled chainbridge.
		ChainbridgeDisabled {
			asset_id: <T as pallet_assets::Config>::AssetId,
			chain_id: u8,
			resource_id: [u8; 32],
		},
	}

	#[pallet::error]
	pub enum Error<T> {
		AssetAlreadyExist,
		AssetNotRegistered,
		BridgeAlreadyEnabled,
		BridgeAlreadyDisabled,
	}

	#[pallet::call]
	impl<T: Config> Pallet<T>
	where
		T: pallet_assets::Config,
	{
		#[pallet::weight(195_000_000)]
		#[transactional]
		pub fn force_register_asset(
			origin: OriginFor<T>,
			asset: XTransferAsset,
			asset_id: T::AssetId,
			properties: AssetProperties,
			owner: <T::Lookup as StaticLookup>::Source,
		) -> DispatchResult {
			T::AssetsCommitteeOrigin::ensure_origin(origin.clone())?;
			// Ensure location has not been registered
			ensure!(
				IdByAssets::<T>::get(&asset) == None,
				Error::<T>::AssetAlreadyExist
			);
			// Ensure asset_id has not been registered
			ensure!(
				AssetByIds::<T>::get(&asset_id) == None,
				Error::<T>::AssetAlreadyExist
			);
			pallet_assets::pallet::Pallet::<T>::force_create(
				origin.clone(),
				asset_id,
				owner,
				true,
				T::MinBalance::get(),
			)?;
			IdByAssets::<T>::insert(&asset, asset_id);
			AssetByIds::<T>::insert(asset_id, &asset);

			RegistryInfoByIds::<T>::insert(
				asset_id,
				AssetRegistryInfo {
					location: asset.clone().into(),
					reserve_location: asset.reserve_location(),
					// Xcmp will be enabled when assets being registered.
					enabled_bridges: vec![XBridge {
						config: XBridgeConfig::Xcmp,
						metadata: Box::new(Vec::new()),
					}],
					properties: properties.clone(),
				},
			);
			pallet_assets::pallet::Pallet::<T>::force_set_metadata(
				origin,
				asset_id,
				properties.name,
				properties.symbol,
				properties.decimals,
				false,
			)?;

			Self::deposit_event(Event::AssetRegistered { asset_id, asset });
			Ok(())
		}

		/// Clean asset info stored in asset wrapper, not call pallet_assets::destory(),
		/// By cleaning them in current pallet, xcm and bridge transfering on this asset
		/// will not success anymore, we should call pallet_assets::destory() manually
		/// if we want to delete this asset from our chain
		#[pallet::weight(195_000_000)]
		#[transactional]
		pub fn force_unregister_asset(
			origin: OriginFor<T>,
			asset_id: T::AssetId,
		) -> DispatchResult {
			T::AssetsCommitteeOrigin::ensure_origin(origin)?;
			let asset = AssetByIds::<T>::get(&asset_id).ok_or(Error::<T>::AssetNotRegistered)?;
			let info =
				RegistryInfoByIds::<T>::get(&asset_id).ok_or(Error::<T>::AssetNotRegistered)?;

			AssetByIds::<T>::remove(&asset_id);
			IdByAssets::<T>::remove(&asset);

			// Unbind resource id and asset if have chain bridge set
			for bridge in info.enabled_bridges {
				if let XBridgeConfig::ChainBridge {
					chain_id: _,
					resource_id,
					..
				} = bridge.config
				{
					AssetByResourceIds::<T>::remove(&resource_id);
				}
			}
			// Delete registry info
			RegistryInfoByIds::<T>::remove(&asset_id);

			Self::deposit_event(Event::AssetUnRegistered { asset_id, asset });
			Ok(())
		}

		#[pallet::weight(195_000_000)]
		#[transactional]
		pub fn force_set_metadata(
			origin: OriginFor<T>,
			asset_id: T::AssetId,
			properties: AssetProperties,
		) -> DispatchResult {
			T::AssetsCommitteeOrigin::ensure_origin(origin.clone())?;

			let mut info =
				RegistryInfoByIds::<T>::get(&asset_id).ok_or(Error::<T>::AssetNotRegistered)?;
			info.properties = properties.clone();
			RegistryInfoByIds::<T>::insert(&asset_id, &info);
			pallet_assets::pallet::Pallet::<T>::force_set_metadata(
				origin,
				asset_id,
				properties.name,
				properties.symbol,
				properties.decimals,
				false,
			)?;

			Ok(())
		}

		#[pallet::weight(195_000_000)]
		#[transactional]
		pub fn force_enable_chainbridge(
			origin: OriginFor<T>,
			asset_id: T::AssetId,
			chain_id: u8,
			is_mintable: bool,
			metadata: Box<Vec<u8>>,
		) -> DispatchResult {
			T::AssetsCommitteeOrigin::ensure_origin(origin)?;
			let asset = AssetByIds::<T>::get(&asset_id).ok_or(Error::<T>::AssetNotRegistered)?;
			let mut info =
				RegistryInfoByIds::<T>::get(&asset_id).ok_or(Error::<T>::AssetNotRegistered)?;
			let resource_id: [u8; 32] = asset.clone().into_rid(chain_id);

			ensure!(
				AssetByResourceIds::<T>::get(&resource_id) == None,
				Error::<T>::BridgeAlreadyEnabled,
			);
			AssetByResourceIds::<T>::insert(&resource_id, &asset);
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

			Self::deposit_event(Event::ChainbridgeEnabled {
				asset_id,
				chain_id,
				resource_id,
			});
			Ok(())
		}

		#[pallet::weight(195_000_000)]
		#[transactional]
		pub fn force_disable_chainbridge(
			origin: OriginFor<T>,
			asset_id: T::AssetId,
			chain_id: u8,
		) -> DispatchResult {
			T::AssetsCommitteeOrigin::ensure_origin(origin)?;
			let asset = AssetByIds::<T>::get(&asset_id).ok_or(Error::<T>::AssetNotRegistered)?;
			let mut info =
				RegistryInfoByIds::<T>::get(&asset_id).ok_or(Error::<T>::AssetNotRegistered)?;
			let resource_id: [u8; 32] = asset.clone().into_rid(chain_id);

			ensure!(
				AssetByResourceIds::<T>::get(&resource_id).is_some(),
				Error::<T>::BridgeAlreadyDisabled,
			);
			AssetByResourceIds::<T>::remove(&resource_id);
			// Unbind resource id and asset
			if let Some(idx) = info
				.enabled_bridges
				.iter()
				.position(|item| match item.config {
					XBridgeConfig::ChainBridge {
						chain_id: cid,
						resource_id: rid,
						..
					} => cid == chain_id && rid == resource_id,
					_ => false,
				}) {
				info.enabled_bridges.remove(idx);
			}
			RegistryInfoByIds::<T>::insert(&asset_id, &info);

			Self::deposit_event(Event::ChainbridgeDisabled {
				asset_id,
				chain_id,
				resource_id,
			});
			Ok(())
		}
	}

	pub trait GetAssetRegistryInfo<AssetId> {
		fn id(asset: &XTransferAsset) -> Option<AssetId>;
		fn asset(id: &AssetId) -> Option<XTransferAsset>;
		fn lookup_by_resource_id(resource_id: &[u8; 32]) -> Option<XTransferAsset>;
		fn decimals(id: &AssetId) -> Option<u8>;
	}

	impl<T: Config> GetAssetRegistryInfo<<T as pallet_assets::Config>::AssetId> for Pallet<T> {
		fn id(asset: &XTransferAsset) -> Option<<T as pallet_assets::Config>::AssetId> {
			IdByAssets::<T>::get(asset)
		}

		fn asset(id: &<T as pallet_assets::Config>::AssetId) -> Option<XTransferAsset> {
			AssetByIds::<T>::get(id)
		}

		fn lookup_by_resource_id(resource_id: &[u8; 32]) -> Option<XTransferAsset> {
			AssetByResourceIds::<T>::get(resource_id)
		}

		fn decimals(id: &<T as pallet_assets::Config>::AssetId) -> Option<u8> {
			RegistryInfoByIds::<T>::get(&id).map(|m| m.properties.decimals)
		}
	}
}
