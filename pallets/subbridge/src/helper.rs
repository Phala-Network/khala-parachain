use assets_registry::{GetAssetRegistryInfo, NativeAssetChecker};
use frame_support::{pallet_prelude::*, traits::ContainsPair};
use sp_runtime::traits::CheckedConversion;
use sp_std::{
	convert::{Into, TryFrom, TryInto},
	marker::PhantomData,
	result,
};
use xcm::latest::{
	prelude::*, AssetId::Concrete, Fungibility::Fungible, MultiAsset, MultiLocation,
};
use xcm_builder::TakeRevenue;
use xcm_executor::traits::{Error as MatchError, MatchesFungible, MatchesFungibles, TransactAsset};

pub struct NativeAssetMatcher<C>(PhantomData<C>);
impl<C: NativeAssetChecker, B: TryFrom<u128>> MatchesFungible<B> for NativeAssetMatcher<C> {
	fn matches_fungible(a: &MultiAsset) -> Option<B> {
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
impl<AssetId, Balance: Clone + From<u128>, AssetsInfo: GetAssetRegistryInfo<AssetId>>
	MatchesFungibles<AssetId, Balance> for ConcreteAssetsMatcher<AssetId, Balance, AssetsInfo>
{
	fn matches_fungibles(a: &MultiAsset) -> result::Result<(AssetId, Balance), MatchError> {
		let (&amount, location) = match (&a.fun, &a.id) {
			(Fungible(ref amount), Concrete(ref id)) => (amount, id),
			_ => return Err(MatchError::AssetNotHandled),
		};
		let asset_id: AssetId = AssetsInfo::id(&location).ok_or(MatchError::AssetNotHandled)?;
		let amount = amount
			.try_into()
			.map_err(|_| MatchError::AmountToBalanceConversionFailed)?;
		Ok((asset_id.into(), amount))
	}
}

// We only trust the origin to send us assets that they identify as their
// sovereign assets.
pub struct AssetOriginFilter;
impl ContainsPair<MultiAsset, MultiLocation> for AssetOriginFilter {
	fn contains(asset: &MultiAsset, origin: &MultiLocation) -> bool {
		if let Some(ref id) = ConcrateAsset::origin(asset) {
			if id == origin {
				return true;
			}
		}
		false
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
				// Sibling parachain
				(1, Some(Parachain(id))) => Some(MultiLocation::new(1, X1(Parachain(*id)))),
				// Parent
				(1, _) => Some(MultiLocation::parent()),
				// Children parachain
				(0, Some(Parachain(id))) => Some(MultiLocation::new(0, X1(Parachain(*id)))),
				// Local: (0, Here)
				(0, None) => Some(id),
				_ => None,
			}
		})
	}
}

pub struct XTransferTakeRevenue<Adapter, AccountId, Beneficiary>(
	PhantomData<(Adapter, AccountId, Beneficiary)>,
);
impl<Adapter: TransactAsset, AccountId: Into<[u8; 32]> + Clone, Beneficiary: Get<AccountId>>
	TakeRevenue for XTransferTakeRevenue<Adapter, AccountId, Beneficiary>
{
	fn take_revenue(revenue: MultiAsset) {
		let beneficiary = MultiLocation::new(
			0,
			X1(AccountId32 {
				network: None,
				id: Beneficiary::get().into(),
			}),
		);
		let _ = Adapter::deposit_asset(
			&revenue,
			&beneficiary,
			// Put empty message hash here because we are not sending XCM message
			&XcmContext::with_message_hash([0; 32]),
		);
	}
}
