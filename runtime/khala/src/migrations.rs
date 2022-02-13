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

// https://github.com/paritytech/polkadot/blob/master/runtime/kusama/src/lib.rs#L2913-L2930
// Migration for scheduler pallet to move from a plain Call to a CallOrHash.
pub struct SchedulerMigrationV3;

impl OnRuntimeUpgrade for SchedulerMigrationV3 {
    fn on_runtime_upgrade() -> frame_support::weights::Weight {
        Scheduler::migrate_v2_to_v3()
    }

    #[cfg(feature = "try-runtime")]
    fn pre_upgrade() -> Result<(), &'static str> {
        Scheduler::pre_migrate_to_v3()
    }

    #[cfg(feature = "try-runtime")]
    fn post_upgrade() -> Result<(), &'static str> {
        Scheduler::post_migrate_to_v3()
    }
}
