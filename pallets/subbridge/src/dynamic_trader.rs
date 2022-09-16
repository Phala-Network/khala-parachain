use assets_registry::GetAssetRegistryInfo;
use frame_support::traits::Get;
use sp_std::{marker::PhantomData, result::Result, vec::Vec};
use xcm::latest::{prelude::*, AssetId, Weight as XCMWeight};
use xcm_builder::TakeRevenue;
use xcm_executor::{traits::WeightTrader, Assets};

pub struct DynamicWeightTrader<
	WeightPerSecond,
	FungibleAssetId,
	FungibleAssetsInfo: GetAssetRegistryInfo<FungibleAssetId>,
	R: TakeRevenue,
>(
	XCMWeight,
	u128,
	Option<(AssetId, u128)>,
	PhantomData<(WeightPerSecond, FungibleAssetId, FungibleAssetsInfo, R)>,
);
impl<WeightPerSecond, FungibleAssetId, FungibleAssetsInfo, R> WeightTrader
	for DynamicWeightTrader<WeightPerSecond, FungibleAssetId, FungibleAssetsInfo, R>
where
	WeightPerSecond: Get<u64>,
	FungibleAssetsInfo: GetAssetRegistryInfo<FungibleAssetId>,
	R: TakeRevenue,
{
	fn new() -> Self {
		Self(0, 0, None, PhantomData)
	}

	fn buy_weight(&mut self, weight: XCMWeight, payment: Assets) -> Result<Assets, XcmError> {
		log::trace!(
			target: "xtranfer::weight",
			"DynamicWeightTrader::buy_weight weight: {:?}, payment: {:?}",
			weight, payment.clone(),
		);

		let payment_assets: Vec<MultiAsset> = payment.clone().into();
		let mut last_error = None;
		for payment_asset in payment_assets.iter() {
			match (&payment_asset.id, &payment_asset.fun) {
				(Concrete(ref location), Fungible(_)) => {
					// We found an asset that can be pay as fee from the registered asset list
					if let Some((id, units_per_second)) = FungibleAssetsInfo::price(&location) {
						let amount =
							units_per_second * (weight as u128) / (WeightPerSecond::get() as u128);
						if amount == 0 {
							return Ok(payment.clone());
						}

						// Note unused must contain asset that not used to pay fee, so here we deduct fee
						// from `payment` rather than `payment_asset`
						match payment.clone().checked_sub((id.clone(), amount).into()) {
							Ok(unused) => {
								self.0 = self.0.saturating_add(weight);
								self.1 = self.1.saturating_add(amount);
								self.2 = Some((id.clone(), units_per_second));
								log::trace!(
									target: "xtranfer::weight",
									"DynamicWeightTrader::successfully by weight: {:?}, amount: {:?}, asset id: {:?}",
									weight, amount, id,
								);
								return Ok(unused);
							}
							Err(_) => last_error = Some(XcmError::TooExpensive),
						}
					} else {
						last_error = Some(XcmError::TooExpensive);
					}
				}
				// Only fungible assets can be used to by weight
				_ => last_error = Some(XcmError::TooExpensive),
			}
		}

		Err(last_error.unwrap_or(XcmError::AssetNotFound))
	}

	fn refund_weight(&mut self, weight: XCMWeight) -> Option<MultiAsset> {
		log::trace!(target: "xtranfer::weight", "DynamicWeightTrader::refund_weight weight: {:?}", weight);

		// If we have deducted some fee from payment assets
		if let Some((id, units_per_second)) = &self.2 {
			let weight = weight.min(self.0);
			let amount = units_per_second * (weight as u128) / (WeightPerSecond::get() as u128);
			self.0 -= weight;
			self.1 = self.1.saturating_sub(amount);
			if amount > 0 {
				Some((id.clone(), amount).into())
			} else {
				None
			}
		} else {
			None
		}
	}
}

impl<WeightPerSecond, FungibleAssetId, FungibleAssetsInfo, R> Drop
	for DynamicWeightTrader<WeightPerSecond, FungibleAssetId, FungibleAssetsInfo, R>
where
	FungibleAssetsInfo: GetAssetRegistryInfo<FungibleAssetId>,
	R: TakeRevenue,
{
	fn drop(&mut self) {
		if self.1 > 0 {
			let (id, _) = self
				.2
				.clone()
				.expect("Unexpected weight payment result; qed.");
			R::take_revenue((id, self.1).into());
		}
	}
}
