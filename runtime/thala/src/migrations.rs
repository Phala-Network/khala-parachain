#[allow(unused_imports)]
use super::*;
#[allow(unused_imports)]
use frame_support::traits::OnRuntimeUpgrade;
pub struct SubbridgeMigrations;

impl OnRuntimeUpgrade for SubbridgeMigrations {
    fn on_runtime_upgrade() -> frame_support::weights::Weight {
        subbridge_pallets::migration::subbridge_migration::migrate::<Runtime>()
    }

    #[cfg(feature = "try-runtime")]
    fn pre_upgrade() -> Result<(), &'static str> {
        subbridge_pallets::migration::subbridge_migration::pre_migrate::<Runtime>()
    }

    #[cfg(feature = "try-runtime")]
    fn post_upgrade() -> Result<(), &'static str> {
        subbridge_pallets::migration::subbridge_migration::post_migrate::<Runtime>()
    }
}

pub struct AssetsRegistryMigrations;

impl OnRuntimeUpgrade for AssetsRegistryMigrations {
    fn on_runtime_upgrade() -> frame_support::weights::Weight {
        assets_registry::migration::assets_registry_migration::migrate::<Runtime>()
    }

    #[cfg(feature = "try-runtime")]
    fn pre_upgrade() -> Result<(), &'static str> {
        assets_registry::migration::assets_registry_migration::pre_migrate::<Runtime>()
    }

    #[cfg(feature = "try-runtime")]
    fn post_upgrade() -> Result<(), &'static str> {
        assets_registry::migration::assets_registry_migration::post_migrate::<Runtime>()
    }
}

pub struct PhalaPalletV6Migrations;

impl OnRuntimeUpgrade for PhalaPalletV6Migrations {
    fn on_runtime_upgrade() -> frame_support::weights::Weight {
        phala_pallets::migrations::v6::migrate::<Runtime>()
    }

    #[cfg(feature = "try-runtime")]
    fn pre_upgrade() -> Result<(), &'static str> {
        phala_pallets::migrations::v6::pre_migrate::<Runtime>()
    }

    #[cfg(feature = "try-runtime")]
    fn post_upgrade() -> Result<(), &'static str> {
        phala_pallets::migrations::v6::post_migrate::<Runtime>()
    }
}
