pub use self::pallet::*;

#[allow(unused_variables)]
#[allow(clippy::large_enum_variant)]
#[frame_support::pallet]
pub mod pallet {
	use frame_support::RawOrigin;
	use frame_system::pallet_prelude::*;
	use sygma_traits::{ChainID, DomainID, ExtractRecipient, ResourceId};
	use xcm::latest::{prelude::*, AssetId as XcmAssetId, MultiLocation};

	const LOG_TARGET: &str = "runtime::sygma-wrapper";

	#[pallet::pallet]
	#[pallet::generate_store(pub (super) trait Store)]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: frame_system::Config + sygma_bridge::Config {
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
	}

	#[pallet::error]
	pub enum Error<T> {
		/// Extract dest location failed
		IllegalDestination,
		/// Can not transfer asset to dest
		CannotDepositAsset,
		/// Unimplemented function
		Unimplemented,
	}

	impl ExtractRecipient for Pallet<T> {
		fn extract_recipient(dest: &MultiLocation) -> Option<Vec<u8>> {
			// For example, we force a dest location should be represented by following format.
			match (dest.parents, &dest.interior) {
				(0, Junctions::X2(GeneralKey(recipient), GeneralIndex(_dest_domain_id))) => {
					Some(recipient.to_vec())
				}
				_ => None,
			}
		}
	}

	impl ExtractDestDomainID for Pallet<T> {
		fn extract_dest_domain_id(dest: &MultiLocation) -> Option<DomainID> {
			match (dest.parents, &dest.interior) {
				(0, Junctions::X2(GeneralKey(_recipient), GeneralIndex(dest_domain_id))) => {
					Some(dest_domain_id.as_u8())
				}
				_ => None,
			}
		}
	}

	impl<T: Config> BridgeChecker for Pallet<T>
	where
		<T as frame_system::Config>::AccountId: From<[u8; 32]> + Into<[u8; 32]>,
	{
		fn can_deposit_asset(asset: MultiAsset, dest: MultiLocation) -> bool {
			// TODO: impl
			true
		}

		fn can_send_data(_data: &Vec<u8>, _dest: MultiLocation) -> bool {
			// TODO: impl
			false
		}
	}

	pub struct BridgeTransactImpl<T>(PhantomData<T>);
	impl<T: Config> BridgeTransact for BridgeTransactImpl<T>
	where
		<T as frame_system::Config>::AccountId: From<[u8; 32]> + Into<[u8; 32]>,
	{
		fn new() -> Self {
			Self(PhantomData)
		}

		/// Initiates a transfer of a fungible asset out of the chain. This should be called by another pallet.
		#[transactional]
		fn transfer_fungible(
			&self,
			sender: [u8; 32],
			asset: MultiAsset,
			dest: MultiLocation,
			_max_weight: Option<XCMWeight>,
		) -> DispatchResult {
			log::trace!(
				target: LOG_TARGET,
				"SygmaWrapper fungible transfer, sender: {:?}, asset: {:?}, dest: {:?}.",
				sender,
				&asset,
				&dest,
				max_weight,
			);

			// Check if we can deposit asset into dest.
			ensure!(
				Pallet::<T>::can_deposit_asset(asset.clone(), dest.clone()),
				Error::<T>::CannotDepositAsset
			);

			// Transfer asset through sygma bridge
			<sygma_bridge::pallet::Pallet<T>>::deposit(
				RawOrigin::Signed(sender.into()).into(),
				asset,
				dest,
			)
		}

		/// Initiates a transfer of a nonfungible asset out of the chain. This should be called by another pallet.
		#[transactional]
		fn transfer_nonfungible(
			&self,
			_sender: [u8; 32],
			_asset: MultiAsset,
			_dest: MultiLocation,
			_max_weight: Option<XCMWeight>,
		) -> DispatchResult {
			Err(Error::<T>::Unimplemented.into())
		}

		/// Initiates a transfer of generic data out of the chain. This should be called by another pallet.
		#[transactional]
		fn transfer_generic(
			&self,
			_sender: [u8; 32],
			_data: &Vec<u8>,
			_dest: MultiLocation,
			_max_weight: Option<XCMWeight>,
		) -> DispatchResult {
			Err(Error::<T>::Unimplemented.into())
		}
	}
}
