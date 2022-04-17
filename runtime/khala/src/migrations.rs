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

        let r = phala_pallets::stakepool::Pallet::<super::Runtime>::start_mining(
            super::Origin::signed(
                sp_runtime::AccountId32::new(
                    hex_literal::hex!["e8b86f4b9f116d5ee10a2f1aaba5c60c0790ec875de23706f7bec378ea6e840a"]
                )
            ),
            1889,
            phala_types::WorkerPublicKey(
                hex_literal::hex!["b693384f9c836b664226c827b89629ad6e169537290ab5351a878d9097f0376d"]
            ),
            18500_000000000000,
        );
        log::warn!("r = {:?}", r);
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
