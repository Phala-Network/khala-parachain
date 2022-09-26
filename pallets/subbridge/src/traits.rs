use frame_support::dispatch::{DispatchError, DispatchResult};
use sp_std::vec::Vec;
use xcm::latest::{MultiAsset, MultiLocation, Weight as XCMWeight};

pub trait BridgeChecker {
	/// Return true if the asset can be send to destination
	fn can_deposit_asset(asset: MultiAsset, dest: MultiLocation) -> bool;
	/// Return true if the data can be send to destination
	fn can_send_data(data: &Vec<u8>, dest: MultiLocation) -> bool;
}

pub trait BridgeTransact: Sized {
	fn new() -> Self;

	fn transfer_fungible(
		&self,
		sender: [u8; 32],
		asset: MultiAsset,
		dest: MultiLocation,
		max_weight: Option<XCMWeight>,
	) -> DispatchResult;

	fn transfer_nonfungible(
		&self,
		sender: [u8; 32],
		asset: MultiAsset,
		dest: MultiLocation,
		max_weight: Option<XCMWeight>,
	) -> DispatchResult;

	fn transfer_generic(
		&self,
		sender: [u8; 32],
		data: &Vec<u8>,
		dest: MultiLocation,
		max_weight: Option<XCMWeight>,
	) -> DispatchResult;
}

#[impl_trait_for_tuples::impl_for_tuples(30)]
impl BridgeTransact for Tuple {
	fn new() -> Self {
		for_tuples!( ( #( Tuple::new() ),* ) )
	}

	fn transfer_fungible(
		&self,
		sender: [u8; 32],
		asset: MultiAsset,
		dest: MultiLocation,
		max_weight: Option<XCMWeight>,
	) -> DispatchResult {
		let mut last_error = None;
		for_tuples!( #(
            match Tuple.transfer_fungible(sender, asset.clone(), dest.clone(), max_weight) {
                Ok(()) => return Ok(()),
                Err(e) => { last_error = Some(e) }
            }
        )* );
		let last_error = last_error.unwrap_or(DispatchError::Other("TransactFailed"));
		log::trace!(target: "runtime::BridgeTransact::transfer_fungible", "last_error: {:?}", last_error);
		Err(last_error)
	}

	fn transfer_nonfungible(
		&self,
		sender: [u8; 32],
		asset: MultiAsset,
		dest: MultiLocation,
		max_weight: Option<XCMWeight>,
	) -> DispatchResult {
		let mut last_error = None;
		for_tuples!( #(
            match Tuple.transfer_nonfungible(sender, asset.clone(), dest.clone(), max_weight) {
                Ok(()) => return Ok(()),
                Err(e) => { last_error = Some(e) }
            }
        )* );
		let last_error = last_error.unwrap_or(DispatchError::Other("TransactFailed"));
		log::trace!(target: "runtime::BridgeTransact::transfer_nonfungible", "last_error: {:?}", last_error);
		Err(last_error)
	}

	fn transfer_generic(
		&self,
		sender: [u8; 32],
		data: &Vec<u8>,
		dest: MultiLocation,
		max_weight: Option<XCMWeight>,
	) -> DispatchResult {
		let mut last_error = None;
		for_tuples!( #(
            match Tuple.transfer_generic(sender, data, dest.clone(), max_weight) {
                Ok(()) => return Ok(()),
                Err(e) => { last_error = Some(e) }
            }
        )* );
		let last_error = last_error.unwrap_or(DispatchError::Other("TransactFailed"));
		log::trace!(target: "runtime::BridgeTransact::transfer_generic", "last_error: {:?}", last_error);
		Err(last_error)
	}
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
