pub use self::pallet::*;

#[allow(unused_variables)]
#[allow(clippy::large_enum_variant)]
#[frame_support::pallet]
pub mod pallet {
	use crate::traits::*;
	use assets_registry::SYGMA_PATH_KEY;
	use frame_support::{dispatch::RawOrigin, pallet_prelude::*, transactional};
	use funty::Fundamental;
	use sp_std::vec::Vec;
	use sygma_traits::{DomainID, ExtractDestinationData};
	use xcm::latest::{prelude::*, MultiLocation, Weight as XCMWeight};

	const LOG_TARGET: &str = "runtime::sygma-wrapper";
	const STORAGE_VERSION: StorageVersion = StorageVersion::new(0);

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	#[pallet::storage_version(STORAGE_VERSION)]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: frame_system::Config + sygma_bridge::Config {
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// Assets sent to EVM chain.
		AssetTransfered {
			asset: MultiAsset,
			origin: MultiLocation,
			dest: MultiLocation,
		},
	}

	#[pallet::error]
	pub enum Error<T> {
		/// Can not transfer asset to dest
		CannotDepositAsset,
		/// Unimplemented function
		Unimplemented,
	}

	impl<T> ExtractDestinationData for Pallet<T> {
		fn extract_dest(dest: &MultiLocation) -> Option<(Vec<u8>, DomainID)> {
			match (dest.parents, &dest.interior) {
				(
					0,
					Junctions::X3(
						GeneralKey(sygma_path),
						GeneralIndex(dest_domain_id),
						GeneralKey(recipient),
					),
				) => {
					if sygma_path.clone().into_inner() == SYGMA_PATH_KEY.to_vec() {
						return Some((recipient.to_vec(), dest_domain_id.as_u8()));
					}
					None
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
			match Self::extract_dest(&dest) {
				// The destination path should follow SubBridge protocol
				Some((_, dest_domain)) => {
					// Reject all destination not supported by sygmabridge
					//
					// Sygma bridge itself will verify fee setting and its the (ResourceId, AssetId)
					// list returned by AssetRegistry will represent if asset has been registered and
					// enabled sygmabridge transfer. We don't need verify more here
					sygma_bridge::DestDomainIds::<T>::get(&dest_domain) == true
				}
				_ => false,
			}
			// TODO: NonFungible verification
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
			);
			let origin_location = Junction::AccountId32 {
				network: NetworkId::Any,
				id: sender,
			}
			.into();

			// Check if we can deposit asset into dest.
			ensure!(
				Pallet::<T>::can_deposit_asset(asset.clone(), dest.clone()),
				Error::<T>::CannotDepositAsset
			);

			// Transfer asset through sygma bridge
			<sygma_bridge::pallet::Pallet<T>>::deposit(
				RawOrigin::Signed(sender.into()).into(),
				asset.clone(),
				dest.clone(),
			)?;

			Pallet::<T>::deposit_event(Event::AssetTransfered {
				asset,
				origin: origin_location,
				dest,
			});

			Ok(())
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
