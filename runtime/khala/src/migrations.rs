use super::*;
use frame_support::traits::OnRuntimeUpgrade;

// Note to "late-migration":
//
// All the migrations defined in this file are so called "late-migration". We should have done the
// pallet migrations as soon as we perform the runtime upgrade. However the runtime v1090 was done
// without applying the necessary migrations. Without the migrations, the affected apllets can no
// longer access the state db properly.
//
// So here we need to redo the migrations afterward. An immedate problem is that, after the new
// pallets are upgraded, they may have already written some data under the new pallet storage
// prefixes. Most of the pre_upgrade logic checks there's no data under the new pallets as a safe
// guard. However for "late-migrations" this is not the case.
//
// The final decision is to just skip the pre_upgrade checks. We have carefully checked all the
// pre_upgrade checks and confimed that only the prefix checks are skipped. All the other checks
// are still performed in an offline try-runtime test.

const BOUNTIES_OLD_PREFIX: &str = "Treasury";

/// Migrate from 'Treasury' to the new prefix 'Bounties'
pub struct BountiesPrefixMigration;

impl OnRuntimeUpgrade for BountiesPrefixMigration {
	fn on_runtime_upgrade() -> frame_support::weights::Weight {
		use frame_support::traits::PalletInfo;
		let name = <Runtime as frame_system::Config>::PalletInfo::name::<Bounties>()
			.expect("Bounties is part of runtime, so it has a name; qed");
		pallet_bounties::migrations::v4::migrate::<Runtime, Bounties, _>(BOUNTIES_OLD_PREFIX, name)
	}
	#[cfg(feature = "try-runtime")]
	fn pre_upgrade() -> Result<(), &'static str> {
        // Skipped because of "late-migration".
        //
		// use frame_support::traits::PalletInfo;
		// let name = <Runtime as frame_system::Config>::PalletInfo::name::<Bounties>()
		// 	.expect("Bounties is part of runtime, so it has a name; qed");
		// pallet_bounties::migrations::v4::pre_migration::<Runtime, Bounties, _>(
		// 	BOUNTIES_OLD_PREFIX,
		// 	name,
		// );
		Ok(())
	}

	#[cfg(feature = "try-runtime")]
	fn post_upgrade() -> Result<(), &'static str> {
		use frame_support::traits::PalletInfo;
		let name = <Runtime as frame_system::Config>::PalletInfo::name::<Bounties>()
			.expect("Bounties is part of runtime, so it has a name; qed");
		pallet_bounties::migrations::v4::post_migration::<Runtime, Bounties, _>(
			BOUNTIES_OLD_PREFIX,
			name,
		);
		Ok(())
	}
}

const COUNCIL_OLD_PREFIX: &str = "Instance1Collective";
/// Migrate from `Instance1Collective` to the new pallet prefix `Council`
pub struct CouncilStoragePrefixMigration;

impl OnRuntimeUpgrade for CouncilStoragePrefixMigration {
	fn on_runtime_upgrade() -> frame_support::weights::Weight {
		pallet_collective::migrations::v4::migrate::<Runtime, Council, _>(COUNCIL_OLD_PREFIX)
	}

	#[cfg(feature = "try-runtime")]
	fn pre_upgrade() -> Result<(), &'static str> {
        // Skipped because of "late-migration".
        //
		// pallet_collective::migrations::v4::pre_migrate::<Council, _>(COUNCIL_OLD_PREFIX);
		Ok(())
	}

	#[cfg(feature = "try-runtime")]
	fn post_upgrade() -> Result<(), &'static str> {
		pallet_collective::migrations::v4::post_migrate::<Council, _>(COUNCIL_OLD_PREFIX);
		Ok(())
	}
}

const TECHNICAL_COMMITTEE_OLD_PREFIX: &str = "Instance2Collective";
/// Migrate from `Instance2Collective` to the new pallet prefix `TechnicalCommittee`
pub struct TechnicalCommitteeStoragePrefixMigration;

impl OnRuntimeUpgrade for TechnicalCommitteeStoragePrefixMigration {
	fn on_runtime_upgrade() -> frame_support::weights::Weight {
		pallet_collective::migrations::v4::migrate::<Runtime, TechnicalCommittee, _>(
			TECHNICAL_COMMITTEE_OLD_PREFIX,
		)
	}

	#[cfg(feature = "try-runtime")]
	fn pre_upgrade() -> Result<(), &'static str> {
        // Skipped because of "late-migration".
        //
		// pallet_collective::migrations::v4::pre_migrate::<TechnicalCommittee, _>(
		// 	TECHNICAL_COMMITTEE_OLD_PREFIX,
		// );
		Ok(())
	}

	#[cfg(feature = "try-runtime")]
	fn post_upgrade() -> Result<(), &'static str> {
		pallet_collective::migrations::v4::post_migrate::<TechnicalCommittee, _>(
			TECHNICAL_COMMITTEE_OLD_PREFIX,
		);
		Ok(())
	}
}

const TECHNICAL_MEMBERSHIP_OLD_PREFIX: &str = "Instance1Membership";
/// Migrate from `Instance1Membership` to the new pallet prefix `TechnicalMembership`
pub struct TechnicalMembershipStoragePrefixMigration;

impl OnRuntimeUpgrade for TechnicalMembershipStoragePrefixMigration {
	fn on_runtime_upgrade() -> frame_support::weights::Weight {
		use frame_support::traits::PalletInfo;
		let name = <Runtime as frame_system::Config>::PalletInfo::name::<TechnicalMembership>()
			.expect("TechnicalMembership is part of runtime, so it has a name; qed");
		pallet_membership::migrations::v4::migrate::<Runtime, TechnicalMembership, _>(
			TECHNICAL_MEMBERSHIP_OLD_PREFIX,
			name,
		)
	}

	#[cfg(feature = "try-runtime")]
	fn pre_upgrade() -> Result<(), &'static str> {
		use frame_support::traits::PalletInfo;
		let name = <Runtime as frame_system::Config>::PalletInfo::name::<TechnicalMembership>()
			.expect("TechnicalMembership is part of runtime, so it has a name; qed");
		pallet_membership::migrations::v4::pre_migrate::<TechnicalMembership, _>(
			TECHNICAL_MEMBERSHIP_OLD_PREFIX,
			name,
		);
		Ok(())
	}

	#[cfg(feature = "try-runtime")]
	fn post_upgrade() -> Result<(), &'static str> {
		use frame_support::traits::PalletInfo;
		let name = <Runtime as frame_system::Config>::PalletInfo::name::<TechnicalMembership>()
			.expect("TechnicalMembership is part of runtime, so it has a name; qed");
		pallet_membership::migrations::v4::post_migrate::<TechnicalMembership, _>(
			TECHNICAL_MEMBERSHIP_OLD_PREFIX,
			name,
		);
		Ok(())
	}
}

const TIPS_OLD_PREFIX: &str = "Treasury";
/// Migrate pallet-tips from `Treasury` to the new pallet prefix `Tips`
pub struct MigrateTipsPalletPrefix;

impl OnRuntimeUpgrade for MigrateTipsPalletPrefix {
	fn on_runtime_upgrade() -> frame_support::weights::Weight {
		pallet_tips::migrations::v4::migrate::<Runtime, Tips, _>(TIPS_OLD_PREFIX)
	}

	#[cfg(feature = "try-runtime")]
	fn pre_upgrade() -> Result<(), &'static str> {
		pallet_tips::migrations::v4::pre_migrate::<Runtime, Tips, _>(TIPS_OLD_PREFIX);
		Ok(())
	}

	#[cfg(feature = "try-runtime")]
	fn post_upgrade() -> Result<(), &'static str> {
		pallet_tips::migrations::v4::post_migrate::<Runtime, Tips, _>(TIPS_OLD_PREFIX);
		Ok(())
	}
}

/// Migrate from `PalletVersion` to the new `StorageVersion`
pub struct MigratePalletVersionToStorageVersion;

impl OnRuntimeUpgrade for MigratePalletVersionToStorageVersion {
	fn on_runtime_upgrade() -> frame_support::weights::Weight {
		frame_support::migrations::migrate_from_pallet_version_to_storage_version::<
			AllPalletsWithSystem,
		>(&RocksDbWeight::get())
	}
}
