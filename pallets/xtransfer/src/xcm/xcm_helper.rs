pub use self::xcm_helper::*;

pub mod xcm_helper {
	use crate::pallet_assets_wrapper::{XTransferAsset, XTransferAssetInfo};
	use cumulus_primitives_core::ParaId;
	use frame_support::pallet_prelude::*;
	use sp_runtime::traits::CheckedConversion;
	use sp_std::{
		convert::{Into, TryFrom, TryInto},
		marker::PhantomData,
		result,
	};
	use xcm::latest::{
		prelude::*, AssetId::Concrete, Error as XcmError, Fungibility::Fungible, MultiAsset,
		MultiLocation, Result as XcmResult,
	};
	use xcm_executor::{
		traits::{
			Error as MatchError, FilterAssetLocation, MatchesFungible, MatchesFungibles,
			TransactAsset,
		},
		Assets,
	};

	const LOG_TARGET: &str = "xcm-helper";

	pub struct NativeAssetMatcher<C>(PhantomData<C>);
	impl<C: NativeAssetChecker, B: TryFrom<u128>> MatchesFungible<B> for NativeAssetMatcher<C> {
		fn matches_fungible(a: &MultiAsset) -> Option<B> {
			log::trace!(
				target: LOG_TARGET,
				"NativeAssetMatcher check fungible {:?}.",
				a.clone(),
			);
			match (&a.id, &a.fun) {
				(Concrete(_), Fungible(ref amount)) if C::is_native_asset(a) => {
					CheckedConversion::checked_from(*amount)
				}
				_ => None,
			}
		}
	}

	pub struct ConcreteAssetsMatcher<Assets, Balance, AssetsInfo>(
		PhantomData<(Assets, Balance, AssetsInfo)>,
	);
	impl<
			Assets: pallet_assets::Config,
			Balance: Clone + From<u128>,
			AssetsInfo: XTransferAssetInfo<Assets>,
		> MatchesFungibles<Assets::AssetId, Balance> for ConcreteAssetsMatcher<Assets, Balance, AssetsInfo>
	{
		fn matches_fungibles(a: &MultiAsset) -> result::Result<(Assets::AssetId, Balance), MatchError> {
			log::trace!(
				target: LOG_TARGET,
				"ConcreteAssetsMatcher check fungible {:?}.",
				a.clone(),
			);
			let (&amount, location) = match (&a.fun, &a.id) {
				(Fungible(ref amount), Concrete(ref id)) => (amount, id),
				_ => return Err(MatchError::AssetNotFound),
			};
			let xtransfer_asset: XTransferAsset = location
				.clone()
				.try_into()
				.map_err(|_| MatchError::AssetIdConversionFailed)?;
			let asset_id: Assets::AssetId =
				AssetsInfo::id(&xtransfer_asset).ok_or(MatchError::AssetNotFound)?;
			let amount = amount
				.try_into()
				.map_err(|_| MatchError::AmountToBalanceConversionFailed)?;
			Ok((asset_id.into(), amount))
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
		pub fn is_native_asset_id(id: &MultiLocation) -> bool {
			let native_locations = [
				MultiLocation::here(),
				(1, X1(Parachain(T::get().into()))).into(),
			];
			native_locations.contains(id)
		}
	}

	pub trait NativeAssetChecker {
		fn is_native_asset(asset: &MultiAsset) -> bool;
	}
	impl<T: Get<ParaId>> NativeAssetChecker for NativeAssetFilter<T> {
		fn is_native_asset(asset: &MultiAsset) -> bool {
			match (&asset.id, &asset.fun) {
				// So far our native asset is concrete
				(Concrete(ref id), Fungible(_)) if Self::is_native_asset_id(id) => true,
				_ => false,
			}
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

	pub struct XTransferAdapter<NativeAdapter, AssetsAdapter, NativeChecker>(
		PhantomData<(NativeAdapter, AssetsAdapter, NativeChecker)>,
	);

	impl<
			NativeAdapter: TransactAsset,
			AssetsAdapter: TransactAsset,
			NativeChecker: NativeAssetChecker,
		> TransactAsset for XTransferAdapter<NativeAdapter, AssetsAdapter, NativeChecker>
	{
		fn can_check_in(_origin: &MultiLocation, what: &MultiAsset) -> XcmResult {
			if NativeChecker::is_native_asset(what) {
				NativeAdapter::can_check_in(_origin, what)
			} else {
				AssetsAdapter::can_check_in(_origin, what)
			}
		}

		fn check_in(_origin: &MultiLocation, what: &MultiAsset) {
			if NativeChecker::is_native_asset(what) {
				NativeAdapter::check_in(_origin, what)
			} else {
				AssetsAdapter::check_in(_origin, what)
			}
		}

		fn check_out(_dest: &MultiLocation, what: &MultiAsset) {
			if NativeChecker::is_native_asset(what) {
				NativeAdapter::check_out(_dest, what)
			} else {
				AssetsAdapter::check_out(_dest, what)
			}
		}

		fn deposit_asset(what: &MultiAsset, who: &MultiLocation) -> XcmResult {
			if NativeChecker::is_native_asset(what) {
				NativeAdapter::deposit_asset(what, who)
			} else {
				AssetsAdapter::deposit_asset(what, who)
			}
		}

		fn withdraw_asset(
			what: &MultiAsset,
			who: &MultiLocation,
		) -> result::Result<Assets, XcmError> {
			if NativeChecker::is_native_asset(what) {
				NativeAdapter::withdraw_asset(what, who)
			} else {
				AssetsAdapter::withdraw_asset(what, who)
			}
		}
	}
}
