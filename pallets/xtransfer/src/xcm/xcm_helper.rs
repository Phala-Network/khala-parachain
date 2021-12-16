pub use self::xcm_helper::*;

pub mod xcm_helper {
	use crate::bridge::pallet::{BridgeChainId, BridgeTransact};
	use crate::pallet_assets_wrapper::{XTransferAsset, XTransferAssetInfo};
	use cumulus_primitives_core::ParaId;
	use frame_support::pallet_prelude::*;
	use sp_core::U256;
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
	use xcm_builder::TakeRevenue;
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
			log::error!(
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

	pub struct ConcreteAssetsMatcher<AssetId, Balance, AssetsInfo>(
		PhantomData<(AssetId, Balance, AssetsInfo)>,
	);
	impl<AssetId, Balance: Clone + From<u128>, AssetsInfo: XTransferAssetInfo<AssetId>>
		MatchesFungibles<AssetId, Balance> for ConcreteAssetsMatcher<AssetId, Balance, AssetsInfo>
	{
		fn matches_fungibles(a: &MultiAsset) -> result::Result<(AssetId, Balance), MatchError> {
			log::error!(
				target: LOG_TARGET,
				"ConcreteAssetsMatcher check fungible {:?}.",
				a.clone(),
			);
			let (&amount, location) = match (&a.fun, &a.id) {
				(Fungible(ref amount), Concrete(ref id)) => (amount, id),
				_ => return Err(MatchError::AssetNotFound),
			};
			let xtransfer_asset: XTransferAsset = location.clone().into();
			let asset_id: AssetId =
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

	pub struct XTransferAdapter<NativeAdapter, AssetsAdapter, NativeChecker, BridgeTransactor>(
		PhantomData<(
			NativeAdapter,
			AssetsAdapter,
			NativeChecker,
			BridgeTransactor,
		)>,
	);

	impl<
			NativeAdapter: TransactAsset,
			AssetsAdapter: TransactAsset,
			NativeChecker: NativeAssetChecker,
			BridgeTransactor: BridgeTransact,
		> TransactAsset
		for XTransferAdapter<NativeAdapter, AssetsAdapter, NativeChecker, BridgeTransactor>
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
			log::error!(
				target: LOG_TARGET,
				"XTransferAdapter deposit_asset, what: {:?}, who: {:?}.",
				&what,
				&who,
			);

			match &who.interior {
				// destnation is foreign chain, forward it through bridge
				Junctions::X2(GeneralIndex(dest_id), GeneralKey(recipient)) => {
					let (&amount, location) = match (&what.fun, &what.id) {
						(Fungible(amount), Concrete(id)) => (amount, id),
						_ => return Err(XcmError::Unimplemented),
					};
					let xtransfer_asset: XTransferAsset = location.clone().into();
					let dest_id: BridgeChainId = dest_id
						.clone()
						.try_into()
						.expect("Convert from u128 to dest_id must be ok; qed.");
					BridgeTransactor::transfer_fungible(
						dest_id,
						xtransfer_asset.into(),
						recipient.to_vec(),
						U256::from(amount),
					)
					.map_err(|e| XcmError::FailedToTransactAsset(e.into()))?;

					Ok(())
				}
				// try handle it with transfer adapter
				_ => {
					if NativeChecker::is_native_asset(what) {
						NativeAdapter::deposit_asset(what, who)
					} else {
						AssetsAdapter::deposit_asset(what, who)
					}
				}
			}
		}

		fn withdraw_asset(
			what: &MultiAsset,
			who: &MultiLocation,
		) -> result::Result<Assets, XcmError> {
			log::error!(
				target: LOG_TARGET,
				"XTransferAdapter withdraw_asset, what: {:?}, who: {:?}.",
				&what,
				&who,
			);
			if NativeChecker::is_native_asset(what) {
				NativeAdapter::withdraw_asset(what, who)
			} else {
				AssetsAdapter::withdraw_asset(what, who)
			}
		}
	}

	pub struct XTransferTakeRevenue<TransferAdapter, AccountId, Beneficiary>(
		PhantomData<(TransferAdapter, AccountId, Beneficiary)>,
	);
	impl<
			TransferAdapter: TransactAsset,
			AccountId: From<[u8; 32]> + Into<[u8; 32]> + Clone,
			Beneficiary: Get<AccountId>,
		> TakeRevenue for XTransferTakeRevenue<TransferAdapter, AccountId, Beneficiary>
	{
		fn take_revenue(revenue: MultiAsset) {
			let beneficiary = MultiLocation::new(
				0,
				X1(AccountId32 {
					network: NetworkId::Any,
					id: Beneficiary::get().into(),
				}),
			);
			log::error!(
				target: LOG_TARGET,
				"XTransferTakeRevenue take_revenue, revenue: {:?}, beneficiary: {:?}.",
				&revenue,
				&beneficiary,
			);
			let _ = TransferAdapter::deposit_asset(&revenue, &beneficiary);
		}
	}
}
