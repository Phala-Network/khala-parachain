#[allow(unused_imports)]
use super::*;
#[allow(unused_imports)]
use frame_support::traits::OnRuntimeUpgrade;
pub struct PhalaV4Migration;

impl OnRuntimeUpgrade for PhalaV4Migration {
	fn on_runtime_upgrade() -> frame_support::weights::Weight {
		phala_pallets::migrations::v4::migrate::<Runtime>()
	}

	#[cfg(feature = "try-runtime")]
	fn pre_upgrade() -> Result<(), &'static str> {
		phala_pallets::migrations::v4::pre_migrate::<Runtime>()
	}

	#[cfg(feature = "try-runtime")]
	fn post_upgrade() -> Result<(), &'static str> {
		phala_pallets::migrations::v4::post_migrate::<Runtime>()
	}
}

pub struct PhalaPalletsV5;

impl OnRuntimeUpgrade for PhalaPalletsV5 {
    fn on_runtime_upgrade() -> frame_support::weights::Weight {
        phala_pallets::migrations::v5::migrate::<Runtime>()
    }

    #[cfg(feature = "try-runtime")]
    fn pre_upgrade() -> Result<(), &'static str> {
        phala_pallets::migrations::v5::pre_migrate::<Runtime>()
    }

    #[cfg(feature = "try-runtime")]
    fn post_upgrade() -> Result<(), &'static str> {
        phala_pallets::migrations::v5::post_migrate::<Runtime>()
    }
}
