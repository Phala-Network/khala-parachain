#[allow(unused_imports)]
use super::*;
#[allow(unused_imports)]
use frame_support::traits::OnRuntimeUpgrade;

pub struct ReproTest;

impl OnRuntimeUpgrade for ReproTest {
	/// Execute some pre-checks prior to a runtime upgrade.
	///
	/// This hook is never meant to be executed on-chain but is meant to be used by testing tools.
	#[cfg(feature = "try-runtime")]
	fn pre_upgrade() -> Result<(), &'static str> {

		log::warn!("ReproTest");

        let tokenomic = phala_pallets::mining::Pallet::<super::Runtime>::tokenomic().unwrap();
        let ve = tokenomic.ve(18900_000000000000, 1698, 2);
		log::warn!("ve = {}", ve);
		Ok(())
	}

	/// Execute some post-checks after a runtime upgrade.
	///
	/// This hook is never meant to be executed on-chain but is meant to be used by testing tools.
	#[cfg(feature = "try-runtime")]
	fn post_upgrade() -> Result<(), &'static str> {
		Ok(())
	}

}
