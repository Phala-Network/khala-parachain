#[allow(unused_imports)]
use super::*;
#[allow(unused_imports)]
use frame_support::traits::OnRuntimeUpgrade;

use sp_core::H256;
use sp_runtime::{traits::AccountIdConversion, AccountId32};

pub struct SdnRegistryTest;

impl OnRuntimeUpgrade for SdnRegistryTest {
	#[cfg(feature = "try-runtime")]
	fn pre_upgrade() -> Result<(), &'static str> {
        // use assets_registry::pallet::AssetProperties;
        // type AssetsRegistry = assets_registry::Pallet<super::Runtime>;
        // type Collective = pallet_collective::Pallet<super::Runtime>;

		log::warn!("SdnRegistryTest");

        // let mut result = AssetsRegistry::force_register_asset(
        //     Origin::root(),
        //     MultiLocation::new(1, X1(Parachain(2007))).into(),
        //     12,
        //     AssetProperties {
        //         name: b"SDN".to_vec(),
        //         symbol: b"SDN".to_vec(),
        //         decimals: 18,
        //     },
        // );

        // log::warn!("Registry asset result: {:?}", result);

        // result = AssetsRegistry::force_set_price(
        //     Origin::root(),
        //     12,
        //     NativeExecutionPrice::get().saturating_mul(10u128.saturating_pow(6)),
        // );

        // log::warn!("Set price result: {:?}", result);

        let hash_slice: [u8; 32] = hex_literal::hex!("bc1432b2191d61cf197d20dc86ac0156620ae1de06758a95e08698e4fa19e3c6");
        let hash: H256 = hash_slice.into();

        let pubkey: [u8; 32] = hex_literal::hex!("4ce421370cf0257d869618ec25c324ed4c6c7f65289297a3c134332c212e350b");
        let marvin: AccountId32 = AccountId32::new(pubkey);

        let close_result = Council::close(
            Origin::signed(marvin),
            hash,
            130u32,
            Weight::from_ref_time(414270000u64),
            48u32,
        );
        log::warn!("Close proposal result: {:?}", close_result);

		Ok(())
	}
	/// Execute some post-checks after a runtime upgrade.
	///
	/// This hook is never meant to be executed on-chain but is meant to be used by testing tools.
	#[cfg(feature = "try-runtime")]
	fn post_upgrade() -> Result<(), &'static str> {
		Ok(())
	}

    fn on_runtime_upgrade() -> frame_support::weights::Weight {
        Weight::zero()
    }
}

// Note to "late-migration":
//
// All the migrations defined in this file are so called "late-migration". We should have done the
// pallet migrations as soon as we perform the runtime upgrade. However the runtime v1090 was done
// without applying the necessary migrations. Without the migrations, affected pallets can no
// longer access the state db properly.
//
// So here we need to redo the migrations afterward. An immediate problem is that, after the new
// pallets are upgraded, they may have already written some data under the new pallet storage
// prefixes. Most of the pre_upgrade logic checks there's no data under the new pallets as a safe
// guard. However for "late-migrations" this is not the case.
//
// The final decision is to just skip the pre_upgrade checks. We have carefully checked all the
// pre_upgrade checks and confirmed that only the prefix checks are skipped. All the other checks
// are still performed in an offline try-runtime test.
