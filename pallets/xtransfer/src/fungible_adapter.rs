use crate::traits::*;
use assets_registry::AccountId32Conversion;
use sp_std::{convert::Into, marker::PhantomData, result, vec::Vec};
use xcm::latest::{prelude::*, Error as XcmError, MultiAsset, MultiLocation, Result as XcmResult};
use xcm_executor::{traits::TransactAsset, Assets};

const LOG_TARGET: &str = "runtime::fungbible-adapter";

pub struct XTransferAdapter<NativeAdapter, AssetsAdapter, Transactor, NativeChecker>(
	PhantomData<(NativeAdapter, AssetsAdapter, Transactor, NativeChecker)>,
);

impl<
		NativeAdapter: TransactAsset,
		AssetsAdapter: TransactAsset,
		Transactor: OnWithdrawn + OnDeposited + OnForwarded,
		NativeChecker: NativeAssetChecker,
	> TransactAsset for XTransferAdapter<NativeAdapter, AssetsAdapter, Transactor, NativeChecker>
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
				let temporary_account =
					MultiLocation::new(0, X1(GeneralKey(b"bridge_transfer".to_vec())))
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
