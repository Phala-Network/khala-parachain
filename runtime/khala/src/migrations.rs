#[allow(unused_imports)]
use super::*;
#[allow(unused_imports)]
use frame_support::traits::OnRuntimeUpgrade;
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
        assets_registry::migration::assets_registry_v3_migration_for_khala::migrate::<Runtime>()
    }

    #[cfg(feature = "try-runtime")]
    fn pre_upgrade() -> Result<(), &'static str> {
        assets_registry::migration::assets_registry_v3_migration_for_khala::pre_migrate::<Runtime>()
    }

    #[cfg(feature = "try-runtime")]
    fn post_upgrade() -> Result<(), &'static str> {
        assets_registry::migration::assets_registry_v3_migration_for_khala::post_migrate::<Runtime>(
        )
    }
}
