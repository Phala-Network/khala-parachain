pub use self::pallet::*;

#[allow(unused_variables)]
#[frame_support::pallet]
pub mod pallet {
	use codec::{Decode, Encode};
	use frame_support::{dispatch::DispatchResult, pallet_prelude::*, traits::StorageVersion};
	use frame_system::pallet_prelude::*;
	use scale_info::TypeInfo;
	use sp_runtime::traits::StaticLookup;
	use sp_std::convert::From;
	use xcm::latest::{prelude::*, MultiLocation};

	// Junction used to indicate chainbridge assets. str "cb"
	pub const CB_ASSET_KEY: &[u8] = &[0x63, 0x62];

	#[derive(Clone, Decode, Encode, Eq, PartialEq, Ord, PartialOrd, Debug, TypeInfo)]
	pub struct XTransferAsset(MultiLocation);

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
	pub trait ReserveLocation {
		fn reserve(&self) -> Option<MultiLocation>;
	}

	impl ReserveLocation for MultiLocation {
		fn reserve(&self) -> Option<MultiLocation> {
			match self.first_interior() {
				Some(Parachain(para_id)) => {
					match self.interior.at(1) {
						// identify solo chain
						Some(GeneralKey(cb_key)) => {
							if cb_key == &CB_ASSET_KEY.to_vec() {
								//self.interior.at(2) should contains solo chain id
								match self.interior.at(2) {
									Some(GeneralIndex(chain_id)) => {
										return Some(
											(
												0,
												X2(
													GeneralKey((&cb_key).to_vec()),
													GeneralIndex(*chain_id),
												),
											)
												.into(),
										);
									}
									_ => {
										return None;
									}
								}
							} else {
								// maybe assets from other chain
								return Some((self.parents, Parachain(*para_id)).into());
							}
						}
						_ => {
							return Some((self.parents, Parachain(*para_id)).into());
						}
					}
				}
				_ => {
					// location not contains Junction::Parachain
					match self.interior() {
						Here | X1(AccountId32 { network: _, id: _ }) => {
							return Some((self.parents, Here).into());
						}
						X3(GeneralKey(cb_key), GeneralIndex(evm_chain_id), GeneralKey(_)) => {
							if cb_key == &CB_ASSET_KEY.to_vec() {
								// identify solo chain
								return Some(
									(
										0,
										X2(
											GeneralKey((&cb_key).to_vec()),
											GeneralIndex(*evm_chain_id),
										),
									)
										.into(),
								);
							} else {
								return None;
							}
						}
						_ => {
							return None;
						}
					}
				}
			}
		}
	}

	impl ReserveLocation for XTransferAsset {
		fn reserve(&self) -> Option<MultiLocation> {
			self.0.reserve()
		}
	}

	impl XTransferAsset {
		pub fn into_rid(self, chain_id: u8) -> [u8; 32] {
			let mut rid = sp_io::hashing::keccak_256(&self.0.encode());
			rid[0] = chain_id;
			rid
		}
	}

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	#[pallet::storage_version(STORAGE_VERSION)]
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
	pub type AssetToId<T: Config> =
		StorageMap<_, Twox64Concat, XTransferAsset, <T as pallet_assets::Config>::AssetId>;

	/// Mapping asset id to corresponding asset
	#[pallet::storage]
	#[pallet::getter(fn id_to_asset)]
	pub type IdToAsset<T: Config> =
		StorageMap<_, Twox64Concat, <T as pallet_assets::Config>::AssetId, XTransferAsset>;

	/// Mapping resource id to corresponding asset
	#[pallet::storage]
	#[pallet::getter(fn resource_id_to_asset)]
	pub type ResourceIdToAssets<T: Config> = StorageMap<_, Twox64Concat, [u8; 32], XTransferAsset>;

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
		/// Asset setup for a solo chain. \[asset_id, chain_id, rid]
		SolochainSetuped {
			asset_id: <T as pallet_assets::Config>::AssetId,
			chain_id: u8,
			rid: [u8; 32],
		},
	}

	#[pallet::error]
	pub enum Error<T> {
		AssetAlreadyExist,
		SolochainAlreadySetted,
	}

	#[pallet::call]
	impl<T: Config> Pallet<T>
	where
		T: pallet_assets::Config,
	{
		#[pallet::weight(195_000_000)]
		pub fn force_register_asset(
			origin: OriginFor<T>,
			asset: XTransferAsset,
			asset_id: T::AssetId,
			owner: <T::Lookup as StaticLookup>::Source,
		) -> DispatchResult {
			T::AssetsCommitteeOrigin::ensure_origin(origin.clone())?;
			// ensure location has not been registered
			ensure!(
				AssetToId::<T>::get(&asset) == None,
				Error::<T>::AssetAlreadyExist
			);
			// ensure asset_id has not been registered
			ensure!(
				IdToAsset::<T>::get(&asset_id) == None,
				Error::<T>::AssetAlreadyExist
			);
			pallet_assets::pallet::Pallet::<T>::force_create(
				origin,
				asset_id,
				owner,
				true,
				T::MinBalance::get(),
			)?;
			AssetToId::<T>::insert(&asset, asset_id);
			IdToAsset::<T>::insert(asset_id, &asset);

			Self::deposit_event(Event::AssetRegistered { asset_id, asset });
			Ok(())
		}

		/// Clean asset info stored in asset wrapper, not call pallet_assets::destory(),
		/// By cleaning them in current pallet, xcm and bridge transfering on this asset
		/// will not success anymore, we should call pallet_assets::destory() manually
		/// if we want to delete this asset from our chain
		#[pallet::weight(195_000_000)]
		pub fn force_unregister_asset(
			origin: OriginFor<T>,
			asset_id: T::AssetId,
		) -> DispatchResult {
			T::AssetsCommitteeOrigin::ensure_origin(origin)?;
			if let Some(asset) = IdToAsset::<T>::get(&asset_id) {
				IdToAsset::<T>::remove(&asset_id);
				AssetToId::<T>::remove(&asset);

				Self::deposit_event(Event::AssetUnRegistered { asset_id, asset });
			}
			Ok(())
		}

		#[pallet::weight(195_000_000)]
		pub fn force_setup_solochain(
			origin: OriginFor<T>,
			asset_id: T::AssetId,
			chain_id: u8,
		) -> DispatchResult {
			T::AssetsCommitteeOrigin::ensure_origin(origin)?;
			if let Some(asset) = IdToAsset::<T>::get(&asset_id) {
				let rid: [u8; 32] = asset.clone().into_rid(chain_id);
				ensure!(
					ResourceIdToAssets::<T>::get(&rid) == None,
					Error::<T>::SolochainAlreadySetted
				);
				ResourceIdToAssets::<T>::insert(&rid, &asset);
				Self::deposit_event(Event::SolochainSetuped {
					asset_id,
					chain_id,
					rid,
				});
			}
			Ok(())
		}
	}

	pub trait XTransferAssetInfo<AssetId> {
		fn id(asset: &XTransferAsset) -> Option<AssetId>;
		fn asset(id: &AssetId) -> Option<XTransferAsset>;
		// expect a better name
		fn lookup_by_resource_id(resource_id: &[u8; 32]) -> Option<XTransferAsset>;
	}

	impl<T: Config> XTransferAssetInfo<<T as pallet_assets::Config>::AssetId> for Pallet<T> {
		fn id(asset: &XTransferAsset) -> Option<<T as pallet_assets::Config>::AssetId> {
			AssetToId::<T>::get(asset)
		}

		fn asset(id: &<T as pallet_assets::Config>::AssetId) -> Option<XTransferAsset> {
			IdToAsset::<T>::get(id)
		}

		fn lookup_by_resource_id(resource_id: &[u8; 32]) -> Option<XTransferAsset> {
			ResourceIdToAssets::<T>::get(resource_id)
		}
	}
}
