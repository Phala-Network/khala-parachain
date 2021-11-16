pub use self::xcm_helper::*;

pub mod xcm_helper {
	use cumulus_primitives_core::ParaId;
	use frame_support::{pallet_prelude::*, traits::Contains};
	use sp_runtime::traits::CheckedConversion;
	use sp_std::{convert::TryFrom, marker::PhantomData, vec::Vec};
	use xcm::latest::{
		prelude::*,
		AssetId::{Abstract, Concrete},
		Fungibility::Fungible,
		MultiAsset, MultiLocation,
	};
	use xcm_executor::traits::{FilterAssetLocation, MatchesFungible};

	const LOG_TARGET: &str = "xcm-helper";

	pub struct IsSiblingParachainsConcrete<T>(PhantomData<T>);
	impl<T: Contains<MultiLocation>, B: TryFrom<u128>> MatchesFungible<B>
		for IsSiblingParachainsConcrete<T>
	{
		fn matches_fungible(a: &MultiAsset) -> Option<B> {
			log::trace!(
				target: LOG_TARGET,
				"IsSiblingParachainsConcrete check fungible {:?}.",
				a.clone(),
			);
			match (&a.id, &a.fun) {
				(Concrete(ref id), Fungible(ref amount)) if T::contains(id) => {
					CheckedConversion::checked_from(*amount)
				}
				_ => None,
			}
		}
	}

	pub struct IsSiblingParachainsAbstract<T>(PhantomData<T>);
	impl<T: Contains<Vec<u8>>, B: TryFrom<u128>> MatchesFungible<B> for IsSiblingParachainsAbstract<T> {
		fn matches_fungible(a: &MultiAsset) -> Option<B> {
			log::trace!(
				target: LOG_TARGET,
				"IsSiblingParachainsAbstract check fungible {:?}.",
				a.clone(),
			);
			match (&a.id, &a.fun) {
				(Abstract(ref id), Fungible(ref amount)) if T::contains(&id) => {
					CheckedConversion::checked_from(*amount)
				}
				_ => None,
			}
		}
	}

	// We only trust the origin to send us assets that they identify as their
	// sovereign assets.
	pub struct AssetOriginFilter;
	impl FilterAssetLocation for AssetOriginFilter {
		fn filter_asset_location(asset: &MultiAsset, origin: &MultiLocation) -> bool {
			if let Some(ref id) = ConcrateAsset::origin(asset) {
				if id == origin {
					return true;
				}
			}
			false
		}
	}

	pub struct NativeAssetFilter<T>(PhantomData<T>);
	impl<T: Get<ParaId>> NativeAssetFilter<T> {
		pub fn is_native_asset(asset: &MultiAsset) -> bool {
			match (&asset.id, &asset.fun) {
				// So far our native asset is concrete
				(Concrete(ref id), Fungible(_)) if Self::is_native_asset_id(id) => true,
				_ => false,
			}
		}

		pub fn is_native_asset_id(id: &MultiLocation) -> bool {
			let native_locations = [
				MultiLocation::here(),
				(1, X1(Parachain(T::get().into()))).into(),
			];
			native_locations.contains(id)
		}
	}

	pub struct ConcrateAsset;
	impl ConcrateAsset {
		pub fn id(asset: &MultiAsset) -> Option<MultiLocation> {
			match (&asset.id, &asset.fun) {
				// So far our native asset is concrete
				(Concrete(id), Fungible(_)) => Some(id.clone()),
				_ => None,
			}
		}

		pub fn origin(asset: &MultiAsset) -> Option<MultiLocation> {
			Self::id(asset).and_then(|id| {
				match (id.parents, id.first_interior()) {
					// sibling parachain
					(1, Some(Parachain(id))) => Some(MultiLocation::new(1, X1(Parachain(*id)))),
					// parent
					(1, _) => Some(MultiLocation::parent()),
					// children parachain
					(0, Some(Parachain(id))) => Some(MultiLocation::new(0, X1(Parachain(*id)))),
					// local: (0, Here)
					(0, None) => Some(id),
					_ => None,
				}
			})
		}
	}
}
