use crate::traits::*;
use assets_registry::{AccountId32Conversion, NativeAssetChecker, ReserveAssetChecker};
use sp_std::{convert::Into, marker::PhantomData, result, vec::Vec};
use xcm::latest::{prelude::*, Error as XcmError, MultiAsset, MultiLocation, Result as XcmResult};
use xcm_executor::{traits::TransactAsset, Assets};

const LOG_TARGET: &str = "runtime::fungbible-adapter";

pub struct XTransferAdapter<NativeAdapter, AssetsAdapter, Transactor, NativeChecker, ReserveChecker>(
	PhantomData<(
		NativeAdapter,
		AssetsAdapter,
		Transactor,
		NativeChecker,
		ReserveChecker,
	)>,
);

impl<
		NativeAdapter: TransactAsset,
		AssetsAdapter: TransactAsset,
		Transactor: OnWithdrawn + OnDeposited + OnForwarded,
		NativeChecker: NativeAssetChecker,
		ReserveChecker: ReserveAssetChecker,
	> TransactAsset
	for XTransferAdapter<NativeAdapter, AssetsAdapter, Transactor, NativeChecker, ReserveChecker>
{
	fn deposit_asset(what: &MultiAsset, who: &MultiLocation) -> XcmResult {
		// In case we got a local consensus location within asset, which may cause unexpected behaviors on EVM birdges,
		// always try to convert asset location into gloable consensus location first. But it's ok if conversion faild, because
		// only reserve assets (exclude PHA) need to do these stuff.
		let gloableconsensus_asset = ReserveChecker::to_gloableconsensus_asset(what);
		let what: &MultiAsset = if NativeChecker::is_native_asset(what) {
			what
		} else {
			&gloableconsensus_asset
		};

		match (who.parents, &who.interior) {
			// Deposit to local accounts or sibling parachain sovereign accounts
			(0, &X1(AccountId32 { .. })) | (1, &X1(Parachain(_))) => {
				log::trace!(
					target: LOG_TARGET,
					"deposit asset to local account, what: {:?}, who: {:?}.",
					what,
					who,
				);
				if NativeChecker::is_native_asset(what) {
					NativeAdapter::deposit_asset(what, who)?;
				} else {
					AssetsAdapter::deposit_asset(what, who)?;
				}
				Transactor::on_deposited(what.clone(), who.clone(), Vec::new())
					.map_err(|e| XcmError::FailedToTransactAsset(e.into()))?;
			}
			// Otherwise forward it through the bridge, if destination hasn't been bridged, error will be returned.
			_ => {
				log::trace!(
					target: LOG_TARGET,
					"forward asset to destination through another bridge, what: {:?}, who: {:?}.",
					what,
					who,
				);

				// Deposit asset into temporary account, then forward through other bridges
				let temporary_account = MultiLocation::new(
					0,
					X1(GeneralKey(
						b"bridge_transfer"
							.to_vec()
							.try_into()
							.expect("less than length limit; qed"),
					)),
				)
				.into_account();
				if NativeChecker::is_native_asset(what) {
					NativeAdapter::deposit_asset(
						what,
						&Junction::AccountId32 {
							network: NetworkId::Any,
							id: temporary_account,
						}
						.into(),
					)?;
				} else {
					AssetsAdapter::deposit_asset(
						what,
						&Junction::AccountId32 {
							network: NetworkId::Any,
							id: temporary_account,
						}
						.into(),
					)?;
				}

				Transactor::on_forwarded(what.clone(), who.clone(), Vec::new())
					.map_err(|e| XcmError::FailedToTransactAsset(e.into()))?;
			}
		}

		Ok(())
	}

	fn withdraw_asset(what: &MultiAsset, who: &MultiLocation) -> result::Result<Assets, XcmError> {
		// In case we got a local consensus location within asset, which may cause unexpected behaviors on EVM birdges,
		// always try to convert asset location into gloable consensus location first. But it's ok if conversion faild, because
		// only reserve assets (exclude PHA) need to do these stuff.
		let gloableconsensus_asset = ReserveChecker::to_gloableconsensus_asset(what);
		let what: &MultiAsset = if NativeChecker::is_native_asset(what) {
			what
		} else {
			&gloableconsensus_asset
		};

		log::trace!(
			target: LOG_TARGET,
			"withdraw asset from local account, what: {:?}, who: {:?}.",
			&what,
			&who,
		);
		let assets = if NativeChecker::is_native_asset(what) {
			NativeAdapter::withdraw_asset(what, who)?
		} else {
			AssetsAdapter::withdraw_asset(what, who)?
		};
		Transactor::on_withdrawn(what.clone(), who.clone(), Vec::new())
			.map_err(|e| return XcmError::FailedToTransactAsset(e.into()))?;
		Ok(assets)
	}
}
