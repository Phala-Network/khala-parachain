pub use self::xcm_helper::*;

pub mod xcm_helper {
	use crate::bridge::pallet::{BridgeChainId, BridgeTransact};
	use crate::bridge_transfer::pallet::GetBridgeFee;
	use crate::pallet_assets_wrapper::{
		AccountId32Conversion, ExtractReserveLocation, GetAssetRegistryInfo, XTransferAsset,
		CB_ASSET_KEY,
	};
	use crate::xcm::xcm_transfer::{XcmOnDeposited, XcmOnWithdrawn};
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

	const LOG_TARGET: &str = "runtime::xcm-helper";

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

	pub struct ConcreteAssetsMatcher<AssetId, Balance, AssetsInfo>(
		PhantomData<(AssetId, Balance, AssetsInfo)>,
	);
	impl<AssetId, Balance: Clone + From<u128>, AssetsInfo: GetAssetRegistryInfo<AssetId>>
		MatchesFungibles<AssetId, Balance> for ConcreteAssetsMatcher<AssetId, Balance, AssetsInfo>
	{
		fn matches_fungibles(a: &MultiAsset) -> result::Result<(AssetId, Balance), MatchError> {
			log::trace!(
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

	pub trait NativeAssetChecker {
		fn is_native_asset(asset: &MultiAsset) -> bool;
		fn is_native_asset_id(id: &MultiLocation) -> bool;
		fn native_asset_id() -> MultiLocation;
	}

	pub struct NativeAssetFilter<T>(PhantomData<T>);
	impl<T: Get<ParaId>> NativeAssetChecker for NativeAssetFilter<T> {
		fn is_native_asset(asset: &MultiAsset) -> bool {
			match (&asset.id, &asset.fun) {
				// So far our native asset is concrete
				(Concrete(ref id), Fungible(_)) if Self::is_native_asset_id(id) => true,
				_ => false,
			}
		}

		fn is_native_asset_id(id: &MultiLocation) -> bool {
			let native_locations = [
				MultiLocation::here(),
				(1, X1(Parachain(T::get().into()))).into(),
			];
			native_locations.contains(id)
		}

		fn native_asset_id() -> MultiLocation {
			(1, X1(Parachain(T::get().into()))).into()
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

	pub struct XTransferAdapter<
		NativeAdapter,
		AssetsAdapter,
		DepositHandler,
		WithdrawHandler,
		NativeChecker,
		BridgeTransactor,
		BridgeFeeInfo,
		AccountId,
		Treasury,
	>(
		PhantomData<(
			NativeAdapter,
			AssetsAdapter,
			DepositHandler,
			WithdrawHandler,
			NativeChecker,
			BridgeTransactor,
			BridgeFeeInfo,
			AccountId,
			Treasury,
		)>,
	);

	impl<
			NativeAdapter: TransactAsset,
			AssetsAdapter: TransactAsset,
			DepositHandler: XcmOnDeposited,
			WithdrawHandler: XcmOnWithdrawn,
			NativeChecker: NativeAssetChecker,
			BridgeTransactor: BridgeTransact,
			BridgeFeeInfo: GetBridgeFee,
			AccountId: From<[u8; 32]> + Into<[u8; 32]> + Clone,
			Treasury: Get<AccountId>,
		> TransactAsset
		for XTransferAdapter<
			NativeAdapter,
			AssetsAdapter,
			DepositHandler,
			WithdrawHandler,
			NativeChecker,
			BridgeTransactor,
			BridgeFeeInfo,
			AccountId,
			Treasury,
		>
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
			log::trace!(
				target: LOG_TARGET,
				"XTransferAdapter deposit_asset, what: {:?}, who: {:?}.",
				&what,
				&who,
			);

			match (who.parents, &who.interior) {
				// Destnation is a foreign chain. Forward it through the bridge
				(
					0,
					Junctions::X3(GeneralKey(cb_key), GeneralIndex(dest_id), GeneralKey(recipient)),
				) => {
					ensure!(
						&cb_key == &CB_ASSET_KEY,
						XcmError::FailedToTransactAsset("DismatchPath")
					);
					let (&amount, location) = match (&what.fun, &what.id) {
						(Fungible(amount), Concrete(id)) => (amount, id),
						_ => return Err(XcmError::Unimplemented),
					};
					let dest_id: BridgeChainId = dest_id
						.clone()
						.try_into()
						.map_err(|_| XcmError::FailedToTransactAsset("ChainIdConversionFailed"))?;

					// Deduct some fees if assets would be forwarded to solo chains.
					let fee = BridgeFeeInfo::get_fee(dest_id, what)
						.ok_or(XcmError::FailedToTransactAsset("FailedGetFee"))?;
					ensure!(
						amount > fee,
						XcmError::FailedToTransactAsset("Insufficient")
					);
					let fee_asset: MultiAsset = (location.clone(), fee).into();
					// Transfer fee to treasury
					let treasury = MultiLocation::new(
						0,
						X1(AccountId32 {
							network: NetworkId::Any,
							id: Treasury::get().into(),
						}),
					);
					if NativeChecker::is_native_asset(&fee_asset) {
						NativeAdapter::deposit_asset(&fee_asset, &treasury)
							.map_err(|_| XcmError::FailedToTransactAsset("FeeTransferFailed"))?;
					} else {
						AssetsAdapter::deposit_asset(&fee_asset, &treasury)
							.map_err(|_| XcmError::FailedToTransactAsset("FeeTransferFailed"))?;
					}

					// transfering_amount > 0
					let transfering_amount = amount - fee;
					let transfering_asset: MultiAsset =
						(location.clone(), transfering_amount).into();
					let dest_reserve: MultiLocation = (
						0,
						X2(
							GeneralKey(CB_ASSET_KEY.to_vec()),
							GeneralIndex(dest_id as u128),
						),
					)
						.into();
					let asset_reserve_location = location
						.reserve_location()
						.ok_or(XcmError::FailedToTransactAsset("FailedGetreserve"))?;
					// If we are forwarding asset to its non-reserve destination, deposit assets
					// to reserve account first
					if asset_reserve_location != dest_reserve {
						log::trace!(
							target: LOG_TARGET,
							"XTransferAdapter, reserve of asset and dest dismatch, deposit asset to reserve account.",
						);
						let reserve_account: MultiLocation = (
							0,
							X1(AccountId32 {
								network: NetworkId::Any,
								id: dest_reserve.into_account().into(),
							}),
						)
							.into();

						if NativeChecker::is_native_asset(&transfering_asset) {
							NativeAdapter::deposit_asset(&transfering_asset, &reserve_account)
								.map_err(|_| {
									XcmError::FailedToTransactAsset("ReserveTransferFailed")
								})?;
						} else {
							AssetsAdapter::deposit_asset(&transfering_asset, &reserve_account)
								.map_err(|_| {
									XcmError::FailedToTransactAsset("ReserveTransferFailed")
								})?;
						}
					}

					// Currently, we use chainbridge forwards transfer to solo chain, which use resource id
					// to indicate an asset.
					//
					// When asset is native token, e.g. PHA, we need transfer the location to MultiLocation::here()
					// from (1, X1(Parachain(id))) to match resource id registered in solo chains.
					let rid = if NativeChecker::is_native_asset(&transfering_asset) {
						XTransferAsset(MultiLocation::here()).into_rid(dest_id)
					} else {
						let xtransfer_asset: XTransferAsset = location.clone().into();
						xtransfer_asset.into_rid(dest_id)
					};

					// This operation will not do real transfer, it just emits FungibleTransfer event
					// to notify relayers submit proposal to our bridge contract that deployed on EVM chains.
					BridgeTransactor::transfer_fungible(
						dest_id,
						rid,
						recipient.to_vec(),
						U256::from(transfering_amount),
					)
					.map_err(|e| XcmError::FailedToTransactAsset(e.into()))?;
				}
				// Try handle it with transfer adapter
				_ => {
					if NativeChecker::is_native_asset(what) {
						NativeAdapter::deposit_asset(what, who).map_err(|e| return e)?;
					} else {
						AssetsAdapter::deposit_asset(what, who).map_err(|e| return e)?;
					}
				}
			}

			DepositHandler::on_deposited(what.clone(), who.clone())
				.map_err(|e| XcmError::FailedToTransactAsset(e.into()))?;
			Ok(())
		}

		fn withdraw_asset(
			what: &MultiAsset,
			who: &MultiLocation,
		) -> result::Result<Assets, XcmError> {
			log::trace!(
				target: LOG_TARGET,
				"XTransferAdapter withdraw_asset, what: {:?}, who: {:?}.",
				&what,
				&who,
			);
			let assets = if NativeChecker::is_native_asset(what) {
				NativeAdapter::withdraw_asset(what, who).map_err(|e| return e)?
			} else {
				AssetsAdapter::withdraw_asset(what, who).map_err(|e| return e)?
			};
			WithdrawHandler::on_withdrawn(what.clone(), who.clone())
				.map_err(|e| return XcmError::FailedToTransactAsset(e.into()))?;
			Ok(assets)
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
			log::trace!(
				target: LOG_TARGET,
				"XTransferTakeRevenue take_revenue, revenue: {:?}, beneficiary: {:?}.",
				&revenue,
				&beneficiary,
			);
			let _ = TransferAdapter::deposit_asset(&revenue, &beneficiary);
		}
	}
}
