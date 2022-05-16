#[allow(unused_imports)]
use super::*;
#[allow(unused_imports)]
use frame_support::traits::OnRuntimeUpgrade;

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

pub struct SubbridgeV3Migrations;

impl OnRuntimeUpgrade for SubbridgeV3Migrations {
    fn on_runtime_upgrade() -> frame_support::weights::Weight {
        subbridge_pallets::migration::subbridge_v3_migration::migrate::<Runtime>()
    }

    #[cfg(feature = "try-runtime")]
    fn pre_upgrade() -> Result<(), &'static str> {
        subbridge_pallets::migration::subbridge_v3_migration::pre_migrate::<Runtime>()
    }

    #[cfg(feature = "try-runtime")]
    fn post_upgrade() -> Result<(), &'static str> {
        subbridge_pallets::migration::subbridge_v3_migration::post_migrate::<Runtime>()
    }
}

pub struct AssetsRegistryV2Migrations;

impl OnRuntimeUpgrade for AssetsRegistryV2Migrations {
    fn on_runtime_upgrade() -> frame_support::weights::Weight {
        assets_registry::migration::assets_registry_v3_migration_for_rhala::migrate::<Runtime>()
    }

    #[cfg(feature = "try-runtime")]
    fn pre_upgrade() -> Result<(), &'static str> {
        assets_registry::migration::assets_registry_v3_migration_for_rhala::pre_migrate::<Runtime>()
    }

    #[cfg(feature = "try-runtime")]
    fn post_upgrade() -> Result<(), &'static str> {
        assets_registry::migration::assets_registry_v3_migration_for_rhala::post_migrate::<Runtime>()
    }
}
