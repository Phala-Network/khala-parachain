use frame_support::{dispatch::DispatchResult, weights::Weight};
use xcm::latest::{prelude::*, MultiAsset, MultiLocation};

pub trait BridgeTransact {
    fn can_deposit_asset(
        asset: MultiAsset,
        dest: MultiLocation,
    ) -> bool;wz

    fn can_send_data(
        data: &Vec<u8>,
        dest: MultiLocation
    ) -> bool;

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
