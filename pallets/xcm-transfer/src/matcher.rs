pub use self::matcher::*;

pub mod matcher {
	use frame_support::traits::Contains;
	use sp_runtime::traits::CheckedConversion;
	use sp_std::{convert::TryFrom, marker::PhantomData, vec::Vec};
	use xcm::v1::{
		AssetId::{Abstract, Concrete},
		Fungibility::Fungible,
		MultiAsset, MultiLocation,
	};
	use xcm_executor::traits::MatchesFungible;

	const LOG_TARGET: &str = "xcm-transfer:matcher";

	pub struct IsSiblingParachainsConcrete<T>(PhantomData<T>);
	impl<T: Contains<MultiLocation>, B: TryFrom<u128>> MatchesFungible<B>
		for IsSiblingParachainsConcrete<T>
	{
		fn matches_fungible(a: &MultiAsset) -> Option<B> {
			log::error!(
				target: LOG_TARGET,
				"IsSiblingParachainsConcrete check fungible {:?}.",
				a.clone(),
			);
			match (&a.id, &a.fun) {
				(Concrete(ref id), Fungible(ref amount)) if T::contains(id) => {
					CheckedConversion::checked_from(*amount)
				}
				_ => None,
			}
		}
	}

	pub struct IsSiblingParachainsAbstract<T>(PhantomData<T>);
	impl<T: Contains<Vec<u8>>, B: TryFrom<u128>> MatchesFungible<B> for IsSiblingParachainsAbstract<T> {
		fn matches_fungible(a: &MultiAsset) -> Option<B> {
			log::error!(
				target: LOG_TARGET,
				"IsSiblingParachainsAbstract check fungible {:?}.",
				a.clone(),
			);
			match (&a.id, &a.fun) {
				(Abstract(ref id), Fungible(ref amount)) if T::contains(&id) => {
					CheckedConversion::checked_from(*amount)
				}
				_ => None,
			}
		}
	}
}
