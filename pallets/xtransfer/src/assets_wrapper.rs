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
	use xcm::latest::MultiLocation;

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

	impl From<XTransferAsset> for [u8; 32] {
		fn from(x: XTransferAsset) -> Self {
			sp_io::hashing::keccak_256(&x.0.encode())
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
	pub type ResourceIdToAsset<T: Config> = StorageMap<_, Twox64Concat, [u8; 32], XTransferAsset>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// Asset is registerd. \[asset_id, asset\]
		AssetRegistered {
			asset_id: <T as pallet_assets::Config>::AssetId,
			asset: XTransferAsset,
		},
		/// Asset is unregisterd. \[asset_id, asset\]
		AssetUnRegistered {
			asset_id: <T as pallet_assets::Config>::AssetId,
			asset: XTransferAsset,
		},
	}

	#[pallet::error]
	pub enum Error<T> {
		AssetAlreadyExist,
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
			let resource_id: [u8; 32] = asset.clone().into();
			AssetToId::<T>::insert(&asset, asset_id);
			IdToAsset::<T>::insert(asset_id, &asset);
			ResourceIdToAsset::<T>::insert(&resource_id, &asset);

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
				let resource_id: [u8; 32] = asset.clone().into();
				IdToAsset::<T>::remove(&asset_id);
				AssetToId::<T>::remove(&asset);
				ResourceIdToAsset::<T>::remove(&resource_id);

				Self::deposit_event(Event::AssetUnRegistered { asset_id, asset });
			}
			Ok(())
		}
	}

	pub trait XTransferAssetInfo<AssetId> {
		fn id(asset: &XTransferAsset) -> Option<AssetId>;
		fn asset(id: &AssetId) -> Option<XTransferAsset>;
		// expect a better name
		fn from_resource_id(resource_id: &[u8; 32]) -> Option<XTransferAsset>;
	}

	impl<T: Config> XTransferAssetInfo<<T as pallet_assets::Config>::AssetId> for Pallet<T> {
		fn id(asset: &XTransferAsset) -> Option<<T as pallet_assets::Config>::AssetId> {
			AssetToId::<T>::get(asset)
		}

		fn asset(id: &<T as pallet_assets::Config>::AssetId) -> Option<XTransferAsset> {
			IdToAsset::<T>::get(id)
		}

		fn from_resource_id(resource_id: &[u8; 32]) -> Option<XTransferAsset> {
			ResourceIdToAsset::<T>::get(resource_id)
		}
	}
}
