pub use self::pallet::*;

#[allow(unused_variables)]
#[frame_support::pallet]
pub mod pallet {
	use codec::{Decode, Encode};
	use frame_support::{dispatch::DispatchResult, pallet_prelude::*, traits::StorageVersion};
	use frame_system::pallet_prelude::*;
	use scale_info::TypeInfo;
	use sp_runtime::traits::StaticLookup;
	use sp_std::{
		convert::{From, TryFrom, TryInto},
		result,
	};
	use xcm::latest::MultiLocation;

	#[derive(Clone, Decode, Encode, Eq, PartialEq, Ord, PartialOrd, Debug, TypeInfo)]
	pub enum XTransferAsset {
		ParachainAsset(MultiLocation),
		SolochainAsset([u8; 32]),
	}

	impl TryFrom<MultiLocation> for XTransferAsset {
		type Error = ();
		fn try_from(x: MultiLocation) -> result::Result<Self, ()> {
			Ok(XTransferAsset::ParachainAsset(x))
		}
	}

	impl TryFrom<XTransferAsset> for MultiLocation {
		type Error = ();
		fn try_from(x: XTransferAsset) -> result::Result<Self, ()> {
			match x {
				XTransferAsset::ParachainAsset(location) => Ok(location),
				_ => Err(()),
			}
		}
	}

	impl TryFrom<[u8; 32]> for XTransferAsset {
		type Error = ();
		fn try_from(x: [u8; 32]) -> result::Result<Self, ()> {
			Ok(XTransferAsset::SolochainAsset(x))
		}
	}

	pub type XTransferAssetId = u32;

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	#[pallet::storage_version(STORAGE_VERSION)]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: frame_system::Config + pallet_assets::Config {
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
		type AssetsCommitteeOrigin: EnsureOrigin<Self::Origin>;
	}

	const STORAGE_VERSION: StorageVersion = StorageVersion::new(0);

	/// The number of total assets
	#[pallet::storage]
	#[pallet::getter(fn asset_count)]
	pub type AssetCount<T> = StorageValue<_, u32, ValueQuery>;

	/// Mapping asset to corresponding asset id
	#[pallet::storage]
	#[pallet::getter(fn asset_to_id)]
	pub type AssetToId<T: Config> = StorageMap<_, Twox64Concat, XTransferAsset, XTransferAssetId>;

	/// Mapping asset id to corresponding asset
	#[pallet::storage]
	#[pallet::getter(fn id_to_asset)]
	pub type IdToAsset<T: Config> = StorageMap<_, Twox64Concat, XTransferAssetId, XTransferAsset>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// Asset been registerd. \[id, asset\]
		ForceAssetRegistered(XTransferAssetId, XTransferAsset),
		/// Asset been unregisterd. \[id, asset\]
		ForceAssetUnregistered(XTransferAssetId, XTransferAsset),
	}

	#[pallet::error]
	pub enum Error<T> {
		AssetAlreadyExist,
	}

	#[pallet::call]
	impl<T: Config> Pallet<T>
	where
		T: pallet_assets::Config<AssetId = XTransferAssetId>,
	{
		#[pallet::weight(195_000_000)]
		pub fn force_register_asset(
			origin: OriginFor<T>,
			asset: XTransferAsset,
			owner: <T::Lookup as StaticLookup>::Source,
			#[pallet::compact] min_balance: T::Balance,
		) -> DispatchResult {
			T::AssetsCommitteeOrigin::ensure_origin(origin.clone())?;
			ensure!(
				AssetToId::<T>::get(&asset) == None,
				Error::<T>::AssetAlreadyExist
			);
			let id = AssetCount::<T>::get();
			let asset_id: XTransferAssetId = id.try_into().unwrap();
			pallet_assets::pallet::Pallet::<T>::force_create(
				origin,
				asset_id.clone(),
				owner,
				true,
				min_balance,
			)?;
			AssetToId::<T>::insert(&asset, &id);
			IdToAsset::<T>::insert(&id, &asset);
			AssetCount::<T>::put(id + 1);

			Self::deposit_event(Event::ForceAssetRegistered(asset_id, asset));
			Ok(())
		}

		/// Clean asset info stored in asset wrapper, not call pallet_assets::destory(),
		/// By cleaning them in current pallet, xcm and bridge transfering on this asset
		/// will not success anymore, we should call pallet_assets::destory() manually
		/// if we want to delete this asset from our chain
		#[pallet::weight(195_000_000)]
		pub fn force_unregister_asset(
			origin: OriginFor<T>,
			id: XTransferAssetId,
		) -> DispatchResult {
			T::AssetsCommitteeOrigin::ensure_origin(origin)?;
			if let Some(asset) = IdToAsset::<T>::get(&id) {
				IdToAsset::<T>::remove(&id);
				AssetToId::<T>::remove(&asset);
				Self::deposit_event(Event::ForceAssetUnregistered(id, asset));
			}
			Ok(())
		}
	}

	pub trait XTransferAssetInfo {
		fn id(asset: &XTransferAsset) -> Option<XTransferAssetId>;
		fn asset(id: &XTransferAssetId) -> Option<XTransferAsset>;
	}

	impl<T: Config> XTransferAssetInfo for Pallet<T> {
		fn id(asset: &XTransferAsset) -> Option<XTransferAssetId> {
			AssetToId::<T>::get(asset)
		}

		fn asset(id: &XTransferAssetId) -> Option<XTransferAsset> {
			IdToAsset::<T>::get(id)
		}
	}
}
