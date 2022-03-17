use frame_support::{dispatch::DispatchResult, weights::Weight};
use sp_std::vec::Vec;
use xcm::latest::{MultiAsset, MultiLocation};

pub trait BridgeChecker {
	/// Return true if the asset can be send to destination
	fn can_deposit_asset(asset: MultiAsset, dest: MultiLocation) -> bool;
	/// Return true if the data can be send to destination
	fn can_send_data(data: &Vec<u8>, dest: MultiLocation) -> bool;
}

pub trait BridgeTransact {
	fn transfer_fungible(
		sender: [u8; 32],
		asset: MultiAsset,
		dest: MultiLocation,
		max_weight: Option<Weight>,
	) -> DispatchResult;

	fn transfer_nonfungible(
		sender: [u8; 32],
		asset: MultiAsset,
		dest: MultiLocation,
		max_weight: Option<Weight>,
	) -> DispatchResult;

	fn transfer_generic(
		sender: [u8; 32],
		data: &Vec<u8>,
		dest: MultiLocation,
		max_weight: Option<Weight>,
	) -> DispatchResult;
}

pub trait OnWithdrawn {
	fn on_withdrawn(what: MultiAsset, who: MultiLocation, memo: Vec<u8>) -> DispatchResult;
}

pub trait OnDeposited {
	fn on_deposited(what: MultiAsset, who: MultiLocation, memo: Vec<u8>) -> DispatchResult;
}

pub trait OnForwarded {
	fn on_forwarded(what: MultiAsset, who: MultiLocation, memo: Vec<u8>) -> DispatchResult;
}
pub trait NativeAssetChecker {
	fn is_native_asset(asset: &MultiAsset) -> bool;
	fn is_native_asset_location(id: &MultiLocation) -> bool;
	fn native_asset_location() -> MultiLocation;
}
